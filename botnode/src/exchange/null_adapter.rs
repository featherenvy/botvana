use super::adapter::ExchangeAdapter;

pub(crate) struct NullAdapter;

impl ExchangeAdapter for NullAdapter {
    const NAME: &'static str = "null-adapter";
}
