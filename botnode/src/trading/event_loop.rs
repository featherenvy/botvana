use crate::exchange::{ExchangeEvent, ExchangeRequest};
use crate::prelude::*;

/// Runs trading event loop
pub fn run_loop(
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
                match event.r#type {
                    MarketEventType::OrderbookUpdate(market, orderbook) => {
                        let bid = orderbook.bids.price_vec.last().unwrap_or(&0.0);
                        let ask = orderbook.asks.price_vec.first().unwrap_or(&0.0);
                        info!("{}/{}: {}/{}", exchange, market, bid, ask);
                    }
                    _ => {}
                }
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
