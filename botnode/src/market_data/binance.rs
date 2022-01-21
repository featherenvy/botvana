//! Binance market data adapter

pub(crate) mod rest;
pub(crate) mod ws;

use super::prelude::*;
use crate::{market_data::Market, prelude::*};
use botvana::exchange::ExchangeRef;

#[derive(Debug)]
pub struct Binance {
    pub metrics: BinanceMetrics,
    cur_idx: u64,
    api_url: Box<str>,
    symbol_map: Vec<(String, String)>,
}

impl Default for Binance {
    fn default() -> Self {
        Binance {
            api_url: Box::from("https://api.binance.com"),
            cur_idx: 0,
            metrics: BinanceMetrics::default(),
            symbol_map: vec![],
        }
    }
}

#[async_trait(?Send)]
impl RestMarketDataAdapter for Binance {
    const NAME: &'static str = "binance-rest";
    const EXCHANGE_REF: ExchangeRef = ExchangeRef::BinanceSpot;

    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        let client: surf::Client = surf::Config::new()
            .set_base_url(Url::parse(&self.api_url).map_err(MarketDataError::with_source)?)
            .set_timeout(Some(Duration::from_secs(20)))
            .try_into()
            .map_err(MarketDataError::with_source)?;

        let mut res = client.get(format!("/api/v3/exchangeInfo")).await.unwrap();
        let body = res.body_string().await.unwrap();

        let info = serde_json::from_slice::<rest::ExchangeInfo>(body.as_bytes())
            .map_err(MarketDataError::with_source)?;

        debug!("{} markets on Binance", info.symbols.len());

        Ok(info
            .symbols
            .iter()
            .filter_map(|sym| Market::try_from(sym).ok())
            .collect())
    }

    /// Fetches orderbook snapshot
    async fn fetch_orderbook_snapshot(
        &self,
        symbol: &str,
    ) -> Result<PlainOrderbook<f64>, MarketDataError> {
        let client: surf::Client = surf::Config::new()
            .set_base_url(Url::parse(&self.api_url).map_err(MarketDataError::with_source)?)
            .set_timeout(Some(Duration::from_secs(5)))
            .try_into()
            .map_err(MarketDataError::with_source)?;

        let mut res = client
            .get(format!("/api/v3/depth?symbol={}", symbol))
            .await
            .unwrap();
        let body = res.body_string().await.unwrap();

        let snapshot = serde_json::from_slice::<rest::OrderbookSnapshot>(body.as_bytes())
            .map_err(MarketDataError::with_source)?;

        let mut orderbook = PlainOrderbook::<f64>::with_capacity(1000);
        orderbook.update(&snapshot.bids, &snapshot.asks);

        Ok(orderbook)
    }
}

impl WsMarketDataAdapter for Binance {
    fn throughput_metrics(&self) -> &Throughput<StdInstant, RefCell<metered::common::TxPerSec>> {
        &self.metrics.throughput
    }

    fn ws_url(&self) -> Box<str> {
        Box::from("wss://stream.binance.com:9443/ws")
    }

    fn subscribe_msgs(&mut self, markets: &[&str]) -> Box<[String]> {
        self.cur_idx += 1;

        let params: Vec<_> = markets
            .iter()
            .map(|market| {
                let market = market.to_lowercase().replace("-", "").replace("/", "");

                [
                    format!("{}@depth@100ms", market),
                    format!("{}@trade", market),
                    format!("{}@bookTicker", market),
                ]
            })
            .flatten()
            .collect();

        Box::new([json!({"method": "SUBSCRIBE", "params": params, "id": self.cur_idx}).to_string()])
    }

