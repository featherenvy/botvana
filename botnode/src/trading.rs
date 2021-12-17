//! Trading engine
use crate::prelude::*;

/// Trading engine
pub struct TradingEngine {
    market_data_rx: RingReceiver<MarketEvent>,
    indicator_rx: RingReceiver<IndicatorEvent>,
}

impl TradingEngine {
    pub fn new(
        market_data_rx: RingReceiver<MarketEvent>,
        indicator_rx: RingReceiver<IndicatorEvent>,
    ) -> Self {
        Self {
            market_data_rx,
            indicator_rx,
        }
    }
}

#[async_trait(?Send)]
impl Engine for TradingEngine {
    type Data = ();

    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting trading engine");

        run_trading_loop(self.market_data_rx, self.indicator_rx, shutdown)
            .await
            .map_err(|_| EngineError {})
    }

    /// Returns dummy data receiver
    fn data_rx(&self) -> ring_channel::RingReceiver<Self::Data> {
        let (_data_tx, data_rx) =
            ring_channel::ring_channel::<()>(NonZeroUsize::new(1024).unwrap());
        data_rx
    }
}

impl ToString for TradingEngine {
    fn to_string(&self) -> String {
        "trading-engine".to_string()
    }
}

/// Trading engine loop
pub async fn run_trading_loop(
    mut market_data_rx: RingReceiver<MarketEvent>,
    mut indicator_rx: RingReceiver<IndicatorEvent>,
    _shutdown: Shutdown,
) -> Result<(), Box<dyn std::error::Error>> {
    loop {
        match market_data_rx.try_recv() {
            Ok(event) => {
                info!("market data = {:?}", event);
            }
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => break Ok(()),
        }

        match indicator_rx.try_recv() {
            Ok(event) => {
                info!("indicator = {:?}", event);
            }
            Err(TryRecvError::Empty) => continue,
            Err(TryRecvError::Disconnected) => break Ok(()),
        }
    }
}
