//! Serum DEX adapter implementation

mod ws;

use std::cell::RefCell;

use metered::{common::TxPerSec, time_source::StdInstant, *};
use serde_json::json;

use crate::{
    market_data::{adapter::*, error::*},
    prelude::*,
};

#[derive(Default, Debug)]
pub struct SerumMetrics {
    throughput: Throughput<StdInstant, RefCell<TxPerSec>>,
}

/// Serum adapter
#[derive(Default, Debug)]
pub struct Serum {
    pub metrics: SerumMetrics,
}

impl WsMarketDataAdapter for Serum {
    fn throughput_metrics(&self) -> &Throughput<StdInstant, RefCell<metered::common::TxPerSec>> {
        &self.metrics.throughput
    }

    fn ws_url(&self) -> Box<str> {
        Box::from("wss://ftx.com/ws")
    }

    fn subscribe_msgs(&mut self, markets: &[&str]) -> Box<[String]> {
        markets
            .iter()
            .map(|market| {
                info!("Subscribing for {market}");

                [
                    json!({"op": "subscribe", "channel": "orderbook", "market": market})
                        .to_string(),
                    json!({"op": "subscribe", "channel": "trades", "market": market}).to_string(),
                ]
            })
            .flatten()
            .collect()
    }

    /// Processes Websocket text message
    fn process_ws_msg(
        &self,
        msg: &str,
        markets: &mut HashMap<Box<str>, PlainOrderbook<f64>>,
    ) -> Result<Option<MarketEvent>, MarketDataError> {
        let ws_msg = serde_json::from_slice::<ws::WsMsg>(msg.as_bytes());

        match ws_msg {
            Ok(ws_msg) => Ok(process_market_ws_message(ws_msg, markets)?),
            Err(e) => {
                error!("Failed to parse {msg}");

                Err(MarketDataError::with_source(e))
            }
        }
    }
}

#[inline]
fn process_market_ws_message(
    _ws_msg: ws::WsMsg,
    _markets: &mut HashMap<Box<str>, PlainOrderbook<f64>>,
) -> Result<Option<MarketEvent>, MarketDataError> {
    Ok(None)
}
