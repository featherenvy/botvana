/// Trading engine loop
use crate::prelude::*;

pub async fn run_loop(
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    loop {
        if shutdown.shutdown_started() {
            return Ok(());
        }

        for (exchange, market_data_rx) in market_data_rxs.iter() {
            if let Some(event) = market_data_rx.try_pop() {
                trace!("market_event = {} {:?}", exchange, event);
            }
        }

        if let Some(event) = indicator_rx.try_pop() {
            trace!("indicator = {:?}", event);
        }
    }
}
