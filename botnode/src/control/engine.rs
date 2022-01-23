use botvana::exchange::ExchangeRef;

use crate::{
    audit::engine::*, engine::*, exchange::engine::*, indicator::engine::*, market_data::*,
    prelude::*, trading::engine::*,
};

use super::BotnodeStatus;

const CONSUMER_LIMIT: usize = 16;
const QUEUE_LEN: usize = 1024;

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
    pub(super) status_rxs: HashMap<EngineType, spsc_queue::Consumer<EngineStatus>>,
    pub(super) market_data_rxs: ConsumersMap<Box<str>, MarketEvent>,
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
            market_data_rxs: ConsumersMap::default(),
            status_rxs: HashMap::new(),
        }
    }

    /// Spawns the engines based on given configuration and wires them up using channels.
    pub(super) fn spawn_engines(
        &mut self,
        config: BotConfiguration,
        shutdown: Shutdown,
    ) -> Result<(), ()> {
        let n_exchanges = config.exchanges.len();
        // Build market data receiver hashmap for each client:
        //  - trading engine
        //  - indicator engine
        //  - control engine
        //  - audit engine
        let mut market_data_rxs = vec![
            ConsumersMap::with_capacity(n_exchanges),
            ConsumersMap::with_capacity(n_exchanges),
            ConsumersMap::with_capacity(n_exchanges),
            ConsumersMap::with_capacity(n_exchanges),
        ];

        for (i, exchange) in config.exchanges.iter().enumerate() {
            debug!("starting exchange {:?}", exchange);

            self.spawn_market_engine(
                i + 1,
                exchange.as_ref(),
                shutdown.clone(),
                &mut market_data_rxs,
            )
            .expect(&format!("Failed to start {exchange} market data engine"));
        }

        self.market_data_rxs = market_data_rxs.pop().unwrap();

        let (exchange_request_tx, exchange_request_rx) = spsc_queue::make(100);

        let mut exchange_engine = ExchangeEngine::new(
            self.data_rx(),
            crate::exchange::null_adapter::NullAdapter {},
            exchange_request_rx,
        );

        self.status_rxs
            .insert(EngineType::ExchangeEngine, exchange_engine.status_rx());

        let mut indicator_engine =
            IndicatorEngine::new(self.data_rx(), market_data_rxs.pop().unwrap());

        self.status_rxs
            .insert(EngineType::IndicatorEngine, indicator_engine.status_rx());

        let trading_engine = TradingEngine::new(
            market_data_rxs.pop().unwrap(),
            indicator_engine.data_rx(),
            exchange_request_tx,
            exchange_engine.data_rx(),
        );

        self.status_rxs
            .insert(EngineType::TradingEngine, trading_engine.status_rx());

        let audit_engine = AuditEngine::new(market_data_rxs.pop().unwrap());

        self.status_rxs
            .insert(EngineType::AuditEngine, audit_engine.status_rx());

        spawn_engine(n_exchanges + 3, indicator_engine, shutdown.clone())
            .expect("failed to start indicator engine");

        spawn_engine(n_exchanges + 4, trading_engine, shutdown.clone())
            .expect("failed to start trading engine");

        spawn_engine(n_exchanges + 5, exchange_engine, shutdown.clone())
            .expect("failed to start order engine");

        spawn_engine(n_exchanges + 6, audit_engine, shutdown.clone())
            .expect("failed to start audit engine");

        Ok(())
    }

    fn spawn_market_engine(
        &mut self,
        cpu: usize,
        exchange: &str,
        shutdown: Shutdown,
        market_data_rxs: &mut Vec<ConsumersMap<Box<str>, MarketEvent>>,
    ) -> Result<glommio::ExecutorJoinHandle<()>, StartEngineError> {
        match exchange {
            "ftx" => {
                let ftx_adapter = crate::market_data::ftx::Ftx::default();
                let mut market_data_engine =
                    MarketDataEngine::<_, 4>::new(self.data_rx(), ftx_adapter);

                self.status_rxs.insert(
                    EngineType::MarketDataEngine(ExchangeRef::Ftx),
                    market_data_engine.status_rx(),
                );

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(Box::from(exchange), market_data_engine.data_rx());
                });

                spawn_engine(cpu, market_data_engine, shutdown)
            }
            "binance" => {
                let binance_adapter = crate::market_data::binance::Binance::default();
                let mut market_data_engine =
                    MarketDataEngine::<_, 4>::new(self.data_rx(), binance_adapter);

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(Box::from(exchange), market_data_engine.data_rx());
                });

                self.status_rxs.insert(
                    EngineType::MarketDataEngine(ExchangeRef::BinanceSpot),
                    market_data_engine.status_rx(),
                );

                spawn_engine(cpu, market_data_engine, shutdown)
            }
            _ => {
                error!("Unknown exchange {exchange}");
                Err(StartEngineError {
                    source: "asd".into(),
                })
            }
        }
    }
}

#[async_trait(?Send)]
impl Engine for ControlEngine {
    /// Returns engine name
    fn name(&self) -> String {
        "control-engine".to_string()
    }

    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
        unimplemented!();
    }

    /// Start the control engine
    async fn start(mut self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting control engine");

        glommio::timer::sleep(std::time::Duration::from_secs(1)).await;

        while let Err(e) = super::event_loop::run_control_loop(&mut self, shutdown.clone()).await {
            error!("Control engine error: {e:?}");
            glommio::timer::sleep(std::time::Duration::from_secs(1)).await;
        }

        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Control engine error: {msg}")]
pub struct ControlEngineError {
    pub(super) msg: &'static str,
}

impl EngineData for ControlEngine {
    type Data = BotConfiguration;

    /// Returns dummy data receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (config_tx, config_rx) = spsc_queue::make(QUEUE_LEN);
        self.config_txs.push(config_tx);
        config_rx
    }

    /// Returns config transmitters used to notify other engines
    fn data_txs(&self) -> &[spsc_queue::Producer<Self::Data>] {
        self.config_txs.as_slice()
    }
}
