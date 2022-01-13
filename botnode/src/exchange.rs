pub(crate) mod adapter;

use crate::exchange::adapter::ExchangeAdapter;
use crate::prelude::*;

/// Exchange engine for Botnode
pub struct ExchangeEngine<A> {
    adapter: A,
    config_rx: spsc_queue::Consumer<BotConfiguration>,
}

impl<A: ExchangeAdapter> ExchangeEngine<A> {
    pub fn new(config_rx: spsc_queue::Consumer<BotConfiguration>, adapter: A) -> Self {
        Self { adapter, config_rx }
    }
}

#[async_trait(?Send)]
impl<A: ExchangeAdapter> Engine for ExchangeEngine<A> {
    type Data = ExchangeEvent;

    fn name(&self) -> String {
        "order-engine".to_string()
    }

    async fn start(self, _shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting order engine");

        run_event_loop(self.config_rx)?;

        Ok(())
    }
}

/// Runs the order event loop
fn run_event_loop(config_rx: spsc_queue::Consumer<BotConfiguration>) -> Result<(), EngineError> {
    let config = await_value(config_rx);
    info!("got config = {:?}", config);

    loop {}
}

#[derive(Clone)]
pub enum ExchangeEvent {
    BalanceChange,
}
