use std::thread;

use crate::prelude::*;
use crate::{audit::*, engine::*, indicator::*, market_data::*, trading::*};

const CONSUMER_LIMIT: usize = 16;

/// Control engine for Botnode
///
/// The control engine maintains the connection to Botvana server.
pub struct ControlEngine {
    bot_id: BotId,
    server_addr: String,
    status: BotnodeStatus,
    ping_interval: std::time::Duration,
    config_txs: ArrayVec<spsc_queue::Producer<BotConfiguration>, CONSUMER_LIMIT>,
    bot_configuration: Option<BotConfiguration>,
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
    fn spawn_engines(&mut self, config: BotConfiguration, shutdown: Shutdown) -> Result<(), ()> {
        let mut market_data_rxs = vec![HashMap::new(), HashMap::new()];
        let n_exchanges = config.exchanges.len();

        for (i, exchange) in config.exchanges.iter().enumerate() {
            debug!("starting exchange {:?}", exchange);

            self.spawn_market_engine(
                i + 1,
                exchange.as_ref(),
                shutdown.clone(),
                &mut market_data_rxs,
            ).expect(&format!("Failed to start {} market data engine", exchange));
        }

        let mut indicator_engine =
            IndicatorEngine::new(self.data_rx(), market_data_rxs.pop().unwrap());

        let trading_engine =
            TradingEngine::new(market_data_rxs.pop().unwrap(), indicator_engine.data_rx());

        spawn_engine(n_exchanges + 2, AuditEngine::new(), shutdown.clone())
            .expect("failed to start audit engine");

        spawn_engine(n_exchanges + 3, trading_engine, shutdown.clone())
            .expect("failed to start trading engine");

        spawn_engine(n_exchanges + 4, indicator_engine, shutdown.clone())
            .expect("failed to start indicator engine");

        Ok(())
    }

    fn spawn_market_engine(
        &mut self,
        cpu: usize,
        exchange: &str,
        shutdown: Shutdown,
        market_data_rxs: &mut Vec<HashMap<Box<str>, Vec<spsc_queue::Consumer<MarketEvent>>>>,
    ) -> Result<thread::JoinHandle<()>, StartEngineError> {
        match exchange {
            "ftx" => {
                let ftx_adapter = crate::market_data::ftx::Ftx::default();
                let mut market_data_engine = MarketDataEngine::new(self.data_rx(), ftx_adapter);

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(
                        Box::from(exchange),
                        vec![market_data_engine.data_rx(), market_data_engine.data_rx()],
                    );
                });

                spawn_engine(cpu, market_data_engine, shutdown)
            }
            "binance" => {
                let binance_adapter = crate::market_data::binance::Binance::default();
                let mut market_data_engine = MarketDataEngine::new(self.data_rx(), binance_adapter);

                market_data_rxs.iter_mut().for_each(|rx| {
                    rx.insert(
                        Box::from(exchange),
                        vec![market_data_engine.data_rx(), market_data_engine.data_rx()],
                    );
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

        while let Err(e) = run_control_loop(&mut self, shutdown.clone()).await {
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
    msg: &'static str,
}

/// Botnode status
#[derive(Clone, PartialEq)]
enum BotnodeStatus {
    /// Connecting to botvana-server
    Connecting,
    /// Connected to botvana-server
    Online,
    /// Not connected to botvana-server
    Offline,
}

/// Runs the Botnode control engine that runs the connection to Botvana
///
/// This connects to Botvana server on a given address, sends the Hello
/// message and runs the loop.
async fn run_control_loop(
    control: &mut ControlEngine,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    // Get a token to delay shutdown until the token is dropped
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(EngineError::with_source)?;

    control.status = BotnodeStatus::Connecting;

    let stream = TcpStream::connect(control.server_addr.clone())
        .await
        .map_err(EngineError::with_source)?;

    let mut framed = Framed::new(stream, BotvanaCodec);

    let msg = Message::hello(control.bot_id.clone());
    if let Err(e) = framed.send(msg).await {
        error!("Error framing the message: {:?}", e);
    }

    loop {
        futures::select! {
            msg = framed.next().fuse() => {
                debug!("msg = {:?}", msg);

                match msg {
                    Some(Ok(msg)) => {
                        if matches!(
                            control.status,
                            BotnodeStatus::Offline | BotnodeStatus::Connecting
                            ) {
                            control.status = BotnodeStatus::Online;
                        }

                        debug!("received from server = {:?}", msg);

                        if let Message::BotConfiguration(bot_config) = msg {
                            debug!("config = {:?}", bot_config);

                            control.bot_configuration = Some(bot_config.clone());

                            control.spawn_engines(bot_config.clone(), shutdown.clone()).unwrap();

                            control.push_value(bot_config);
                        }
                    }
                    Some(Err(e)) => {
                        return Err(EngineError::with_source(e));
                    }
                    None => {
                        return Err(EngineError::with_source(ControlEngineError {
                            msg: "Disconnected from botvana-server"
                        }));
                    }
                }
            }
            _ = async_std::task::sleep(control.ping_interval).fuse() => {
                if let Err(e) = framed.send(Message::ping()).await {
                    error!("Failed to send ping message: {:?}", e);
                }
            }
            _ = shutdown.wait_shutdown_triggered().fuse() => {
                break Ok(());
            }
        }
    }
}
