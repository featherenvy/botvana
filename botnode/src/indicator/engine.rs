use crate::prelude::*;

const CONSUMER_LIMIT: usize = 16;
const QUEUE_LEN: usize = 1024;

/// Indicator producing engine
pub struct IndicatorEngine {
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    data_txs: ArrayVec<spsc_queue::Producer<IndicatorEvent>, CONSUMER_LIMIT>,
    indicators_config: Box<[IndicatorConfig]>,
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
}

impl IndicatorEngine {
    pub fn new(
        config_rx: spsc_queue::Consumer<BotConfiguration>,
        market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    ) -> Self {
        Self {
            config_rx,
            data_txs: ArrayVec::<_, CONSUMER_LIMIT>::new(),
            indicators_config: Box::new([]),
            market_data_rxs,
        }
    }
}

#[async_trait(?Send)]
impl Engine for IndicatorEngine {
    type Data = IndicatorEvent;

    fn name(&self) -> String {
        "indicator-engine".to_string()
    }

    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting indicator engine");

        let config = await_value(self.config_rx);
        debug!("config = {:?}", config);
        self.indicators_config = config.indicators;

        super::event_loop::run_indicator_loop(self.market_data_rxs, shutdown).await
    }

    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        &self.data_txs
    }

    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (data_tx, data_rx) = spsc_queue::make(QUEUE_LEN);
        self.data_txs.push(data_tx);
        data_rx
    }
}
