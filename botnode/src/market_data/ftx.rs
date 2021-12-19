//! FTX adapter implementation
use std::borrow::Borrow;
use std::time::Duration;

use surf::Url;

use crate::market_data::adapter::*;
use crate::market_data::{error::MarketDataError, Market};
use crate::prelude::*;

pub struct Ftx;

#[async_trait(?Send)]
impl MarketDataAdapter for Ftx {
    fn trade_stream(&self) -> TradeStream {
        TradeStream {}
    }

    fn orderbook_stream(&self) -> OrderbookStream {
        OrderbookStream {}
    }

    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        let client: surf::Client = surf::Config::new()
            .set_base_url(Url::parse("https://ftx.com").map_err(MarketDataError::with_source)?)
            .set_timeout(Some(Duration::from_secs(5)))
            .try_into()
            .map_err(MarketDataError::with_source)?;

        let mut res = client.get("/api/markets").await.unwrap();
        let body = res.body_string().await.unwrap();

        // let mut file = ImmutableFileBuilder::new(&format!("http-vcr"))
        //     .build_sink()
        //     .await?;
        // file.write_all(body.as_bytes()).await.unwrap();
        // file.seal().await;

        let root = serde_json::from_slice::<rest::ResponseRoot>(body.as_bytes())
            .map_err(MarketDataError::with_source)?;
        let result: &rest::ResponseResult = root.result.borrow();

        let rest::ResponseResult::Markets(markets) = result;
        println!("{:?}", markets);

        Ok(markets
            .iter()
            .filter_map(|m| Market::try_from(m.borrow()).ok())
            .collect())
    }
}

pub mod rest {
    use std::borrow::Cow;

    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseRoot<'a> {
        pub success: bool,
        #[serde(borrow)]
        pub result: Cow<'a, ResponseResult<'a>>,
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(untagged)]
    pub enum ResponseResult<'a> {
        #[serde(borrow)]
        Markets(Vec<Cow<'a, MarketInfo<'a>>>),
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MarketInfo<'a> {
        #[serde(borrow)]
        pub name: &'a str,
        #[serde(borrow)]
        pub base_currency: Option<&'a str>,
        #[serde(borrow)]
        pub quote_currency: Option<&'a str>,
        // pub quote_volume24h: Option<f64>,
        // pub change1h: Option<f64>,
        // pub change24h: Option<f64>,
        // pub change_bod: Option<f64>,
        // pub high_leverage_fee_exempt: bool,
        pub min_provide_size: f64,
        pub r#type: &'a str,
        #[serde(borrow)]
        pub underlying: Option<&'a str>,
        pub enabled: bool,
        // pub ask: Option<f64>,
        // pub bid: Option<f64>,
        // pub last: Option<f64>,
        pub post_only: Option<bool>,
        // pub price: Option<f64>,
        pub price_increment: f64,
        pub size_increment: f64,
        pub restricted: Option<bool>,
        //pub volume_usd24h: Option<f64>,
    }

    impl<'a> TryFrom<&'a MarketInfo<'a>> for botvana::market::Market {
        type Error = String;

        fn try_from(market: &'a MarketInfo<'a>) -> Result<Self, Self::Error> {
            let r#type = match market.r#type {
                "spot" => botvana::market::MarketType::Spot(botvana::market::SpotMarket {
                    base: market
                        .base_currency
                        .ok_or_else(|| "Missing base currency".to_string())?
                        .to_string(),
                    quote: market
                        .quote_currency
                        .ok_or_else(|| "Missing quote currency".to_string())?
                        .to_string(),
                }),
                "future" => botvana::market::MarketType::Futures,
                _ => return Err(format!("Invalid market type: {}", market.r#type)),
            };

            Ok(Self {
                name: market.name.to_string(),
                native_symbol: market.name.to_string(),
                size_increment: market.size_increment,
                price_increment: market.price_increment,
                r#type,
            })
        }
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub enum MarketType {
        Spot,
        Future,
    }
}

pub mod ws {
    use std::borrow::Cow;

    use serde::Deserialize;

    /// FTX Websocket message
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct WsMsg<'a> {
        #[serde(borrow)]
        pub channel: &'a str,
        pub market: Option<&'a str>,
        #[serde(borrow)]
        pub data: Cow<'a, Data<'a>>,
        pub r#type: Option<String>,
    }

    /// FTX Data of websocket message
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(untagged)]
    pub enum Data<'a> {
        #[serde(borrow)]
        Trades(Box<[Trade<'a>]>),
        #[serde(borrow)]
        Orderbook(OrderbookMsg<'a>),
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct OrderbookMsg<'a> {
        //pub checksum: i32,
        pub time: f64,
        pub bids: Box<[(f64, f64)]>,
        pub asks: Box<[(f64, f64)]>,
        pub action: &'a str,
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    pub struct Trade<'a> {
        pub id: i64,
        pub price: f64,
        pub side: &'a str,
        pub size: f64,
        pub liquidation: bool,
        pub time: &'a str,
    }

    impl<'a> TryFrom<&Trade<'a>> for botvana::market::trade::Trade {
        type Error = String;

        fn try_from(trade: &Trade<'a>) -> Result<Self, Self::Error> {
            Ok(Self {
                price: trade.price,
                size: trade.size,
                received_at: std::time::Instant::now(),
                time: trade
                    .time
                    .parse()
                    .map_err(|_| format!("error parsing: {}", trade.time))?,
            })
        }
    }
}
