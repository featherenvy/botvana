use crate::prelude::*;

/// Controller trait
trait Controller {
    fn start_engine(self, cpu: usize, shutdown: Shutdown) -> Result<(), StartEngineError>;
}

struct EngineController<E> {
    engine: E,
    cpu: Option<usize>,
    shutdown: Option<Shutdown>,
}

impl<E> From<E> for EngineController<E> {
    fn from(engine: E) -> Self {
        Self {
            engine,
            cpu: None,
            shutdown: None,
        }
    }
}

impl<E> Controller for EngineController<E>
where
    E: Engine + Send + 'static,
{
    fn start_engine(mut self, cpu: usize, shutdown: Shutdown) -> Result<(), StartEngineError> {
        self.cpu = Some(cpu);
        self.shutdown = Some(shutdown.clone());

        LocalExecutorBuilder::new()
            .pin_to_cpu(cpu)
            .spin_before_park(std::time::Duration::from_micros(250))
            .name(E::NAME)
            .spawn(move || async move {
                match self.engine.start(shutdown).await {
                    Ok(_handle) => {}
                    Err(e) => {
                        error!("Error starting the engine: {:?}", e);
                    }
                }
            })
            .map_err(StartEngineError::from)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug)]
    struct TestData;

    struct TestEngine;

    #[async_trait(?Send)]
    impl Engine for TestEngine {
        const NAME: &'static str = "test-engine";

        type Data = ();

        async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
            loop {
                if shutdown.shutdown_started() {
                    return Ok(());
                }
            }
        }

        fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
            let (_data_tx, data_rx) = spsc_queue::make(1);
            data_rx
        }
    }

    fn test_engine() -> TestEngine {
        TestEngine {}
    }

    #[test]
    fn test_engine_shutdown() {
        let engine = test_engine();
        let shutdown = Shutdown::new();

        start_engine(0, engine, shutdown.clone()).unwrap();

        shutdown.shutdown();

        assert!(shutdown.shutdown_completed());
    }

    #[test]
    fn name() {}
}
