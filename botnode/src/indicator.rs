//! Indicator engine
use crate::prelude::*;

/// Indicator producing engine
pub struct IndicatorEngine {
    data_tx: ring_channel::RingSender<IndicatorEvent>,
    data_rx: ring_channel::RingReceiver<IndicatorEvent>,
    market_data_rx: ring_channel::RingReceiver<MarketEvent>,
}

#[derive(Debug)]
pub enum IndicatorEvent {}

impl IndicatorEngine {
    pub fn new(market_data_rx: ring_channel::RingReceiver<MarketEvent>) -> Self {
        let (data_tx, data_rx) = ring_channel::ring_channel(NonZeroUsize::new(1024).unwrap());
        Self {
            data_tx,
            data_rx,
            market_data_rx,
        }
    }
}

#[async_trait(?Send)]
impl Engine for IndicatorEngine {
    type Data = IndicatorEvent;

    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting indicator engine");

        run_indicator_loop(self.market_data_rx, shutdown).await
    }

    fn data_rx(&self) -> RingReceiver<Self::Data> {
        self.data_rx.clone()
    }
}

impl ToString for IndicatorEngine {
    fn to_string(&self) -> String {
        "indicator-engine".to_string()
    }
}

/// Indicator engine loop
pub async fn run_indicator_loop(
    mut market_data_rx: ring_channel::RingReceiver<MarketEvent>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(|e| EngineError::with_source(e))?;

    loop {
        if shutdown.shutdown_started() {
            info!("shutting down indicator engine");

            break Ok(());
        }
        match market_data_rx.try_recv() {
            Ok(event) => {
                //info!("market_event = {:?}", event);
                if let Err(e) = process_market_event(event) {
                    error!("Failed to process market event: {}", e);
                }
            }
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => break Ok(()),
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
