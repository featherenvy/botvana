//! Exchange Adapter
//!
//! This module defines market data adapter traits that when implemented allow
//! the market data engine to operate on any exchange.

use super::error::ExchangeError;
use crate::exchange::order_request::OrderRequest;
use crate::exchange::order_response::OrderResponse;
use crate::prelude::*;

/// Market data adapter trait
#[async_trait(?Send)]
pub trait ExchangeAdapter {
    const NAME: &'static str;
}

pub trait HttpExchangeAdapter<T> {
    fn place_order(order: OrderRequest) -> Result<OrderResponse, ExchangeError>;

    fn cancel_order(order: OrderRequest) -> Result<OrderResponse, ExchangeError>;
}
