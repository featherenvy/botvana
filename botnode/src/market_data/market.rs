use chrono::{DateTime, Utc};

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
