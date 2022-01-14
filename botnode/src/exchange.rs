pub(crate) mod adapter;
pub(crate) mod engine;
pub(crate) mod error;
pub(crate) mod null_adapter;
pub(crate) mod order_request;
pub(crate) mod order_response;

#[derive(Clone)]
pub enum ExchangeEvent {
    BalanceChange,
}
