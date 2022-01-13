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
    const NAME: &'static str = "binance-rest";

    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        let client: surf::Client = surf::Config::new()
            .set_base_url(Url::parse(&self.api_url).map_err(MarketDataError::with_source)?)
            .set_timeout(Some(Duration::from_secs(10)))
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
                        let orderbook = markets.get_mut(update.symbol);
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
                    ws::WsMsg::Response(response) => Ok(None),
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

    #[derive(Deserialize, Debug)]
    pub struct WsResponse {
        result: serde_json::Value,
        id: i32,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    pub enum WsMsg<'a> {
        #[serde(borrow)]
        OrderbookTicker(WsBookTicker<'a>),
        #[serde(borrow)]
        Trade(WsTrade<'a>),
        #[serde(borrow)]
        DepthUpdate(WsDepthUpdate<'a>),
        Response(WsResponse),
    }

    #[derive(Debug, Deserialize)]
    pub struct WsTrade<'a> {
        #[serde(rename = "e")]
        pub event: &'a str,
        #[serde(rename = "E")]
        pub event_time: u64,
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
        pub buyer_order_id: u64,
        #[serde(rename = "a")]
        pub seller_order_id: u64,
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
    use std::convert::TryFrom;

    use serde::{Deserialize, Deserializer};

    use botvana::market::orderbook::*;

    #[derive(Debug, Deserialize)]
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

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ExchangeInfo<'a> {
        server_time: f64,
        #[serde(borrow)]
        pub symbols: Box<[SymbolInfo<'a>]>,
    }

    impl<'a> TryFrom<&SymbolInfo<'a>> for botvana::market::Market {
        type Error = Box<dyn std::error::Error>;

        fn try_from(symbol_info: &SymbolInfo<'a>) -> Result<Self, Self::Error> {
            let size_increment = 1.0 / 10_i32.pow(symbol_info.base_asset_precision as u32) as f64;
            let price_increment = 1.0 / 10_i32.pow(symbol_info.quote_asset_precision as u32) as f64;

            Ok(Self {
                name: symbol_info.symbol.to_string(),
                native_symbol: symbol_info.symbol.to_string(),
                size_increment,
                price_increment,
                r#type: botvana::market::MarketType::Spot(botvana::market::SpotMarket {
                    base: symbol_info.base_asset.to_string(),
                    quote: symbol_info.quote_asset.to_string(),
                }),
            })
        }
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct SymbolInfo<'a> {
        pub symbol: &'a str,
        pub status: &'a str,
        pub base_asset: &'a str,
        pub base_asset_precision: u8,
        pub quote_asset: &'a str,
        pub quote_asset_precision: u8,
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_orderbook_snapshot_parse() {
            let json: &str = include_str!("../../tests/binance_api_v3_depth.json");

            let _: OrderbookSnapshot = serde_json::from_str(&json).unwrap();
        }

        #[test]
        fn test_try_from_symbol_info_for_markets() {
            let symbol_info = SymbolInfo {
                symbol: "BTCUSDT",
                status: "TRADING",
                base_asset: "BTC",
                base_asset_precision: 8,
                quote_asset: "USDT",
                quote_asset_precision: 2,
            };

            let market = botvana::market::Market::try_from(&symbol_info).unwrap();

            assert_eq!(market.price_increment, 0.01);
            assert_eq!(market.size_increment, 0.00000001);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_orderbook_snapshot() {
        let b = Binance::default();

        //smol::block_on(b.fetch_orderbook_snapshot("ETHBTC")).unwrap();
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
