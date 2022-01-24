//! Market Data Adapter
//!
//! This module defines market data adapter traits that when implemented allow
//! the market data engine to operate on any exchange.

use async_tungstenite::{async_std::connect_async, tungstenite::Message};
use glommio::timer::sleep;

use crate::{market_data::prelude::*, prelude::*};
use botvana::{exchange::ExchangeRef, market::MarketVec};

/// Market data adapter trait
#[async_trait(?Send)]
pub trait MarketDataAdapter<const TX_CAP: usize> {
    const NAME: &'static str;
    const EXCHANGE_REF: ExchangeRef;

    /// Fetches and returns markets information
    async fn fetch_markets(&self) -> Result<Box<MarketVec>, MarketDataError>;

    /// Runs the adapter event loop
    async fn run_loop(
        &mut self,
        data_txs: crate::channels::ProducersArray<MarketEvent, TX_CAP>,
        markets: &[&str],
        shutdown: Shutdown,
    ) -> Result<(), MarketDataError> {
        loop {
            if let Err(e) = self
                .run_exchange_connection_loop(&data_txs, &markets, shutdown.clone())
                .await
            {
                error!("Error running exchange connection loop: {e}");
            }

            if shutdown.shutdown_started() {
                break Ok(());
            }

            let wait = Duration::from_secs(5);
            warn!("disconnected from the exchange; waiting for {wait:?}");
            sleep(wait).await;
        }
    }

    /// Runs the exchange connection event loop
    async fn run_exchange_connection_loop(
        &mut self,
        data_txs: &crate::channels::ProducersArray<MarketEvent, TX_CAP>,
        markets: &[&str],
        shutdown: Shutdown,
    ) -> Result<Option<MarketEvent>, MarketDataError>;
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

/// REST-API market data adapter
#[async_trait(?Send)]
pub trait RestMarketDataAdapter {
    const NAME: &'static str;
    const EXCHANGE_REF: ExchangeRef;

    /// Fetch orderbook snapshot for given symbol
    async fn fetch_orderbook_snapshot(
        &self,
        symbol: &str,
    ) -> Result<PlainOrderbook<f64>, MarketDataError>;

    /// Fetches availables markets
    async fn fetch_markets(&self) -> Result<Box<[Market]>, MarketDataError>;
}

#[async_trait(?Send)]
impl<T, const TX_CAP: usize> MarketDataAdapter<TX_CAP> for T
where
    T: WsMarketDataAdapter + RestMarketDataAdapter,
{
    const NAME: &'static str = <T as RestMarketDataAdapter>::NAME;
    const EXCHANGE_REF: ExchangeRef = <T as RestMarketDataAdapter>::EXCHANGE_REF;

    /// Fetches availables markets on Binance
    async fn fetch_markets(&self) -> Result<Box<MarketVec>, MarketDataError> {
        let boxed_markets = <T as RestMarketDataAdapter>::fetch_markets(&self).await?;
        Ok(Box::new(boxed_markets.into()))
    }

    /// Runs the exchange connection event loop
    async fn run_exchange_connection_loop(
        &mut self,
        data_txs: &crate::channels::ProducersArray<MarketEvent, TX_CAP>,
        markets: &[&str],
        shutdown: Shutdown,
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
        let throughput = self.throughput_metrics();

        info!("markets = {:?}", markets);

        loop {
            if shutdown.shutdown_started() {
                info!("Market data adapter shutting down");
                break Ok(None);
            }

            let msg = ws_stream.next().await;
            measure!(throughput, {
                match msg {
                    Some(Ok(Message::Text(msg))) => match self.process_ws_msg(&msg, &mut markets) {
                        Ok(Some(event)) => {
                            data_txs.push_value(event);
                        }
                        Ok(None) => {}
                        Err(e) => warn!("Failed to process websocket message: {e}"),
                    },
                    Some(Ok(Message::Ping(_))) => {
                        debug!(message = "ping",);
                    }
                    Some(Ok(other)) => {
                        warn!(
                            reason = "unexpected-websocket-message",
                            msg = &*other.to_string()
                        );
                    }
                    Some(Err(e)) => {
                        error!(reason = "disconnected", error = &*e.to_string());
                        break Ok(None);
                    }
                    None => {
                        error!(reason = "disconnected");
                        break Ok(None);
                    }
                }
            });

            if start.elapsed().as_secs() >= 5 {
                start = std::time::Instant::now();
                data_txs.0.iter().enumerate().for_each(|(idx, tx)| {
                    info!("{idx} {tx:?}");
                });
                info!(
                    "max throughput over last 5s = {:?}",
                    throughput.0.borrow().hdr_histogram.max()
                );
                throughput.clear();
            }
        }
    }
}
