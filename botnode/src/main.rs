use std::env::var;
use std::panic;

use async_shutdown::Shutdown;
use futures::prelude::*;
use glommio::LocalExecutor;
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use tracing::{debug, error, info};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use botnode::{audit::*, control::*, engine::*, indicator::*, market_data::*, trading::*};
use botvana::net::msg::BotId;

#[global_allocator]
static ALLOC: snmalloc_rs::SnMalloc = snmalloc_rs::SnMalloc;

fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer().with_thread_names(true))
        .init();

    let (bot_id, server_addr) = load_configuration();

    let shutdown = Shutdown::new();

    {
        let shutdown = shutdown.clone();

        panic::set_hook(Box::new(move |p| {
            error!("Panic coming from one of the threads, exiting");
            debug!("panic = {:?}", p);
            shutdown.shutdown();
        }));
    }

    // Stage 1: create the engines & wire them up

    let mut control_engine = ControlEngine::new(bot_id, server_addr);

    let ftx_adapter = botnode::market_data::ftx::Ftx {
        metrics: botnode::market_data::ftx::FtxMetrics::default(),
    };
    let mut market_data_engine = MarketDataEngine::new(control_engine.data_rx(), ftx_adapter);

    let mut indicator_engine =
        IndicatorEngine::new(control_engine.data_rx(), market_data_engine.data_rx());

    let trading_engine =
        TradingEngine::new(market_data_engine.data_rx(), indicator_engine.data_rx());

    // Stage 2: start the engines

    start_engine(4, AuditEngine::new(), shutdown.clone()).expect("failed to start audit engine");

    start_engine(3, trading_engine, shutdown.clone()).expect("failed to start trading engine");

    start_engine(2, indicator_engine, shutdown.clone()).expect("failed to start indicator engine");

    start_engine(1, market_data_engine, shutdown.clone())
        .expect("failed to start market data engine");

    start_engine(0, control_engine, shutdown.clone()).expect("failed to start control engine");

    // Setup signal handlers for shutdown
    let signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT]).expect("Failed to register signals");
    let local_ex = LocalExecutor::default();
    local_ex.run(handle_signals(signals, shutdown));
}

/// Loads configuration from ENV variables
///
/// Panics if the BOT_ID or SERVER_ADDR variables are missing or
/// BOT_ID can't be parsed as u16 number.
fn load_configuration() -> (BotId, String) {
    let bot_id = var("BOT_ID")
        .expect("Please specify BOT_ID")
        .parse::<BotId>()
        .expect("BOT_ID must be u16 number");

    info!("bot_id = {}", bot_id.0);

    let server_addr = var("SERVER_ADDR").expect("Please specify SERVER_ADDR");

    (bot_id, server_addr)
}

/// Handles shutdown signals from OS
///
/// The function will wait for one of SIGTERM, SIGINT or SIGQUIT signals
/// and starts shutdown procedure when it recieves any of them.
async fn handle_signals(signals: Signals, shutdown: Shutdown) {
    let mut signals = signals.fuse();

    while let Some(signal) = signals.next().await {
        match signal {
            SIGTERM | SIGINT | SIGQUIT => {
                info!("Shutting down");
                shutdown.shutdown();
                break;
            }
            _ => unreachable!(),
        }
    }

    shutdown.wait_shutdown_complete().await;

    info!("Shutdown complete: bye");

    std::process::exit(0);
}
