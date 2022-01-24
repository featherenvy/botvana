#[derive(Debug, thiserror::Error)]
#[error("Exchange error: {source}")]
pub struct ExchangeError {
    pub source: Box<dyn std::error::Error>,
}

impl ExchangeError {
    pub(crate) fn with_source(err: impl std::error::Error + 'static) -> Self {
        Self {
            source: Box::new(err),
        }
    }
}