    fn process_ws_msg(
        &self,
        msg: &str,
        markets: &mut HashMap<Box<str>, PlainOrderbook<f64>>,
    ) -> Result<Option<MarketEvent>, MarketDataError> {
        trace!("got ws_msg = {:?}", msg);

        let ws_msg = serde_json::from_slice::<ws::WsMsg>(msg.as_bytes());

        match ws_msg {
            Err(e) => {
                error!("Error parsing ws_msg: {}", msg);

                Err(MarketDataError {
                    source: Box::new(e),
                })
            }
            Ok(ws_msg) => {
                let data = ws_msg;
                match data {
                    ws::WsMsg::Trade(trade) => {
                        use chrono::TimeZone;
                        let dt = Utc.timestamp(trade.trade_time as i64, 0);
                        let symbol = trade.symbol;
                        let trade = botvana::market::trade::Trade::new(trade.price, trade.size, dt);

                        Ok(Some(MarketEvent::trades(
                            Box::from(symbol),
                            Box::new([trade]),
                        )))
                    }
                    ws::WsMsg::DepthUpdate(update) => {
                        // Convert symbol coming in from the WS API to "internal"
                        let symbol = markets
                            .keys()
                            .find(|k| k.replace("/", "") == update.symbol)
                            .cloned();
                        if let Some(symbol) = symbol {
                            let orderbook = markets.get_mut(&symbol);
                            if let Some(orderbook) = orderbook {
                                orderbook.update_with_timestamp(
                                    &update.bids,
                                    &update.asks,
                                    update.event_time,
                                );
                                Ok(Some(MarketEvent::orderbook_update(
                                    Box::from(update.symbol),
                                    Box::new(orderbook.clone()),
                                )))
                            } else {
                                warn!("No orderbook snapshot found for {}", update.symbol);
                                Ok(None)
                            }
                        } else {
                            warn!("No symbol mapping found for {}", update.symbol);
                            Ok(None)
                        }
                    }
                    ws::WsMsg::OrderbookTicker(book_ticker) => {
                        return Ok(Some(MarketEvent::mid_price_change(
                            Box::from(book_ticker.symbol),
                            book_ticker.bid_price,
                            book_ticker.ask_price,
                        )))
                    }
                    ws::WsMsg::Response(_response) => Ok(None),
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct BinanceMetrics {
    throughput: Throughput<StdInstant, RefCell<metered::common::TxPerSec>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_orderbook_snapshot() {
        let b = Binance::default();

        smol::block_on(b.fetch_orderbook_snapshot("ETHBTC")).unwrap();
    }

    #[test]
    fn test_process_ws_msg_err() {
        let b = Binance::default();

        assert!(b.process_ws_msg("", &mut HashMap::new()).is_err());
    }

    #[test]
    fn test_process_ws_msg_trade() {
        let trade_msg = r#"{
            "e":"trade",
            "E":1642011077609,
            "s":"BTCUSDT",
            "t":1219924203,
            "p":"43806.41000000",
            "q":"0.09158000",
            "b":8964867731,
            "a":8964867628,
            "T":1642011077609,
            "m":false,
            "M":true
        }"#;
        let b = Binance::default();

        let event = b.process_ws_msg(trade_msg, &mut HashMap::new()).unwrap();

        assert!(event.is_some());
    }

    #[test]
    fn test_process_ws_msg_book_ticker() {
        let book_ticker_msg = r#"{
            "u":13639707622,
            "s":"ETHUSDT",
            "b":"3826.81000000",
            "B":"0.07210000",
            "a":"3826.82000000",
            "A":"0.88940000"
        }"#;
        let b = Binance::default();

        let event = b
            .process_ws_msg(book_ticker_msg, &mut HashMap::new())
            .unwrap();

        assert!(event.is_some());
    }

    #[test]
    fn test_process_ws_msg_depth_update() {
        let depth_msg = r#"{
            "e": "depthUpdate",
            "E": 123456789,
            "s": "BNBBTC",
            "U": 157,
            "u": 160,
            "b": [
              [
                "0.0024",
                "10"
              ]
            ],
            "a": [
              [
                "0.0026",
                "100"
              ]
            ]
        }"#;
        let b = Binance::default();

        b.process_ws_msg(depth_msg, &mut HashMap::new()).unwrap();

        //assert!(event.is_some());
    }
}
