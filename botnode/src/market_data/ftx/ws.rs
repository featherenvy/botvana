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
}

/// Data in the websocket message
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum Data<'a> {
    #[serde(borrow)]
    Trades(Box<[Trade<'a>]>),
    #[serde(borrow)]
    Orderbook(OrderbookMsg<'a>),
    None(()),
}

/// Orderbook message - partial snapshot or update
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrderbookMsg<'a> {
    //pub checksum: i32,
    pub time: f64,
    pub bids: Box<[(f64, f64)]>,
    pub asks: Box<[(f64, f64)]>,
    pub action: &'a str,
}

/// Single trade information
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
