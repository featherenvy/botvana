use std::env::var;

use async_shutdown::Shutdown;
use futures::prelude::*;
use glommio::LocalExecutor;
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use tracing::info;

use botnode::{audit::*, control::*, engine::*, indicator::*, market_data::*, trading::*};
use botvana::net::msg::BotId;

fn main() {
    tracing_subscriber::fmt().with_thread_names(true).init();

    let (bot_id, server_addr) = load_configuration();

    let shutdown = Shutdown::new();

    let control_engine = ControlEngine::new(bot_id, server_addr);
    let config_rx = control_engine.data_rx();

    start_engine(0, control_engine, shutdown.clone()).expect("failed to start control engine");

    let ftx_adapter = botnode::market_data::ftx::Ftx {};
    let market_data_engine = MarketDataEngine::new(config_rx.clone(), ftx_adapter);
    let market_data_rx = market_data_engine.data_rx();

    start_engine(1, market_data_engine, shutdown.clone())
        .expect("failed to start market data engine");

    let indicator_engine = IndicatorEngine::new(config_rx.clone(), market_data_rx.clone());
    let indicator_rx = indicator_engine.data_rx();

    start_engine(2, indicator_engine, shutdown.clone()).expect("failed to start indicator engine");

    let trading_engine = TradingEngine::new(market_data_rx, indicator_rx);

    start_engine(3, trading_engine, shutdown.clone()).expect("failed to start trading engine");

    start_engine(4, AuditEngine::new(), shutdown.clone()).expect("failed to start audit engine");

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
