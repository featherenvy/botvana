//! Market module

pub mod event;
pub mod orderbook;
pub mod trade;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use soa_derive::StructOfArray;

use crate::exchange::ExchangeId;

/// Single market information
#[derive(Clone, Debug, StructOfArray)]
#[soa_derive(Clone, Debug, Deserialize, Serialize)]
pub struct Market {
    pub exchange: ExchangeId,
    pub name: String,
    pub native_symbol: String,
    pub size_increment: f64,
    pub price_increment: f64,
    pub r#type: MarketType,
}

impl From<MarketRef<'_>> for Market {
    fn from(market_ref: MarketRef) -> Self {
        Self {
            exchange: market_ref.exchange.clone(),
            name: market_ref.name.clone(),
            native_symbol: market_ref.native_symbol.clone(),
            size_increment: *market_ref.size_increment,
            price_increment: *market_ref.price_increment,
            r#type: market_ref.r#type.clone(),
        }
    }
}

/// Market types enum
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum MarketType {
    Spot(SpotMarket),
    Futures,
}

/// Spot market information
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpotMarket {
    pub base: String,
    pub quote: String,
}

/// Futures market
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FuturesMarket {
    pub expires_at: Option<DateTime<Utc>>,
}

impl From<Box<[Market]>> for MarketVec {
    fn from(markets: Box<[Market]>) -> Self {
        let mut vec = Self::with_capacity(markets.len());
        for market in markets.iter() {
            vec.push(market.clone());
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
        let _ = MarketVec::with_capacity(1024);
    }
}
