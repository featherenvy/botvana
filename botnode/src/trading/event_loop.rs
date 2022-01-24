use crate::exchange::{ExchangeEvent, ExchangeRequest};
use crate::prelude::*;

const STALE_MARKET_EVENT_MS: u64 = 10;

/// Runs trading event loop
pub fn run_loop(
    market_data_rxs: ConsumersMap<Box<str>, MarketEvent>,
    indicator_rx: spsc_queue::Consumer<IndicatorEvent>,
    _exchange_tx: spsc_queue::Producer<ExchangeRequest>,
    exchange_rx: spsc_queue::Consumer<ExchangeEvent>,
    status_tx: spsc_queue::Producer<EngineStatus>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    let mut prices = HashMap::new();

    status_tx.try_push(EngineStatus::Running);

    loop {
        if shutdown.shutdown_started() {
            return Ok(());
        }

        for (exchange, market_data_rx) in market_data_rxs.iter() {
            if let Some(event) = market_data_rx.try_pop() {
                let elapsed = event.timestamp.elapsed().unwrap();

                if elapsed > Duration::from_millis(STALE_MARKET_EVENT_MS) {
                    warn!("Received stale market data: {elapsed:?}");
                    continue;
                }

                process_market_event(exchange, event, elapsed, &mut prices)?
            }
        }

        if let Some(event) = indicator_rx.try_pop() {
            trace!("indicator = {event:?}");
        }

        if let Some(event) = exchange_rx.try_pop() {
            trace!("exchange = {event:?}");
        }
    }
}

#[inline]
fn process_market_event(
    exchange: &str,
    event: MarketEvent,
    elapsed: Duration,
    prices: &mut HashMap<String, (f64, f64)>,
) -> Result<(), EngineError> {
    match event.r#type {
        MarketEventType::OrderbookUpdate(market, orderbook) => {
            let bid = orderbook.bids.price_vec.last();
            let ask = orderbook.asks.price_vec.first();

            match (bid, ask) {
                (Some(bid), Some(ask)) => {
                    let key = format!("{exchange}-{market}");
                    let old_price = prices.get_mut(&key);
                    let bid = bid.clone();
                    let ask = ask.clone();

                    if let Some((old_bid, old_ask)) = old_price {
                        if *old_bid != bid || *old_ask != ask {
                            trace!("{exchange} {market}: {bid}/{ask} (elapsed={elapsed:?})");
                        }
                    }

                    prices.insert(key.clone(), (bid, ask));
                }
                _ => {
                    warn!("Missing bid or ask: {bid:?}/{ask:?}");
                }
            }
        }
        _ => {}
    }

    Ok(())
}
