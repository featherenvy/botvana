use std::borrow::Cow;

use serde::Deserialize;

use botvana::exchange::ExchangeId;

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
            exchange: ExchangeId::Ftx,
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
