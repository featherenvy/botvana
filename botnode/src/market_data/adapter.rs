use async_std::task::sleep;
use async_tungstenite::{async_std::connect_async, tungstenite::Message};

use crate::market_data::{error::MarketDataError, Market};
use crate::{market_data::prelude::*, prelude::*};

/// Market data adapter trait
#[async_trait(?Send)]
pub trait MarketDataAdapter {
    /// Fetches and returns markets information
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError>;

    /// Runs the adapter event loop
    async fn run_loop(
        &mut self,
        data_txs: crate::market_data::MarketDataProducers,
        markets: &[&str],
        shutdown: Shutdown,
    ) -> Result<(), MarketDataError> {
        loop {
            if let Err(e) = self
                .run_exchange_connection_loop(&data_txs, &markets, &shutdown)
                .await
            {
                error!("Error running exchange connection loop: {}", e);
            }

            let wait = Duration::from_secs(5);
            warn!("disconnected from the exchange; waiting for {:?}", wait);
            sleep(wait).await;
        }
    }

    /// Runs the exchange connection event loop
    async fn run_exchange_connection_loop(
        &mut self,
        data_txs: &crate::market_data::MarketDataProducers,
        markets: &[&str],
        shutdown: &Shutdown,
    ) -> Result<Option<MarketEvent>, MarketDataError>;
}

pub enum FeedType {
    Orderbook,
    Trades,
    TopOfBook,
}

/// Websocket adapter for market data
pub trait WsMarketDataAdapter {
    fn ws_url(&self) -> Box<str>;

    /// Returns set of subscribe messages to send to subscribe to given markets
    fn subscribe_msgs(&mut self, markets: &[&str]) -> Box<[String]>;

    /// Returns throughput metrics
    fn throughput_metrics(&self) -> &Throughput<StdInstant, RefCell<metered::common::TxPerSec>>;

    /// Processes Websocket text message
    fn process_ws_msg(
        &self,
        msg: &str,
        markets: &mut HashMap<Box<str>, PlainOrderbook<f64>>,
    ) -> Result<Option<MarketEvent>, MarketDataError>;
}

#[async_trait(?Send)]
pub trait RestMarketDataAdapter {
    /// Fetch orderbook snapshot
    async fn fetch_orderbook_snapshot(
        &self,
        symbol: &str,
    ) -> Result<PlainOrderbook<f64>, MarketDataError>;

    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        Ok(Box::new([]))
    }
}

#[async_trait(?Send)]
impl<T> MarketDataAdapter for T
where
    T: WsMarketDataAdapter + RestMarketDataAdapter,
{
    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError> {
        Ok(Box::new([]))
    }

    /// Runs the exchange connection event loop
    async fn run_exchange_connection_loop(
        &mut self,
        data_txs: &crate::market_data::MarketDataProducers,
        markets: &[&str],
        shutdown: &Shutdown,
    ) -> Result<Option<MarketEvent>, MarketDataError> {
        let _token = shutdown
            .delay_shutdown_token()
            .map_err(MarketDataError::with_source)?;
        let url = self.ws_url();
        info!("connecting to {}", url);
        let (mut ws_stream, _) = connect_async(url.to_string())
            .await
            .map_err(MarketDataError::with_source)?;

        for msg in self.subscribe_msgs(&markets).iter() {
            info!("sending = {}", msg);
            ws_stream
                .send(Message::text(msg))
                .await
                .map_err(MarketDataError::with_source)?;
        }

        let mut markets: HashMap<Box<str>, PlainOrderbook<_>> = markets
            .iter()
            .map(|m| (Box::from(*m), PlainOrderbook::with_capacity(100)))
            .collect();
        let mut start = std::time::Instant::now();

        loop {
            if shutdown.shutdown_started() {
                info!("Market data adapter shutting down");
                break Ok(None);
            }

            let msg = ws_stream.next().await;
            let throughput = self.throughput_metrics();
            measure!(throughput, {
                match msg {
                    Some(Ok(Message::Text(msg))) => match self.process_ws_msg(&msg, &mut markets) {
                        Ok(Some(event)) => data_txs.iter().for_each(|config_tx| {
                            while config_tx.try_push(event.clone()).is_some() {}
                        }),
                        Ok(None) => {}
                        Err(e) => warn!("Failed to process websocket message: {}", e),
                    },
                    Some(Ok(other)) => {
                        warn!(
                            reason = "unexpected-websocket-message",
                            msg = &*other.to_string()
                        );
                    }
                    None | Some(Err(_)) => {
                        error!(reason = "disconnected");
                        break Ok(None);
                    }
                }
            });

            if start.elapsed().as_secs() >= 5 {
                start = std::time::Instant::now();
                info!(
                    "max throughput over last 5s = {:?}",
                    throughput.0.borrow().hdr_histogram.max()
                );
                throughput.clear();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name() {}
}
