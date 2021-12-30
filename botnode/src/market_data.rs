pub mod adapter;
pub mod engine;
pub mod error;
pub mod exchange;
pub mod ftx;

pub use engine::*;

use chrono::{DateTime, Utc};

use botvana::market::{orderbook::PlainOrderbook, trade::Trade, Market};

/// Market event enum produced by market data engine
#[derive(Clone, Debug)]
pub struct MarketEvent {
    pub r#type: MarketEventType,
    timestamp: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub enum MarketEventType {
    Markets(Box<[Market]>),
    Trades(Box<str>, Box<[Trade]>),
    OrderbookUpdate(Box<str>, Box<PlainOrderbook<f64>>),
    MidPriceChange(Box<str>, f64, f64),
}
