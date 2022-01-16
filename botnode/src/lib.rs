pub mod audit;
pub mod control;
pub mod engine;
pub mod error;
pub mod exchange;
pub mod indicator;
pub mod market_data;
pub mod trading;
pub mod util;

/// Useful prelude for implementing botnode engines
pub mod prelude {
    pub use std::collections::HashMap;
    pub use std::time::Duration;

    pub use arrayvec::ArrayVec;
    pub use async_codec::Framed;
    pub use async_shutdown::Shutdown;
    pub use async_trait::async_trait;
    pub use chrono::{DateTime, Utc};
    pub use futures::prelude::*;
    pub use glommio::{
        channels::shared_channel::{self, *},
        channels::spsc_queue,
        net::TcpStream,
        prelude::*,
        LocalExecutor, LocalExecutorBuilder,
    };
    pub use tracing::{debug, error, info, trace, warn};

    pub use botvana::{
        cfg::{BotConfiguration, IndicatorConfig},
        market::orderbook::{PlainOrderbook, PriceLevelsVec, UpdateOrderbook},
        net::{
            codec::BotvanaCodec,
            msg::{BotId, Message},
        },
    };

    pub use crate::engine::*;
    pub use crate::error::{EngineError, StartEngineError};
    pub use crate::indicator::IndicatorEvent;
    pub use crate::market_data::{MarketEvent, MarketEventType};

    pub type DynBoxError = Box<dyn std::error::Error>;
}
