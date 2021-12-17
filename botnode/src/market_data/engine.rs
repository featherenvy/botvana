//! Market Data Engine
use std::borrow::Borrow;
use std::convert::TryFrom;
use std::num::NonZeroUsize;
use std::time::Duration;

use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
// use glommio::io::{ImmutableFile, ImmutableFileBuilder, StreamWriterBuilder};
use serde_json::json;
use surf::Url;

use crate::market_data::{ftx, Market, MarketEvent};
use crate::prelude::*;

/// Engine that maintains connections to exchanges and produces raw market data
pub struct MarketDataEngine {
    data_tx: ring_channel::RingSender<MarketEvent>,
    data_rx: ring_channel::RingReceiver<MarketEvent>,
}

impl MarketDataEngine {
    pub fn new() -> Self {
        let (data_tx, data_rx) = ring_channel::ring_channel(NonZeroUsize::new(1024).unwrap());
        Self { data_tx, data_rx }
    }
}

#[async_trait(?Send)]
impl Engine for MarketDataEngine {
    type Data = MarketEvent;

    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting market data engine");

        match fetch_market_info().await {
            Ok(markets) => {
                self.data_tx.send(MarketEvent::Markets(markets)).unwrap();
            }
            Err(e) => {
                error!("Failed to fetch market info: {:?}", e);
            }
        };

        if let Err(e) = run_market_data_loop(self.data_tx, shutdown).await {
            error!("MDE Error: {}", e);
        }

        Ok(())
    }

    /// Returns cloned market event receiver
    fn data_rx(&self) -> ring_channel::RingReceiver<Self::Data> {
        self.data_rx.clone()
    }
}

pub async fn fetch_market_info() -> anyhow::Result<Box<[Market]>> {
    let client: surf::Client = surf::Config::new()
        .set_base_url(Url::parse("https://ftx.com")?)
        .set_timeout(Some(Duration::from_secs(5)))
        .try_into()?;

    let mut res = client.get("/api/markets").await.unwrap();
    let body = res.body_string().await.unwrap();

    // let mut file = ImmutableFileBuilder::new(&format!("http-vcr"))
    //     .build_sink()
    //     .await?;
    // file.write_all(body.as_bytes()).await.unwrap();
    // file.seal().await;

    let root = serde_json::from_slice::<ftx::rest::ResponseRoot>(body.as_bytes())?;
    let result: &ftx::rest::ResponseResult = root.result.borrow();

    let crate::market_data::ftx::rest::ResponseResult::Markets(markets) = result;
    println!("{:?}", markets);

    Ok(markets
        .iter()
        .filter_map(|m| Market::try_from(m.borrow()).ok())
        .collect())
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

    let subscribe_msg = json!({"op": "subscribe", "channel": "trades", "market": "BTC-PERP"});
    ws_stream
        .send(Message::text(subscribe_msg.to_string()))
        .await?;

    loop {
        futures::select! {
            msg = ws_stream.next().fuse() => {
                match msg {
                    Some(Ok(Message::Text(msg))) => {
                        let start = std::time::Instant::now();
                        let ws_msg = serde_json::from_slice::<ftx::ws::WsMsg>(msg.as_bytes());
                        if let Ok(ws_msg) = ws_msg {
                            let data = ws_msg.data.borrow();
                            let ftx::ws::Data::Trades(trades) = data;

                            info!("got trades = {:?}", trades);

                            let trades: Vec<_> = trades
                                .into_iter()
                                .filter_map(|trade| crate::market_data::trade::Trade::try_from(trade).ok())
                                .collect();

                            info!("parsing took = {:?}", start.elapsed());

                            market_data_tx
                                .send(MarketEvent::Trades(trades.into_boxed_slice()))?;

                        } else {
                            warn!("error parsing: {}", msg);
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

impl ToString for MarketDataEngine {
    fn to_string(&self) -> String {
        "market-data-engine".to_string()
    }
}
