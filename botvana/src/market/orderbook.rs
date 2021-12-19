//! Orderbook
use serde::Deserialize;

pub struct PlainOrderbook<T> {
    _marker: std::marker::PhantomData<T>,
}
