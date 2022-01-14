use std::thread;

use crate::prelude::*;
use crate::{
    audit::*, engine::*, exchange::engine::*, indicator::engine::*, market_data::*,
    trading::engine::*,
};

use super::BotnodeStatus;

const CONSUMER_LIMIT: usize = 16;

/// Control engine for Botnode
///
/// Spawns engines per given configuration and connects to botvana-server.
pub struct ControlEngine {
    pub(super) bot_id: BotId,
    pub(super) server_addr: String,
    pub(super) status: BotnodeStatus,
    pub(super) ping_interval: std::time::Duration,
    pub(super) bot_configuration: Option<BotConfiguration>,
    config_txs: ArrayVec<spsc_queue::Producer<BotConfiguration>, CONSUMER_LIMIT>,
}

impl ControlEngine {
    /// Create new control engine
    pub fn new<T: ToString>(bot_id: BotId, server_addr: T) -> Self {
        Self {
            bot_id,
            server_addr: server_addr.to_string(),
            status: BotnodeStatus::Offline,
            ping_interval: std::time::Duration::from_secs(5),
            config_txs: ArrayVec::<_, CONSUMER_LIMIT>::new(),
            bot_configuration: None,
        }
    }

    /// Spawns the engines based on given configuration and wires them up using channels.
    pub(super) fn spawn_engines(
        &mut self,
        config: BotConfiguration,
        shutdown: Shutdown,
    ) -> Result<(), ()> {
        let mut market_data_rxs = vec![HashMap::new(), HashMap::new()];
        let n_exchanges = config.exchanges.len();

        for (i, exchange) in config.exchanges.iter().enumerate() {
            debug!("starting exchange {:?}", exchange);

            self.spawn_market_engine(
                i + 1,
                exchange.as_ref(),
                shutdown.clone(),
                &mut market_data_rxs,
            )
            .expect(&format!("Failed to start {} market data engine", exchange));
        }

        let exchange_engine = ExchangeEngine::new(
            self.data_rx(),
            crate::exchange::null_adapter::NullAdapter {},
        );

        let mut indicator_engine =
            IndicatorEngine::new(self.data_rx(), market_data_rxs.pop().unwrap());

        let trading_engine =
            TradingEngine::new(market_data_rxs.pop().unwrap(), indicator_engine.data_rx());

        spawn_engine(n_exchanges + 2, AuditEngine::new(), shutdown.clone())
            .expect("failed to start audit engine");

        spawn_engine(n_exchanges + 3, indicator_engine, shutdown.clone())
            .expect("failed to start indicator engine");

        spawn_engine(n_exchanges + 4, trading_engine, shutdown.clone())
            .expect("failed to start trading engine");

        spawn_engine(n_exchanges + 5, exchange_engine, shutdown.clone())
            .expect("failed to start order engine");

        Ok(())
    }

    fn spawn_market_engine(
        &mut self,
        cpu: usize,
        exchange: &str,
        shutdown: Shutdown,
        market_data_rxs: &mut Vec<HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>>,
    ) -> Result<thread::JoinHandle<()>, StartEngineError> {
        match exchange {
            "ftx" => {
                let ftx_adapter = crate::market_data::ftx::Ftx::default();
                let mut market_data_engine = MarketDataEngine::new(self.data_rx(), ftx_adapter);

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(Box::from(exchange), market_data_engine.data_rx());
                });

                spawn_engine(cpu, market_data_engine, shutdown)
            }
            "binance" => {
                let binance_adapter = crate::market_data::binance::Binance::default();
                let mut market_data_engine = MarketDataEngine::new(self.data_rx(), binance_adapter);

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(Box::from(exchange), market_data_engine.data_rx());
                });

                spawn_engine(cpu, market_data_engine, shutdown)
            }
            _ => {
                error!("Unknown exchange {}", exchange);
                Err(StartEngineError {
                    source: "asd".into(),
                })
            }
        }
    }
}

#[async_trait(?Send)]
impl Engine for ControlEngine {
    type Data = BotConfiguration;

    /// Returns engine name
    fn name(&self) -> String {
        "control-engine".to_string()
    }

    /// Start the control engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting control engine");

        async_std::task::sleep(std::time::Duration::from_secs(1)).await;

        while let Err(e) = super::event_loop::run_control_loop(&mut self, shutdown.clone()).await {
            error!("Control engine error: {:?}", e);
            async_std::task::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }

    /// Returns dummy data receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (config_tx, config_rx) = spsc_queue::make(1);
        self.config_txs.push(config_tx);
        config_rx
    }

    /// Returns config transmitters used to notify other engines
    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        self.config_txs.as_slice()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Control engine error: {msg}")]
pub struct ControlEngineError {
    pub(super) msg: &'static str,
}
