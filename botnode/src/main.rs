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

use botnode::{control::engine::*, engine::*};
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

    // Start the control engine that will connect to botvana-server and
    // receive the configuration. Then the control engine spawns other engines
    // based on the configuration it recieves.
    let control_engine = ControlEngine::new(bot_id, server_addr);
    spawn_engine(0, control_engine, shutdown.clone()).expect("failed to start control engine");

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
