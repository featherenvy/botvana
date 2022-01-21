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
    pub use std::time::{Duration, SystemTime};

    pub use metered::{clear::Clear, time_source::StdInstant, *};
    pub use serde_json::json;
    pub use surf::Url;

    pub use crate::market_data::{adapter::*, error::*};
}

use botvana::market::{orderbook::PlainOrderbook, trade::Trade, Market, MarketVec};

/// Market event enum produced by market data engine
#[derive(Clone, Debug)]
pub struct MarketEvent {
    pub r#type: MarketEventType,
    pub timestamp: std::time::SystemTime,
}

#[derive(Clone, Debug)]
pub enum MarketEventType {
    /// Markets update
    Markets(Box<MarketVec>),
    /// Trades happened
    Trades(Box<str>, Box<[Trade]>),
    /// Orderbook updated
    OrderbookUpdate(Box<str>, Box<PlainOrderbook<f64>>),
    /// Mid-price changed
    MidPriceChange(Box<str>, f64, f64),
}

impl MarketEvent {
    /// Creates new `MarketEvent` with current timestamp
    fn new(r#type: MarketEventType) -> Self {
        Self {
            r#type,
            timestamp: std::time::SystemTime::now(),
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

    /// Creates new `MarketEvent::Markets` variant
    fn markets(market_vec: Box<MarketVec>) -> Self {
        Self::new(MarketEventType::Markets(market_vec))
    }
}
