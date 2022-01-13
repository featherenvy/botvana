use async_shutdown::Shutdown;
use async_trait::async_trait;
use glommio::{GlommioError, LocalExecutorBuilder};
use tracing::error;

pub struct EngineHandle;

/// Engine trait
#[async_trait(?Send)]
pub trait StartEngine {
    /// Start the engine loop
    async fn start(self, shutdown: Shutdown) -> Result<(), StartEngineError>;
}

/// Error encountered starting the engine
#[derive(Debug, thiserror::Error)]
#[error("Error starting the engine: {source}")]
pub struct StartEngineError {
    source: Box<dyn std::error::Error>,
}

impl<T: 'static> From<glommio::GlommioError<T>> for StartEngineError {
    fn from(err: GlommioError<T>) -> Self {
        StartEngineError {
            source: Box::new(err),
        }
    }
}
pub fn start_engine<E: StartEngine + Send + 'static>(
    cpu: usize,
    engine: E,
    shutdown: Shutdown,
) -> Result<EngineHandle, StartEngineError> {
    LocalExecutorBuilder::new()
        .pin_to_cpu(cpu)
        .spin_before_park(std::time::Duration::from_micros(250))
        .spawn(move || async move {
            match engine.start(shutdown).await {
                Ok(_handle) => {}
                Err(e) => {
                    error!("Error starting the engine: {:?}", e);
                }
            }
        })
        .map_err(StartEngineError::from)?;

    Ok(EngineHandle {})
}
