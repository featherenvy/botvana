//! Market Data Engine

use crate::market_data::{adapter::*, MarketEvent};
use crate::prelude::*;

pub const CONSUMER_LIMIT: usize = 16;

pub type MarketDataProducers = ArrayVec<spsc_queue::Producer<MarketEvent>, CONSUMER_LIMIT>;

/// Engine that maintains connections to exchanges and produces raw market data
pub struct MarketDataEngine<A: MarketDataAdapter> {
    adapter: A,
    config_rx: spsc_queue::Consumer<BotConfiguration>,
    data_txs: MarketDataProducers,
}

impl<A: MarketDataAdapter> MarketDataEngine<A> {
    pub fn new(config_rx: spsc_queue::Consumer<BotConfiguration>, adapter: A) -> Self {
        Self {
            adapter,
            config_rx,
            data_txs: ArrayVec::<_, CONSUMER_LIMIT>::new(),
        }
    }
}

#[async_trait(?Send)]
impl<A: MarketDataAdapter> Engine for MarketDataEngine<A> {
    const NAME: &'static str = "market-data-engine";

    type Data = MarketEvent;

    /// Start the market data engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting market data engine");

        // First, fetch available markets using the adapter
        match self.adapter.fetch_markets().await {
            Ok(markets) => {
                let event = MarketEvent {
                    r#type: MarketEventType::Markets(markets),
                    timestamp: Utc::now(),
                };
                self.push_value(event);
            }
            Err(e) => {
                error!("Failed to fetch market info: {:?}", e);
            }
        };

        // Await configuration from botvana-server
        debug!("Waiting for configuration");
        let config = await_value(self.config_rx);
        debug!("Got config = {:?}", config);
        let markets: Vec<_> = config
            .markets
            .iter()
            .map(|market| market.as_ref())
            .collect();

        info!("Running loop w/ markets = {:?}", config.markets);
        if let Err(e) = self
            .adapter
            .run_loop(self.data_txs, &markets[..], shutdown)
            .await
        {
            error!("Error running loop: {}", e);
        }

        Ok(())
    }

    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        &self.data_txs
    }

    /// Returns cloned market event receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (data_tx, data_rx) = spsc_queue::make(1);
        self.data_txs.push(data_tx);
        data_rx
    }
}
