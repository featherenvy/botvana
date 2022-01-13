use crate::prelude::*;

/// Error encountered while running the engine
#[derive(Debug, thiserror::Error)]
#[error("Error running the engine: {source}")]
pub struct EngineError {
    source: Box<dyn std::error::Error>,
}

impl EngineError {
    pub fn with_source<T: std::error::Error + 'static>(source: T) -> Self {
        Self {
            source: Box::new(source),
        }
    }
}

/// Error encountered starting the engine
#[derive(Debug, thiserror::Error)]
#[error("Error starting the engine: {source}")]
pub struct StartEngineError {
    pub source: Box<dyn std::error::Error>,
}

impl<T: 'static> From<glommio::GlommioError<T>> for StartEngineError {
    fn from(err: GlommioError<T>) -> Self {
        StartEngineError {
            source: Box::new(err),
        }
    }
}
