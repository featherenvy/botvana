use async_std::task::sleep;
use std::time::Duration;

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

    /// Runs the adapter event loop
    async fn run_loop(
        &mut self,
        data_txs: crate::market_data::MarketDataProducers,
        markets: Box<[Box<str>]>,
        shutdown: Shutdown,
    ) -> Result<(), MarketDataError> {
        loop {
            if let Err(e) = self
                .run_exchange_connection_loop(&data_txs, &markets, &shutdown)
                .await
            {
                error!("Error running exchange connection loop: {}", e);
            }

            let wait = Duration::from_secs(5);
            warn!("disconnected from the exchange; waiting for {:?}", wait);
            sleep(wait).await;
        }
    }

    /// Runs the exchange connection event loop
    async fn run_exchange_connection_loop(
        &mut self,
        data_txs: &crate::market_data::MarketDataProducers,
        markets: &Box<[Box<str>]>,
        shutdown: &Shutdown,
    ) -> Result<Option<MarketEvent>, MarketDataError>;
}

pub struct TradeStream;

pub struct OrderbookStream;
