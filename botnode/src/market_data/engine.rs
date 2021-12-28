//! Market Data Engine
use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::Duration;

use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
// use glommio::io::{ImmutableFile, ImmutableFileBuilder, StreamWriterBuilder};
use serde_json::json;

use crate::market_data::{adapter::*, error::*, ftx, MarketEvent};
use crate::prelude::*;

const CONSUMER_LIMIT: usize = 16;

/// Engine that maintains connections to exchanges and produces raw market data
pub struct MarketDataEngine<A: MarketDataAdapter> {
    adapter: A,
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    data_txs: ArrayVec<spsc_queue::Producer<MarketEvent>, CONSUMER_LIMIT>,
}

impl<A: MarketDataAdapter> MarketDataEngine<A> {
    pub fn new(config_rx: spsc_queue::Consumer<BotConfiguration>, adapter: A) -> Self {
        Self {
            adapter,
            config_rx,
            data_txs: ArrayVec::<_, CONSUMER_LIMIT>::new(),
        }
    }

    pub async fn run_market_data_loop(
        &mut self,
        markets: Box<[String]>,
        shutdown: Shutdown,
    ) -> anyhow::Result<()> {
        loop {
            if let Err(e) = self.run_exchange_connection_loop(&markets, &shutdown).await {
                error!("Error running exchange connection loop: {}", e);
            }

            let wait = Duration::from_secs(5);
            warn!("disconnected from the exchange; waiting for {:?}", wait);
            sleep(wait).await;
        }
    }

/// Runs the websocket loop until Shutdown signal
async fn run_exchange_connection_loop(
    &mut self,
    markets: &Box<[String]>,
    shutdown: &Shutdown,
) -> anyhow::Result<Option<MarketEvent>> {
    let _token = shutdown.delay_shutdown_token()?;
    let url = "wss://ftx.com/ws";
    info!("connecting to {}", url);
    let (mut ws_stream, _) = connect_async(url).await?;

    for market in markets.iter() {
        info!("Subscribing for {}", market);

        let subscribe_msg = json!({"op": "subscribe", "channel": "orderbook", "market": market});
        ws_stream
            .send(Message::text(subscribe_msg.to_string()))
            .await?;

        let subscribe_msg = json!({"op": "subscribe", "channel": "trades", "market": market});
        ws_stream
            .send(Message::text(subscribe_msg.to_string()))
            .await?;
    }

    let mut markets: HashMap<String, PlainOrderbook<_>> = markets
        .into_iter()
        .map(|m| (m.clone(), PlainOrderbook::with_capacity(100)))
        .collect();

    loop {
        futures::select! {
            msg = ws_stream.next().fuse() => {
                match msg {
                    Some(Ok(Message::Text(msg))) => {
                        match process_ws_msg(&msg, &mut markets) {
                            Ok(event) => self.push_value(event),
                            Err(e) => warn!("Failed to process websocket message: {}", e)
                        }
                    }
                    Some(Ok(other)) => {
                        warn!(
                            reason = "unexpected-websocket-message",
                            msg = &*other.to_string()
                            );
                    }
                    None | Some(Err(_)) => {
                        error!(reason = "disconnected");
                        break Ok(None)
                    }
                }
            }
            _ = shutdown.wait_shutdown_triggered().fuse() => {
                info!("Market data engine shutting down");
                break Ok(None);
            }
        }
    }
}

}

#[async_trait(?Send)]
impl<A: MarketDataAdapter> Engine for MarketDataEngine<A> {
    type Data = MarketEvent;

    /// Start the market data engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting market data engine");

        match self.adapter.fetch_markets().await {
            Ok(markets) => {
                let event = MarketEvent {
                    r#type: MarketEventType::Markets(markets),
                    timestamp: Utc::now(),
                };
                self.push_value(event);
            }
            Err(e) => {
                error!("Failed to fetch market info: {:?}", e);
            }
        };

        debug!("Waiting for configuration");
        let config = await_configuration(self.config_rx.clone());
        debug!("Got config = {:?}", config);
        let markets = config.market_data.into_boxed_slice();

        info!("Running loop w/ markets = {:?}", markets);
        if let Err(e) = self.run_market_data_loop(markets, shutdown).await {
            error!("Error running loop: {}", e);
        }

        Ok(())
    }

    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        &self.data_txs
    }

    /// Returns cloned market event receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (data_tx, data_rx) = spsc_queue::make(1);
        self.data_txs.push(data_tx);
        data_rx
    }
}

/// Processes Websocket text message
pub fn process_ws_msg(
    msg: &str,
    markets: &mut HashMap<String, PlainOrderbook<f64>>,
) -> Result<MarketEvent, MarketDataError> {
    let start = std::time::Instant::now();
    let ws_msg = serde_json::from_slice::<ftx::ws::WsMsg>(msg.as_bytes());

    match ws_msg {
        Err(e) => {
            error!("ws_msg {}", msg);

            Err(MarketDataError {
                source: Box::new(e),
            })
        }
        Ok(ws_msg) => {
            let data = ws_msg.data.borrow();
            match data {
                ftx::ws::Data::Trades(trades) => {
                    info!("got trades = {:?}", trades);

                    let trades: Vec<_> = trades
                        .iter()
                        .filter_map(|trade| botvana::market::trade::Trade::try_from(trade).ok())
                        .collect();

                    info!("parsing took = {:?}", start.elapsed());

                    Ok(MarketEvent {
                        r#type: MarketEventType::Trades(trades.into_boxed_slice()),
                        timestamp: Utc::now(),
                    })
                }
                ftx::ws::Data::Orderbook(orderbook_msg) => {
                    let orderbook = match orderbook_msg.action {
                        "partial" => {
                            let orderbook = PlainOrderbook {
                                bids: PriceLevelsVec::from_tuples_vec(orderbook_msg.bids.to_vec()),
                                asks: PriceLevelsVec::from_tuples_vec(orderbook_msg.asks.to_vec()),
                            };
                            info!("orderbook = {:?}", orderbook);
                            markets.insert(ws_msg.market.unwrap().to_string(), orderbook.clone());
                            orderbook
                        }
                        "update" => {
                            let orderbook = markets.get_mut(ws_msg.market.unwrap()).unwrap();
                            orderbook.update(
                                &PriceLevelsVec::from_tuples_vec(orderbook_msg.bids.to_vec()),
                                &PriceLevelsVec::from_tuples_vec(orderbook_msg.asks.to_vec()),
                            );
                            orderbook.clone()
                        }
                        action => {
                            return Err(MarketDataError::with_source(UnknownVariantError {
                                variant: action.to_string(),
                            }))
                        }
                    };

                    Ok(MarketEvent {
                        r#type: MarketEventType::OrderbookUpdate(Box::new([orderbook])),
                        timestamp: Utc::now(),
                    })
                    //info!("got orderbook = {:?}", orderbook);
                }
            }
        }
    }
}

impl<A: MarketDataAdapter> ToString for MarketDataEngine<A> {
    fn to_string(&self) -> String {
        "market-data-engine".to_string()
    }
}
