//! Exchange Adapter
//!
//! This module defines market data adapter traits that when implemented allow
//! the market data engine to operate on any exchange.

use super::error::ExchangeError;
use crate::{
    exchange::order_request::OrderRequest, exchange::order_response::OrderResponse, prelude::*,
};

/// Market data adapter trait
#[async_trait(?Send)]
pub trait ExchangeAdapter {
    const NAME: &'static str;
}

pub(super) trait HttpExchangeOrderAdapter {
    fn place_order(order: OrderRequest) -> Result<OrderResponse, ExchangeError>;

    fn cancel_order(order: OrderRequest) -> Result<OrderResponse, ExchangeError>;
}

pub(super) trait HttpExchangeCancelAll {
    fn cancel_all(market: String) -> Result<(), ExchangeError>;
}
