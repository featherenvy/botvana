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

    pub fn surf_error(e: surf::Error) -> Self {
        Self::with_source(SurfError { error: e })
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Unexpected variant: {variant}")]
pub struct UnknownVariantError {
    pub variant: String,
}

#[derive(Debug, thiserror::Error)]
#[error("Surf error: {error}")]
pub struct SurfError {
    error: surf::Error,
}
