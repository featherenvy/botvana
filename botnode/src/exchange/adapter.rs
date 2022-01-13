//! Exchange Adapter
//!
//! This module defines market data adapter traits that when implemented allow
//! the market data engine to operate on any exchange.

use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};

use crate::{prelude::*};

/// Market data adapter trait
#[async_trait(?Send)]
pub trait ExchangeAdapter {
	const NAME: &'static str;
}

pub(crate) struct NullAdapter;

impl ExchangeAdapter for NullAdapter {
    const NAME: &'static str = "null-adapter";
}

