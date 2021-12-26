//! Indicator engine
use crate::prelude::*;

const CONSUMER_LIMIT: usize = 16;

/// Indicator producing engine
pub struct IndicatorEngine {
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    data_txs: ArrayVec<spsc_queue::Producer<IndicatorEvent>, CONSUMER_LIMIT>,
    indicators_config: Box<[IndicatorConfig]>,
    market_data_rx: spsc_queue::Consumer<MarketEvent>,
}

#[derive(Clone, Debug)]
pub enum IndicatorEvent {}

impl IndicatorEngine {
    pub fn new(
        config_rx: spsc_queue::Consumer<BotConfiguration>,
        market_data_rx: spsc_queue::Consumer<MarketEvent>,
    ) -> Self {
        Self {
            config_rx,
            data_txs: ArrayVec::<_, CONSUMER_LIMIT>::new(),
            indicators_config: Box::new([]),
            market_data_rx,
        }
    }
}

#[async_trait(?Send)]
impl Engine for IndicatorEngine {
    type Data = IndicatorEvent;

    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting indicator engine");

        let config = await_configuration(self.config_rx);
        debug!("config = {:?}", config);
        self.indicators_config = config.indicators.into_boxed_slice();

        run_indicator_loop(self.market_data_rx, shutdown).await
    }

    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        &self.data_txs
    }

    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (data_tx, data_rx) = spsc_queue::make(1);
        self.data_txs.push(data_tx);
        data_rx
    }
}

impl ToString for IndicatorEngine {
    fn to_string(&self) -> String {
        "indicator-engine".to_string()
    }
}

/// Indicator engine loop
pub async fn run_indicator_loop(
    market_data_rx: spsc_queue::Consumer<MarketEvent>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(EngineError::with_source)?;

    loop {
        if shutdown.shutdown_started() {
            info!("shutting down indicator engine");

            break Ok(());
        }

        if let Some(event) = market_data_rx.try_pop() {
            info!("market_event = {:?}", event);
            if let Err(e) = process_market_event(event) {
                error!("Failed to process market event: {}", e);
            }
        }
    }
}

pub fn process_market_event(event: MarketEvent) -> Result<(), DynBoxError> {
    match event {
        MarketEvent::Trades(trades) => {
            if !trades.is_empty() {
                let diff = trades[0].received_at.elapsed();
                info!("core latency = {} us", diff.as_micros());
            }
        }
        _ => {}
    }
    Ok(())
}
