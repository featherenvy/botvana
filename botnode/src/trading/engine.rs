use crate::{
    exchange::{ExchangeEvent, ExchangeRequest},
    prelude::*,
};

/// Trading engine
pub struct TradingEngine {
    market_data_rxs: ConsumersMap<Box<str>, MarketEvent>,
    indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
    exchange_tx: spsc_queue::Producer<ExchangeRequest>,
    exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
    status_tx: spsc_queue::Producer<EngineStatus>,
    status_rx: spsc_queue::Consumer<EngineStatus>,
}

impl TradingEngine {
    pub fn new(
        market_data_rxs: ConsumersMap<Box<str>, MarketEvent>,
        indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
        exchange_tx: spsc_queue::Producer<ExchangeRequest>,
        exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
    ) -> Self {
        let (status_tx, status_rx) = spsc_queue::make(1);
        Self {
            market_data_rxs,
            indicator_rx,
            exchange_tx,
            exchange_rx,
            status_tx,
            status_rx,
        }
    }
}

#[async_trait(?Send)]
impl Engine for TradingEngine {
    fn name(&self) -> String {
        "trading-engine".to_string()
    }

    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
        self.status_rx.clone()
    }

    /// Starts the trading engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting trading engine");

        super::event_loop::run_loop(
            self.market_data_rxs,
            self.indicator_rx,
            self.exchange_tx,
            self.exchange_rx,
            shutdown,
        )
    }
}
