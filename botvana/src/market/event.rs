use super::{orderbook::*, trade::*, MarketVec};

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
    pub fn new(r#type: MarketEventType) -> Self {
        Self {
            r#type,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Creates new `MarketEvent::Trades` variant
    pub fn trades(market: Box<str>, trades: Box<[Trade]>) -> Self {
        Self::new(MarketEventType::Trades(market, trades))
    }

    /// Creates new `MarketEvent::MidPriceChange` variant
    pub fn mid_price_change(market: Box<str>, bid: f64, ask: f64) -> Self {
        Self::new(MarketEventType::MidPriceChange(market, bid, ask))
    }

    /// Creates new `MarketEvent::OrderbookUpdate` variant
    pub fn orderbook_update(market: Box<str>, orderbook: Box<PlainOrderbook<f64>>) -> Self {
        Self::new(MarketEventType::OrderbookUpdate(market, orderbook))
    }

    /// Creates new `MarketEvent::Markets` variant
    pub fn markets(market_vec: Box<MarketVec>) -> Self {
        Self::new(MarketEventType::Markets(market_vec))
    }
}
