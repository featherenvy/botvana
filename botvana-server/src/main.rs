use async_shutdown::Shutdown;
use figment::{
    providers::{Format, Toml},
    Figment,
};
use futures::prelude::*;
use glommio::{prelude::*, CpuSet};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use tide::prelude::*;
use tide::Request;
use tracing::{debug, error, info};
use tracing_subscriber::EnvFilter;

use botvana::state;
use botvana_server::*;

fn main() {
    let shutdown = Shutdown::new();

    let subscriber = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .finish();

    tracing::subscriber::set_global_default(subscriber).expect("failed to set global subscriber");

    let config: config::ServerConfig = Figment::new()
        .merge(Toml::file("cfg/default.toml"))
        .extract()
        .expect("Failed to load configuration");

    let state = state::GlobalState::new();
    let state_ref = state.clone();

    LocalExecutorBuilder::new()
        .pin_to_cpu(0)
        .spawn(|| async move {
            tide::log::start();
            let mut app = tide::with_state(state_ref);

            app.at("/")
                .get(|req: Request<state::GlobalState>| async move {
                    let state = req.state();
                    let connected_bots: Vec<_> = state
                        .connected_bots()
                        .await
                        .iter()
                        .map(|bot_id| bot_id.0)
                        .collect();

                    Ok(json!({
                        "connected_bots": connected_bots,
                    }))
                });

            app.listen("127.0.0.1:8080")
                .await
                .expect("Tide listener failed");
        })
        .unwrap();

    let state_ref = state.clone();
    LocalExecutorBuilder::new()
        .pin_to_cpu(1)
        .spawn(|| async move {
            ws::run_listener(config.ws_server, state_ref).await;
        })
        .unwrap();

    LocalExecutorPoolBuilder::new(2)
        .placement(Placement::MaxSpread(CpuSet::online().ok()))
        .on_all_shards(|| async move {
            debug!("Starting executor");

            if let Err(e) =
                bot_server::serve::<_>(config.bot_server.listen_address, 4096, state).await
            {
                error!("Error while serving: {}", e);
            }
        })
        .unwrap()
        .join_all();

    // Setup signal handlers for shutdown
    let signals = Signals::new(&[SIGINT, SIGTERM, SIGQUIT]).expect("Failed to register signals");
    let local_ex = LocalExecutor::default();
    local_ex.run(handle_signals(signals, shutdown));
}

/// Handles shutdown signals from OS
async fn handle_signals(signals: Signals, shutdown: Shutdown) {
    let mut signals = signals.fuse();

    while let Some(signal) = signals.next().await {
        match signal {
            SIGTERM | SIGINT | SIGQUIT => {
                // Shutdown the system;
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
