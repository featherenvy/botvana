// Core market data modules
pub mod adapter;
pub mod engine;
pub mod error;

// Exchange adapters
pub mod binance;
pub mod ftx;

pub use engine::*;

mod prelude {
    pub use std::borrow::Borrow;
    pub use std::cell::RefCell;
    pub use std::collections::HashMap;
    pub use std::time::Duration;

    pub use metered::{clear::Clear, time_source::StdInstant, *};
    pub use serde_json::json;
    pub use surf::Url;

    pub use crate::market_data::{adapter::*, error::*};
}

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

impl MarketEvent {
    /// Creates new `MarketEvent` with current timestamp
    fn new(r#type: MarketEventType) -> Self {
        Self {
            r#type,
            timestamp: Utc::now(),
        }
    }

    /// Creates new `MarketEvent::Trades` variant
    fn trades(market: Box<str>, trades: Box<[Trade]>) -> Self {
        Self::new(MarketEventType::Trades(market, trades))
    }

    /// Creates new `MarketEvent::MidPriceChange` variant
    fn mid_price_change(market: Box<str>, bid: f64, ask: f64) -> Self {
        Self::new(MarketEventType::MidPriceChange(market, bid, ask))
    }

    /// Creates new `MarketEvent::OrderbookUpdate` variant
    fn orderbook_update(market: Box<str>, orderbook: Box<PlainOrderbook<f64>>) -> Self {
        Self::new(MarketEventType::OrderbookUpdate(market, orderbook))
    }
}
