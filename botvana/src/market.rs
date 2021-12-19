//! Market module

use chrono::{DateTime, Utc};

pub mod orderbook;
pub mod trade;

#[derive(Clone, Debug)]
pub struct Market {
    pub name: String,
    pub native_symbol: String,
    pub size_increment: f64,
    pub price_increment: f64,
    pub r#type: MarketType,
}

#[derive(Clone, Debug)]
pub enum MarketType {
    Spot(SpotMarket),
    Futures,
}

#[derive(Clone, Debug)]
pub struct SpotMarket {
    pub base: String,
    pub quote: String,
}

#[derive(Clone, Debug)]
pub struct FuturesMarket {
    pub expires_at: Option<DateTime<Utc>>,
}
pub struct MarketRef {}

pub struct MarketsVec {
    pub symbol: Vec<String>,
    pub price_increment: Vec<f64>,
    pub size_increment: Vec<f64>,
    pub status: Vec<MarketStatus>,
}

impl MarketsVec {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            symbol: Vec::with_capacity(capacity),
            price_increment: Vec::with_capacity(capacity),
            size_increment: Vec::with_capacity(capacity),
            status: Vec::with_capacity(capacity),
        }
    }
}

pub enum MarketStatus {
    /// Open market that can be traded
    Open,
    /// Post-only - only limit orders can be posted
    PostOnly,
    /// Market is disabled and can't be traded
    Disabled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_market_vec() {
        let _ = MarketsVec::with_capacity(1024);
    }
}
