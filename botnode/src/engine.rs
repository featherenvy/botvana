use crate::prelude::*;
use botvana::exchange::ExchangeRef;

/// Botnode engines type
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum EngineType {
    AuditEngine,
    ControlEngine,
    ExchangeEngine,
    IndicatorEngine,
    MarketDataEngine(ExchangeRef),
    TradingEngine,
}

/// Engine status
#[derive(Clone, Debug)]
pub enum EngineStatus {
    /// The engine is booting
    Booting,
    /// The engine is running and operating as expected
    Running,
    /// The engine is shutting down
    ShuttingDown,
    /// Engine has encountered error and shut down
    Error,
}

impl Default for EngineStatus {
    fn default() -> Self {
        Self::Booting
    }
}

/// Engine trait
#[async_trait(?Send)]
pub trait Engine {
    /// Returns engine name
    fn name(&self) -> String;

    /// Returns engine health receiver
    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus>;

    /// Start the engine loop
    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError>;
}

/// Engine trait
#[async_trait(?Send)]
pub trait EngineData {
    /// Data that the engine produces
    type Data: Clone + std::fmt::Debug;

    /// Returns receiver for the data engine produces
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (_data_tx, data_rx) = spsc_queue::make::<Self::Data>(1);
        data_rx
    }

    /// Returns trasmitters for market data
    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        unimplemented!()
    }

    /// Pushes value onto all data transmitter
    fn push_value(&self, val: Self::Data) {
        self.data_txs()
            .iter()
            .enumerate()
            .for_each(move |(idx, data_tx)| {
                if data_tx.consumer_disconnected() {
                    warn!("Consumer disconnected {:?}", data_tx);
                }

                let val = val.clone();
                let mut res = data_tx.try_push(val);
                let mut fail_cnt = 0;

                while let Some(value) = res {
                    if fail_cnt > 10 {
                        panic!(
                            "Failed to push value onto producer: idx={idx} value={:?}",
                            value
                        );
                    }
                    fail_cnt += 1;

                    res = Some(value);
                }
            });
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
///     fn name(&self) -> String {
///         "example-engine".to_string()
///     }
///
///     async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
///         Ok(())
///     }
/// }
///
/// spawn_engine(0, ExampleEngine {}, Shutdown::new()).unwrap();
/// ```
pub fn spawn_engine<E: Engine + Send + 'static>(
    cpu: usize,
    engine: E,
    shutdown: Shutdown,
) -> Result<glommio::ExecutorJoinHandle<()>, StartEngineError> {
    LocalExecutorBuilder::new(Placement::Fixed(cpu))
        .spin_before_park(std::time::Duration::from_micros(250))
        .name(&engine.name())
        .spawn(move || async move {
            match engine.start(shutdown).await {
                Ok(_handle) => {}
                Err(e) => {
                    error!("Error starting the engine: {:?}", e);
                }
            }
        })
        .map_err(StartEngineError::from)
}

pub fn await_value<T>(rx: spsc_queue::Consumer<T>) -> T {
    loop {
        if let Some(config) = rx.try_pop() {
            break config;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestData;

    struct TestEngine(Vec<spsc_queue::Producer<TestData>>);

    #[async_trait(?Send)]
    impl Engine for TestEngine {
        fn name(&self) -> String {
            "test-engine".to_string()
        }

        fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
            unimplemented!();
        }

        async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
            loop {
                if shutdown.shutdown_started() {
                    return Ok(());
                }
            }
        }
    }

    fn test_engine() -> TestEngine {
        TestEngine(vec![])
    }

    #[test]
    fn test_engine_shutdown() {
        let engine = test_engine();
        let shutdown = Shutdown::new();

        spawn_engine(0, engine, shutdown.clone()).unwrap();

        shutdown.shutdown();

        assert!(shutdown.shutdown_completed());
    }
}
