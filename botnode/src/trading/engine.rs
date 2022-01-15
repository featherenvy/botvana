use crate::exchange::{ExchangeEvent, ExchangeRequest};
use crate::prelude::*;

/// Trading engine
pub struct TradingEngine {
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
    exchange_tx: spsc_queue::Producer<ExchangeRequest>,
    exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
}

impl TradingEngine {
    pub fn new(
        market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
        indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
        exchange_tx: spsc_queue::Producer<ExchangeRequest>,
        exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
    ) -> Self {
        Self {
            market_data_rxs,
            indicator_rx,
            exchange_tx,
            exchange_rx,
        }
    }
}

#[async_trait(?Send)]
impl Engine for TradingEngine {
    type Data = ();

    fn name(&self) -> String {
        "trading-engine".to_string()
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
        .await
    }

    /// Returns dummy data receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (_data_tx, data_rx) = spsc_queue::make::<()>(1024);
        data_rx
    }
}
