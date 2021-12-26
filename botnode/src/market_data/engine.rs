//! Market Data Engine
use std::borrow::Borrow;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::time::Duration;

use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
// use glommio::io::{ImmutableFile, ImmutableFileBuilder, StreamWriterBuilder};
use serde_json::json;

use crate::market_data::{adapter::*, error::MarketDataError, ftx, MarketEvent};
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

    /// Runs the websocket loop until Shutdown signal
    pub async fn run_market_data_loop(
        &mut self,
        markets: Box<[String]>,
        shutdown: Shutdown,
    ) -> anyhow::Result<()> {
        let _token = shutdown.delay_shutdown_token()?;
        let url = "wss://ftx.com/ws";
        info!("connecting to {}", url);
        let (mut ws_stream, _) = connect_async(url).await?;

        for market in markets.iter() {
            info!("Subscribing for {}", market);

            let subscribe_msg =
                json!({"op": "subscribe", "channel": "orderbook", "market": market});
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
                            error!(reason = "disconnected", reconnect_in = "5s");
                            sleep(Duration::from_secs(5)).await;
                            break Ok(());
                        }
                    }
                }
                _ = shutdown.wait_shutdown_triggered().fuse() => {
                    info!("Market data engine shutting down");
                    break Ok(());
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
                let val = MarketEvent::Markets(markets);
                self.push_value(val);
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

                    Ok(MarketEvent::Trades(trades.into_boxed_slice()))
                }
                ftx::ws::Data::Orderbook(orderbook_msg) => {
                    match orderbook_msg.action {
                        "partial" => {
                            let orderbook = PlainOrderbook {
                                bids: PriceLevelsVec::from_tuples_vec(orderbook_msg.bids.to_vec()),
                                asks: PriceLevelsVec::from_tuples_vec(orderbook_msg.asks.to_vec()),
                            };
                            info!("orderbook = {:?}", orderbook);
                            markets.insert(ws_msg.market.unwrap().to_string(), orderbook);
                        }
                        "update" => {
                            let orderbook = markets.get_mut(ws_msg.market.unwrap()).unwrap();
                            orderbook.update(
                                &PriceLevelsVec::from_tuples_vec(orderbook_msg.bids.to_vec()),
                                &PriceLevelsVec::from_tuples_vec(orderbook_msg.asks.to_vec()),
                            );
                        }
                        _ => {}
                    }

                    Ok(MarketEvent::OrderbookUpdate)
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
