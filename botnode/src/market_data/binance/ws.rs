use serde::{Deserialize, Deserializer};
use serde_aux::prelude::*;

use botvana::market::orderbook::*;

#[derive(Deserialize, Debug)]
pub struct WsResponse {
    _result: serde_json::Value,
    _id: i32,
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
