use crate::prelude::*;

#[derive(Debug, Default)]
struct IndicatorState {
    symbols_vec: Vec<Box<str>>,
    tob_vec: Vec<(f64, f64)>,
    timestamp_vec: Vec<f64>,
}

impl IndicatorState {
    fn update_tob(&mut self, time: f64, market_symbol: Box<str>, bid: f64, ask: f64) {
        match self.symbols_vec.iter().position(|s| *s == market_symbol) {
            Some(pos) => {
                *self.tob_vec.get_mut(pos).unwrap() = (bid, ask);
                *self.timestamp_vec.get_mut(pos).unwrap() = time;
            }
            None => {
                self.tob_vec.push((bid, ask));
                self.timestamp_vec.push(time);
                self.symbols_vec.push(market_symbol);
            }
        }
    }
}

/// Indicator engine loop
pub async fn run_indicator_loop(
    market_data_rxs: ConsumersMap<Box<str>, MarketEvent>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(EngineError::with_source)?;

    let mut indicator_state = IndicatorState::default();

    loop {
        if shutdown.shutdown_started() {
            info!("shutting down indicator engine");

            break Ok(());
        }

        for (_, market_data_rx) in market_data_rxs.iter() {
            if let Some(event) = market_data_rx.try_pop() {
                //info!("market_event = {:?}", event);
                if let Err(e) = process_market_event(event, &mut indicator_state) {
                    error!("Failed to process market event: {}", e);
                }
            }
        }
    }
}

/// Handles incoming [`MarketEvent`]
fn process_market_event(
    event: MarketEvent,
    indicator_state: &mut IndicatorState,
) -> Result<(), DynBoxError> {
    match event.r#type {
        MarketEventType::Markets(markets) => {
            trace!("Received {} markets", markets.len());
        }
        MarketEventType::Trades(market_symbol, trades) => {
            if !trades.is_empty() {
                let diff = trades[0].received_at.elapsed();
                trace!("{} core latency = {} us", market_symbol, diff.as_micros());
            }
        }
        MarketEventType::OrderbookUpdate(market_symbol, orderbook) => {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_micros() as f64
                / 1_000_000.0;
            let delay = now - orderbook.time;
            let bid = orderbook.bids.price_vec.last().unwrap_or(&0.0);
            let ask = orderbook.asks.price_vec.first().unwrap_or(&0.0);

            indicator_state.update_tob(orderbook.time, market_symbol.clone(), *bid, *ask);

            trace!("{}: {}/{} ({} delay)", market_symbol, bid, ask, delay,)
        }
        MarketEventType::MidPriceChange(market_symbol, bid, ask) => {
            trace!("{} bid/ask: {}/{}", market_symbol, bid, ask);
        }
    }
    Ok(())
}
