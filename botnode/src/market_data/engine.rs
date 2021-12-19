//! Market Data Engine
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::num::NonZeroUsize;
use std::time::Duration;

use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
// use glommio::io::{ImmutableFile, ImmutableFileBuilder, StreamWriterBuilder};
use serde_json::json;

use crate::market_data::{adapter::*, error::MarketDataError, ftx, MarketEvent};
use crate::prelude::*;

/// Engine that maintains connections to exchanges and produces raw market data
pub struct MarketDataEngine<A: MarketDataAdapter> {
    adapter: A,
    data_tx: ring_channel::RingSender<MarketEvent>,
    data_rx: ring_channel::RingReceiver<MarketEvent>,
}

impl<A: MarketDataAdapter> MarketDataEngine<A> {
    pub fn new(adapter: A) -> Self {
        let (data_tx, data_rx) = ring_channel::ring_channel(NonZeroUsize::new(1024).unwrap());
        Self {
            data_tx,
            data_rx,
            adapter,
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
                self.data_tx.send(MarketEvent::Markets(markets)).unwrap();
            }
            Err(e) => {
                error!("Failed to fetch market info: {:?}", e);
            }
        };

        if let Err(e) = run_market_data_loop(self.data_tx, shutdown).await {
            error!("Market engine error: {}", e);
        }

        Ok(())
    }

    /// Returns cloned market event receiver
    fn data_rx(&self) -> ring_channel::RingReceiver<Self::Data> {
        self.data_rx.clone()
    }
}

/// Runs the websocket loop until Shutdown signal
pub async fn run_market_data_loop(
    mut market_data_tx: ring_channel::RingSender<MarketEvent>,
    shutdown: Shutdown,
) -> anyhow::Result<()> {
    let _token = shutdown.delay_shutdown_token()?;
    let url = "wss://ftx.com/ws";
    info!("connecting to {}", url);
    let (mut ws_stream, _) = connect_async(url).await?;

    let subscribe_msg = json!({"op": "subscribe", "channel": "orderbook", "market": "BTC-PERP"});
    ws_stream
        .send(Message::text(subscribe_msg.to_string()))
        .await?;

    let subscribe_msg = json!({"op": "subscribe", "channel": "trades", "market": "BTC-PERP"});
    ws_stream
        .send(Message::text(subscribe_msg.to_string()))
        .await?;

    loop {
        futures::select! {
            msg = ws_stream.next().fuse() => {
                match msg {
                    Some(Ok(Message::Text(msg))) => {
                        if let Err(e) = process_ws_msg(&msg, &mut market_data_tx) {
                            warn!("Failed to process websocket message: {}", e);
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

/// Processes Websocket text message
pub fn process_ws_msg(
    msg: &str,
    market_data_tx: &mut ring_channel::RingSender<MarketEvent>,
) -> Result<(), MarketDataError> {
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

                    market_data_tx
                        .send(MarketEvent::Trades(trades.into_boxed_slice()))
                        .map_err(|e| MarketDataError {
                            source: Box::new(e),
                        })?;
                }
                ftx::ws::Data::Orderbook(orderbook) => {
                    info!("got orderbook = {:?}", orderbook);
                }
            }

            Ok(())
        }
    }
}

impl<A: MarketDataAdapter> ToString for MarketDataEngine<A> {
    fn to_string(&self) -> String {
        "market-data-engine".to_string()
    }
}
