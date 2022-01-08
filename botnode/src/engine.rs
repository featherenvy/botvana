use crate::prelude::*;

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
    const NAME: &'static str;

    /// Data that the engine produces
    type Data: Clone;

    /// Start the engine loop
    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError>;

    /// Returns receiver for the data engine produces
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (_data_tx, data_rx) = spsc_queue::make::<Self::Data>(100);
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
            .for_each(|config_tx| while config_tx.try_push(val.clone()).is_some() {});
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
///     const NAME: &'static str = "example-engine";
///
///     type Data = ();
///
///     async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
///         Ok(())
///     }
///
///     fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
///         let (_data_tx, data_rx) = spsc_queue::make::<()>(1024);
///         data_rx
///     }
/// }
///
/// start_engine(0, ExampleEngine {}, Shutdown::new()).unwrap();
/// ```
pub fn start_engine<E: Engine + Send + 'static>(
    cpu: usize,
    engine: E,
    shutdown: Shutdown,
) -> Result<(), StartEngineError> {
    LocalExecutorBuilder::new()
        .pin_to_cpu(cpu)
        .spin_before_park(std::time::Duration::from_micros(250))
        .name(E::NAME)
        .spawn(move || async move {
            match engine.start(shutdown).await {
                Ok(_handle) => {}
                Err(e) => {
                    error!("Error starting the engine: {:?}", e);
                }
            }
        })
        .map_err(StartEngineError::from)?;

    Ok(())
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
        const NAME: &'static str = "test-engine";

        type Data = TestData;

        async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
            loop {
                if shutdown.shutdown_started() {
                    return Ok(())
                }
            }
        }

        fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
            let (data_tx, data_rx) = spsc_queue::make(1);
            self.0.push(data_tx);
            data_rx
        }
    }

    fn test_engine() -> TestEngine {
        TestEngine(vec![])
    }

    #[test]
    fn test_engine_shutdown() {
        let engine = test_engine();
        let shutdown = Shutdown::new();

        start_engine(0, engine, shutdown.clone()).unwrap();

        shutdown.shutdown();

        assert!(shutdown.shutdown_completed());
    }
}
