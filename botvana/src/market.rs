//! Market module

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::exchange::ExchangeRef;

pub mod orderbook;
pub mod trade;

/// Single market information
#[derive(Clone, Debug)]
pub struct Market {
    pub exchange: ExchangeRef,
    pub name: String,
    pub native_symbol: String,
    pub size_increment: f64,
    pub price_increment: f64,
    pub r#type: MarketType,
}

/// Market types enum
#[derive(Clone, Debug)]
pub enum MarketType {
    Spot(SpotMarket),
    Futures,
}

/// Spot market information
#[derive(Clone, Debug)]
pub struct SpotMarket {
    pub base: String,
    pub quote: String,
}

/// Futures market
#[derive(Clone, Debug)]
pub struct FuturesMarket {
    pub expires_at: Option<DateTime<Utc>>,
}

/// List of markets stored in columnar format
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct MarketsVec {
    pub exchange: Vec<ExchangeRef>,
    pub symbol: Vec<String>,
    pub price_increment: Vec<f64>,
    pub size_increment: Vec<f64>,
    pub status: Vec<MarketStatus>,
}

impl MarketsVec {
    /// Creates new, empty markets list
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            exchange: Vec::with_capacity(capacity),
            symbol: Vec::with_capacity(capacity),
            price_increment: Vec::with_capacity(capacity),
            size_increment: Vec::with_capacity(capacity),
            status: Vec::with_capacity(capacity),
        }
    }

    /// Returns number of markets
    pub fn len(&self) -> usize {
        self.symbol.len()
    }
}

impl From<Box<[Market]>> for MarketsVec {
    fn from(markets: Box<[Market]>) -> Self {
        let mut vec = Self::with_capacity(markets.len());
        for market in markets.iter() {
            vec.exchange.push(market.exchange);
            vec.symbol.push(market.name.clone());
            vec.price_increment.push(market.price_increment);
            vec.size_increment.push(market.size_increment);
            vec.status.push(MarketStatus::Open);
        }
        vec
    }
}

/// Market status enum
#[derive(Clone, Debug, Deserialize, Serialize)]
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
