use std::{
    net::SocketAddr,
    rc::Rc,
    time::{Duration, Instant},
};

use async_tungstenite::{
    accept_async,
    tungstenite::{Message, Result},
};
use futures::prelude::*;
use glommio::{
    enclose,
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    Task,
};
use serde_json::json;
use tracing::*;

use crate::config;
use botvana::state;

const MAX_CONNECTIONS: u64 = 1024;
const SNAPSHOT_TICK_INTERVAL_MS: u64 = 500;
const READ_TIMEOUT_US: u64 = 500;

/// Runs TCP listener loop and spawn new task for each incoming connection.
pub async fn run_listener(
    ws_config: config::WebsocketServerConfig,
    global_state: state::GlobalState,
) {
    let listener = TcpListener::bind(&ws_config.listen_address).expect("Can't listen");
    let conn_control = Rc::new(Semaphore::new(MAX_CONNECTIONS as _));
    info!("Listening on: {}", ws_config.listen_address);

    while let Ok(stream) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        {
            let global_state = global_state.clone();

            Task::local(enclose! { (conn_control) async move {
                let _permit = conn_control
                    .acquire_permit(1)
                    .await
                    .expect("failed to acquire permit");

                if let Err(e) = handle_connection(peer, stream, global_state).await {
                    error!("Error while handling the connection: {}", e);
                }
            }})
            .detach();
        }
    }
}

/// Handles connection and runs the connection loop
async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    global_state: state::GlobalState,
) -> Result<()> {
    let mut ws_stream = accept_async(stream).await.expect("Failed to accept");

    info!("New WebSocket connection from: {}", peer);

    send_state(&mut ws_stream, &global_state).await?;
    let mut last_state = Instant::now();

    loop {
        let _ = glommio::timer::timeout(Duration::from_micros(READ_TIMEOUT_US), async {
            ws_stream.next().await;
            Ok(())
        })
        .await;

        if last_state.elapsed() >= Duration::from_millis(SNAPSHOT_TICK_INTERVAL_MS) {
            send_state(&mut ws_stream, &global_state).await?;
            last_state = Instant::now();
        }
    }
}

async fn send_state(
    ws_stream: &mut async_tungstenite::WebSocketStream<glommio::net::TcpStream>,
    state: &state::GlobalState,
) -> Result<()> {
    let connected_bots = state.connected_bots();
    let markets = state.markets();
    let orderbooks = state.orderbooks();

    Ok(ws_stream
        .send(Message::Text(
            json!({
                "connected_bots": connected_bots,
                "markets": markets,
                "orderbooks": orderbooks,
            })
            .to_string(),
        ))
        .await?)
}
