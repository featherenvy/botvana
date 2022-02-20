// Core market data modules
pub mod adapter;
pub mod engine;
pub mod error;

// Exchange adapters
pub mod binance;
pub mod ftx;
pub mod serum;

pub use engine::*;

mod prelude {
    pub use std::{
        borrow::Borrow,
        cell::RefCell,
        collections::HashMap,
        time::{Duration, SystemTime},
    };

    pub use metered::{clear::Clear, time_source::StdInstant, *};
    pub use serde_json::json;
    pub use surf::Url;

    pub use crate::market_data::{adapter::*, error::*};
}
