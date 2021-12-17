//! FTX specific code
pub mod rest {
    use std::borrow::Cow;

    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseRoot<'s> {
        pub success: bool,
        #[serde(borrow)]
        pub result: Cow<'s, ResponseResult<'s>>,
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(untagged)]
    pub enum ResponseResult<'s> {
        #[serde(borrow)]
        Markets(Vec<Cow<'s, MarketInfo<'s>>>),
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct MarketInfo<'s> {
        #[serde(borrow)]
        pub name: &'s str,
        #[serde(borrow)]
        pub base_currency: Option<&'s str>,
        #[serde(borrow)]
        pub quote_currency: Option<&'s str>,
        // pub quote_volume24h: Option<f64>,
        // pub change1h: Option<f64>,
        // pub change24h: Option<f64>,
        // pub change_bod: Option<f64>,
        // pub high_leverage_fee_exempt: bool,
        pub min_provide_size: f64,
        pub r#type: &'s str,
        #[serde(borrow)]
        pub underlying: Option<&'s str>,
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

    impl<'s> TryFrom<&'s MarketInfo<'s>> for crate::market_data::market::Market {
        type Error = String;

        fn try_from(market: &'s MarketInfo<'s>) -> Result<Self, Self::Error> {
            let r#type = match market.r#type {
                "spot" => crate::market_data::market::MarketType::Spot(
                    crate::market_data::market::SpotMarket {
                        base: market
                            .base_currency
                            .ok_or_else(|| "Missing base currency".to_string())?
                            .to_string(),
                        quote: market
                            .quote_currency
                            .ok_or_else(|| "Missing quote currency".to_string())?
                            .to_string(),
                    },
                ),
                "future" => crate::market_data::market::MarketType::Futures,
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

    use crate::market_data::trade::TradesVec;

    /// FTX Websocket message
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    pub struct WsMsg<'s> {
        #[serde(borrow)]
        pub channel: &'s str,
        pub market: Option<&'s str>,
        #[serde(borrow)]
        pub data: Cow<'s, Data<'s>>,
        pub r#type: Option<String>,
    }

    /// FTX Data of websocket message
    #[derive(Debug, Clone, PartialEq, Deserialize)]
    #[serde(untagged)]
    pub enum Data<'s> {
        #[serde(borrow)]
        Trades(Box<[Trade<'s>]>),
    }

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    pub struct Trade<'s> {
        pub id: i64,
        pub price: f64,
        pub side: &'s str,
        pub size: f64,
        pub liquidation: bool,
        pub time: &'s str,
    }

    impl<'s> TryFrom<&Trade<'s>> for crate::market_data::trade::Trade {
        type Error = String;

        fn try_from(trade: &Trade<'s>) -> Result<Self, Self::Error> {
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

    impl<'s> TryFrom<Vec<Trade<'s>>> for crate::market_data::trade::TradesVec {
        type Error = String;

        fn try_from(trades: Vec<Trade<'s>>) -> Result<Self, Self::Error> {
            let vec = TradesVec::with_capacity(trades.len());
            Ok(vec)
        }
    }
}
