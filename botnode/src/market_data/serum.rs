//! Serum DEX adapter implementation

mod rest;
mod ws;

use std::cell::RefCell;

use metered::{common::TxPerSec, time_source::StdInstant, *};
use serde_json::json;
use surf::Url;

use crate::{
    market_data::{adapter::*, error::*},
    prelude::*,
};

#[derive(Default, Debug)]
pub struct SerumMetrics {
    throughput: Throughput<StdInstant, RefCell<TxPerSec>>,
}

/// Serum adapter
#[derive(Debug)]
pub struct Serum {
    pub metrics: SerumMetrics,
    pub rest_url: &'static str,
    pub ws_url: &'static str,
}

impl Default for Serum {
    fn default() -> Self {
        Self {
            rest_url: "http://localhost:8000",
            ws_url: "ws://localhost:8000/v1/ws",
            metrics: Default::default(),
        }
    }
}

#[async_trait(?Send)]
impl RestMarketDataAdapter for Serum {
    const NAME: &'static str = "serum";
    const EXCHANGE_REF: ExchangeId = ExchangeId::Serum;

    /// Fetches available markets on Serum
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        let client: surf::Client = surf::Config::new()
            .set_base_url(Url::parse(self.rest_url).map_err(MarketDataError::with_source)?)
            .set_timeout(Some(Duration::from_secs(5)))
            .try_into()
            .map_err(MarketDataError::with_source)?;

        let mut res = client
            .get("/api/markets")
            .await
            .map_err(MarketDataError::surf_error)?;
        let body = res
            .body_string()
            .await
            .map_err(MarketDataError::surf_error)?;

        let markets = serde_json::from_slice::<Box<[rest::Market]>>(body.as_bytes())
            .map_err(MarketDataError::with_source)?;

        Ok(markets
            .iter()
            .filter_map(|m| <Market as TryFrom<&'_ rest::Market<'_>>>::try_from(&m).ok())
            .collect())
    }

    async fn fetch_orderbook_snapshot(
        &self,
        _symbol: &str,
    ) -> Result<PlainOrderbook<f64>, MarketDataError> {
        Ok(PlainOrderbook::<f64>::new())
    }
}

impl WsMarketDataAdapter for Serum {
    fn throughput_metrics(&self) -> &Throughput<StdInstant, RefCell<metered::common::TxPerSec>> {
        &self.metrics.throughput
    }

    fn ws_url(&self) -> Box<str> {
        Box::from(self.ws_url)
    }

    fn subscribe_msgs(&mut self, markets: &[&str]) -> Box<[String]> {
        info!("Subscribing for {markets:?}");

        Box::new([
            json!({"op": "subscribe", "channel": "level2", "markets": markets}).to_string(),
            json!({"op": "subscribe", "channel": "trades", "markets": markets}).to_string(),
        ])
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
    ws_msg: ws::WsMsg,
    markets: &mut HashMap<Box<str>, PlainOrderbook<f64>>,
) -> Result<Option<MarketEvent>, MarketDataError> {
    info!("ws_msg = {ws_msg:?}");

    match ws_msg {
        ws::WsMsg::Subscribed { markets, channel } => {
            info!("Subscribed {channel}: {markets:?}");

            Ok(None)
        }
        ws::WsMsg::RecentTrades {
            market,
            timestamp: _,
            trades,
        } => {
            info!("{market} recent trades: {trades:?}");

            Ok(None)
        }
        ws::WsMsg::L2snapshot(mut snapshot) => {
            let time = chrono::DateTime::parse_from_rfc3339(snapshot.timestamp)
                .map_err(MarketDataError::with_source)?
                .timestamp_millis();
            let orderbook = PlainOrderbook {
                bids: PriceLevelsVec::from_tuples_vec_unsorted(&mut snapshot.bids),
                asks: PriceLevelsVec::from_tuples_vec_unsorted(&mut snapshot.asks),
                time: time as f64,
            };
            markets.insert(Box::from(snapshot.market), orderbook.clone());

            Ok(Some(MarketEvent::orderbook_update(
                Box::from(snapshot.market),
                Box::new(orderbook),
            )))
        }
        ws::WsMsg::L2update(update) => {
            let time = chrono::DateTime::parse_from_rfc3339(update.timestamp)
                .map_err(MarketDataError::with_source)?
                .timestamp_millis();
            let orderbook = markets.get_mut(update.market).unwrap();
            orderbook.update_with_timestamp(
                &PriceLevelsVec::from_tuples_vec(&update.bids),
                &PriceLevelsVec::from_tuples_vec(&update.asks),
                time as f64,
            );

            Ok(Some(MarketEvent::orderbook_update(
                Box::from(update.market),
                Box::new(orderbook.clone()),
            )))
        }
        _ => Ok(None),
    }
}
