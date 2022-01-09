//! Binance market data adapter
use super::prelude::*;
use crate::market_data::Market;
use crate::prelude::*;

#[derive(Debug)]
pub struct Binance {
    pub metrics: BinanceMetrics,
    cur_idx: u64,
    api_url: Box<str>,
    markets: Box<[Box<str>]>,
}

impl Binance {
    fn new(markets: &[&str]) -> Self {
        Self {
            markets: markets
                .iter()
                .map(|s| Box::from(s.clone()))
                .collect::<Box<[_]>>(),
            ..Default::default()
        }
    }
}

impl Default for Binance {
    fn default() -> Self {
        Binance {
            api_url: Box::from("https://api.binance.com"),
            cur_idx: 0,
            metrics: BinanceMetrics::default(),
            markets: Box::new([]),
        }
    }
}

#[async_trait(?Send)]
impl RestMarketDataAdapter for Binance {
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

    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        Ok(Box::new([]))
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
                let market = market.to_lowercase().replace("-", "");
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
        let start = std::time::Instant::now();

        debug!("got ws_msg = {:?}", msg);

        let book_ticker = serde_json::from_slice::<ws::WsBookTicker>(msg.as_bytes());
        match book_ticker {
            Ok(book_ticker) => Ok(Some(MarketEvent {
                r#type: MarketEventType::MidPriceChange(
                    Box::from(book_ticker.symbol),
                    book_ticker.bid_price,
                    book_ticker.ask_price,
                ),
                timestamp: Utc::now(),
            })),
            Err(e) => {
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
                                let trade =
                                    botvana::market::trade::Trade::new(trade.price, trade.size, dt);

                                Ok(Some(MarketEvent {
                                    r#type: MarketEventType::Trades(
                                        Box::from(symbol),
                                        Box::new([trade]),
                                    ),
                                    timestamp: Utc::now(),
                                }))
                            }
                            ws::WsMsg::DepthUpdate(update) => {
                                let orderbook = markets.get_mut(update.symbol);
                                if let Some(orderbook) = orderbook {
                                    orderbook.update_with_timestamp(
                                        &update.bids,
                                        &update.asks,
                                        update.event_time,
                                    );
                                    Ok(Some(MarketEvent {
                                        r#type: MarketEventType::OrderbookUpdate(
                                            Box::from(update.symbol),
                                            Box::new(orderbook.clone()),
                                        ),
                                        timestamp: Utc::now(),
                                    }))
                                } else {
                                    Ok(None)
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Default, Debug)]
pub struct BinanceMetrics {
    throughput: Throughput<StdInstant, RefCell<metered::common::TxPerSec>>,
}

mod ws {
    use serde::{Deserialize, Deserializer};
    use serde_aux::prelude::*;

    use botvana::market::orderbook::*;

    #[derive(Debug, Deserialize)]
    #[serde(tag = "e")]
    pub enum WsMsg<'a> {
        #[serde(borrow)]
        #[serde(rename = "trade")]
        Trade(WsTrade<'a>),
        #[serde(borrow)]
        #[serde(rename = "depthUpdate")]
        DepthUpdate(WsDepthUpdate<'a>),
    }

    #[derive(Debug, Deserialize)]
    pub struct WsTrade<'a> {
        #[serde(rename = "E")]
        pub event_time: f64,
        #[serde(rename = "s")]
        pub symbol: &'a str,
        #[serde(rename = "t")]
        pub trade_id: u64,
        #[serde(rename = "p")]
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pub price: f64,
        #[serde(rename = "q")]
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pub size: f64,
        #[serde(rename = "b")]
        pub buyer_order_id: u32,
        #[serde(rename = "a")]
        pub seller_order_id: u32,
        #[serde(rename = "T")]
        pub trade_time: f64,
        #[serde(rename = "m")]
        pub maket_maker_buyer: bool,
    }

    #[derive(Debug, Deserialize)]
    pub struct WsDepthUpdate<'a> {
        #[serde(rename = "E")]
        pub event_time: f64,
        #[serde(rename = "s")]
        pub symbol: &'a str,
        #[serde(rename = "U")]
        pub first_update_id: u64,
        #[serde(rename = "u")]
        pub final_update_id: u64,
        #[serde(rename = "b")]
        #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
        pub bids: PriceLevelsVec<f64>,
        #[serde(rename = "a")]
        #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
        pub asks: PriceLevelsVec<f64>,
    }

    #[derive(Debug, Deserialize)]
    pub struct WsBookTicker<'a> {
        #[serde(rename = "u")]
        pub update_id: u64,
        #[serde(rename = "s")]
        pub symbol: &'a str,
        #[serde(rename = "b")]
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pub bid_price: f64,
        #[serde(rename = "B")]
        pub bid_size: &'a str,
        #[serde(rename = "a")]
        #[serde(deserialize_with = "deserialize_number_from_string")]
        pub ask_price: f64,
        #[serde(rename = "A")]
        pub ask_size: &'a str,
    }

    fn deserialize_into_price_levels_vec<'de, D>(
        deserializer: D,
    ) -> Result<PriceLevelsVec<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = Box::<[(String, String)]>::deserialize(deserializer)?;

        let mut levels: Vec<(f64, f64)> = buf
            .iter()
            .map(|(price, size)| (price.parse::<f64>().unwrap(), size.parse::<f64>().unwrap()))
            .collect();

        Ok(PriceLevelsVec::from_tuples_vec_unsorted(&mut levels))
    }
}

mod rest {
    use botvana::market::orderbook::*;
    use serde::{Deserialize, Deserializer};

    #[derive(Deserialize)]
    pub struct OrderbookSnapshot {
        #[serde(rename = "lastUpdateId")]
        pub last_update_id: u64,
        #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
        pub bids: PriceLevelsVec<f64>,
        #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
        pub asks: PriceLevelsVec<f64>,
    }

    fn deserialize_into_price_levels_vec<'de, D>(
        deserializer: D,
    ) -> Result<PriceLevelsVec<f64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let buf = Box::<[(String, String)]>::deserialize(deserializer)?;

        let mut levels: Vec<(f64, f64)> = buf
            .iter()
            .map(|(price, size)| (price.parse::<f64>().unwrap(), size.parse::<f64>().unwrap()))
            .collect();

        Ok(PriceLevelsVec::from_tuples_vec_unsorted(&mut levels))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_snapshot_parse() {
        let json: &str = include_str!("../../tests/binance_api_v3_depth.json");

        let _: rest::OrderbookSnapshot = serde_json::from_str(&json).unwrap();
    }

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
            "e": "trade",
            "E": 123456789,
            "s": "BNBBTC",
            "t": 12345,
            "p": "0.001",
            "q": "100",
            "b": 88,
            "a": 50,
            "T": 123456785,
            "m": true,
            "M": true
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

        let event = b.process_ws_msg(depth_msg, &mut HashMap::new()).unwrap();

        //assert!(event.is_some());
    }
}
