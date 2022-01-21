use crate::exchange::adapter::ExchangeAdapter;
use crate::prelude::*;

/// Exchange engine for Botnode
pub struct ExchangeEngine<A> {
    _adapter: A,
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    _request_rx: spsc_queue::Consumer<super::ExchangeRequest>,
    status_tx: spsc_queue::Producer<EngineStatus>,
    status_rx: spsc_queue::Consumer<EngineStatus>,
}

impl<A: ExchangeAdapter> ExchangeEngine<A> {
    pub fn new(
        config_rx: spsc_queue::Consumer<BotConfiguration>,
        adapter: A,
        request_rx: spsc_queue::Consumer<super::ExchangeRequest>,
    ) -> Self {
        let (status_tx, status_rx) = spsc_queue::make(1);
        Self {
            _adapter: adapter,
            config_rx,
            _request_rx: request_rx,
            status_tx,
            status_rx,
        }
    }
}

#[async_trait(?Send)]
impl<A: ExchangeAdapter> Engine for ExchangeEngine<A> {
    fn name(&self) -> String {
        "order-engine".to_string()
    }

    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
        self.status_rx.clone()
    }

    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting order engine");

        run_event_loop(self.config_rx, shutdown)?;

        Ok(())
    }
}

#[async_trait(?Send)]
impl<A: ExchangeAdapter> EngineData for ExchangeEngine<A> {
    type Data = super::ExchangeEvent;
}

/// Runs the order event loop
fn run_event_loop(
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    let config = await_value(config_rx);
    info!("got config = {:?}", config);

    loop {
        if shutdown.shutdown_started() {
            break Ok(());
        }
    }
}
