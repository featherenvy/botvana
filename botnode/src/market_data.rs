pub mod adapter;
pub mod engine;
pub mod error;
pub mod exchange;
pub mod ftx;

pub use engine::*;

use botvana::market::{trade::Trade, Market};

/// Market event enum produced by market data engine
#[derive(Clone, Debug)]
pub enum MarketEvent {
    Markets(Box<[Market]>),
    Trades(Box<[Trade]>),
    MidPriceChange(f64, f64),
}
