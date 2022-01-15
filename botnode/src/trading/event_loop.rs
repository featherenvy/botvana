use crate::exchange::{ExchangeEvent, ExchangeRequest};
use crate::prelude::*;

/// Runs trading event loop
pub async fn run_loop(
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
    exchange_tx: spsc_queue::Producer<ExchangeRequest>,
    exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    loop {
        if shutdown.shutdown_started() {
            return Ok(());
        }

        for (exchange, market_data_rx) in market_data_rxs.iter() {
            if let Some(event) = market_data_rx.try_pop() {
                info!("market_event = {} {:?}", exchange, event);
            }
        }

        if let Some(event) = indicator_rx.try_pop() {
            trace!("indicator = {:?}", event);
        }

        if let Some(event) = exchange_rx.try_pop() {
            trace!("exchange = {:?}", event);
        }
    }
}
