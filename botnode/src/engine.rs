use crate::prelude::*;

/// Handle to engine
pub struct EngineHandle {}

/// Botnode engines type
pub enum EngineType {
    AuditEngine,
    ControlEngine,
    MarketDataEngine,
    TradingEngine,
}

/// Engine trait
#[async_trait(?Send)]
pub trait Engine {
    /// Data that the engine produces
    type Data;

    /// Start the engine loop
    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError>;

    /// Returns receiver for the data engine produces
    fn data_rx(&self) -> RingReceiver<Self::Data> {
        let (_data_tx, data_rx) =
            ring_channel::ring_channel::<Self::Data>(NonZeroUsize::new(1).unwrap());
        data_rx
    }
}

/// Starts given engine in new executor pinned to given CPU.
///
/// # Examples
///
/// ```
/// use botnode::prelude::*;
///
/// struct ExampleEngine;
///
/// #[async_trait(?Send)]
/// impl Engine for ExampleEngine {
///     type Data = ();
///
///     async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
///         Ok(())
///     }
///
///     fn data_rx(&self) -> ring_channel::RingReceiver<Self::Data> {
///         let (_data_tx, data_rx) =
///            ring_channel::ring_channel::<()>(NonZeroUsize::new(1024).unwrap());
///         data_rx
///     }
/// }
///
/// impl ToString for ExampleEngine {
///     fn to_string(&self) -> String {
///         "example".to_string()
///     }
/// }
///
/// start_engine(0, ExampleEngine {}, Shutdown::new()).unwrap();
/// ```
pub fn start_engine<E: Engine + ToString + Send + 'static>(
    cpu: usize,
    engine: E,
    shutdown: Shutdown,
) -> Result<EngineHandle, StartEngineError> {
    LocalExecutorBuilder::new()
        .pin_to_cpu(cpu)
        .spin_before_park(std::time::Duration::from_micros(250))
        .name(&engine.to_string())
        .spawn(move || async move {
            match engine.start(shutdown).await {
                Ok(_handle) => {}
                Err(e) => {
                    error!("Error starting the engine: {:?}", e);
                }
            }
        })
        .map_err(|e| StartEngineError::from(e))?;

    Ok(EngineHandle {})
}
