//! Market Data Engine

use crate::{market_data::adapter::*, prelude::*};

pub const MARKET_DATA_QUEUE_LEN: usize = 512;

/// Market Data Engine
///
/// It maintains connection to the exchange and produces raw market data.
pub struct MarketDataEngine<A: MarketDataAdapter<TX_CAP>, const TX_CAP: usize> {
    adapter: A,
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    data_txs: crate::channels::ProducersArray<MarketEvent, TX_CAP>,
    status_tx: spsc_queue::Producer<EngineStatus>,
    status_rx: spsc_queue::Consumer<EngineStatus>,
}

impl<A: MarketDataAdapter<TX_CAP>, const TX_CAP: usize> MarketDataEngine<A, TX_CAP> {
    pub fn new(config_rx: spsc_queue::Consumer<BotConfiguration>, adapter: A) -> Self {
        let (status_tx, status_rx) = spsc_queue::make(1);
        Self {
            adapter,
            config_rx,
            data_txs: crate::channels::ProducersArray::<MarketEvent, TX_CAP>::default(),
            status_tx,
            status_rx,
        }
    }
}

#[async_trait(?Send)]
impl<A: MarketDataAdapter<TX_CAP>, const TX_CAP: usize> Engine for MarketDataEngine<A, TX_CAP> {
    fn name(&self) -> String {
        format!("market-data-{}", A::NAME)
    }

    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
        self.status_rx.clone()
    }

    /// Start the market data engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting market data engine for {}", A::NAME);

        self.status_tx.try_push(EngineStatus::Booting);

        // First, fetch available markets using the adapter
        match self.adapter.fetch_markets().await {
            Ok(markets) => {
                let event = MarketEvent::markets(markets.into());
                self.push_value(event);
            }
            Err(e) => {
                error!("Failed to fetch market info: {e:?}");
            }
        };

        // Await configuration from botvana-server
        debug!("Waiting for configuration");
        let config = await_value(self.config_rx);
        debug!("Got config = {config:?}");
        let markets: Vec<_> = config
            .markets
            .iter()
            .map(|market| market.as_ref())
            .collect();

        self.status_tx.try_push(EngineStatus::Running);

        info!("Running loop w/ markets = {:?}", config.markets);
        if let Err(e) = self
            .adapter
            .run_loop(self.data_txs, &markets[..], shutdown)
            .await
        {
            error!("Error running loop: {e}");
            self.status_tx.try_push(EngineStatus::Error);
        }

        Ok(())
    }
}

#[async_trait(?Send)]
impl<A: MarketDataAdapter<TX_CAP>, const TX_CAP: usize> EngineData for MarketDataEngine<A, TX_CAP> {
    type Data = MarketEvent;

    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        &self.data_txs.0
    }

    /// Returns cloned market event receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (data_tx, data_rx) = spsc_queue::make(MARKET_DATA_QUEUE_LEN);
        self.data_txs.0.push(data_tx);
        data_rx
    }
}
