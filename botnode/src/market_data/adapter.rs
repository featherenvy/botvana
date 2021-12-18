use crate::market_data::{error::MarketDataError, Market};
use crate::prelude::*;

/// Market data adapter trait
#[async_trait(?Send)]
pub trait MarketDataAdapter {
    /// Returns live trade stream
    fn trade_stream(&self) -> TradeStream;

    /// Returns live orderbook stream
    fn orderbook_stream(&self) -> OrderbookStream;

    /// Fetches and returns markets information
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError>;
}

pub struct TradeStream;

pub struct OrderbookStream;
