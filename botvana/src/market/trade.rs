//! Trade
use chrono::{DateTime, Utc};

#[derive(Clone, Debug)]
pub struct Trade {
    pub price: f64,
    pub size: f64,
    /// Time of the trade specified by the exchange
    pub time: DateTime<Utc>,
    /// Time the trade was received
    pub received_at: std::time::Instant,
}

impl Trade {
    /// Creates new trade
    pub fn new(price: f64, size: f64, time: DateTime<Utc>) -> Self {
        Self {
            price,
            size,
            time,
            received_at: std::time::Instant::now(),
        }
    }
}

/// Columnar struct of trades
pub struct TradesVec {
    pub prices: Vec<f64>,
    pub sizes: Vec<f64>,
    pub times: Vec<DateTime<Utc>>,
    pub received_times: Vec<std::time::Instant>,
}

impl TradesVec {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            prices: Vec::with_capacity(capacity),
            sizes: Vec::with_capacity(capacity),
            times: Vec::with_capacity(capacity),
            received_times: Vec::with_capacity(capacity),
        }
    }
}
