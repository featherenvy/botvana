#[derive(Debug, thiserror::Error)]
#[error("Market data error: {source}")]
pub struct MarketDataError {
    pub source: Box<dyn std::error::Error>,
}

impl MarketDataError {
    pub fn with_source(err: impl std::error::Error + 'static) -> Self {
        Self {
            source: Box::new(err),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unknown variant: {variant}")]
pub struct UnknownVariantError {
    pub variant: String,
}
