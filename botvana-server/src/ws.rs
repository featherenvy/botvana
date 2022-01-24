use std::{net::SocketAddr, rc::Rc, time::Duration};

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
const SNAPSHOT_TICK_INTERVAL_MS: u64 = 1000;

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

        //task::spawn(accept_connection(peer, stream, global_state.clone()));

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
    // let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    // let mut interval =
    //     async_std::stream::interval(Duration::from_millis(SNAPSHOT_TICK_INTERVAL_MS));
    // let mut msg_fut = ws_receiver.next();
    // let mut tick_fut = interval.next();
    use std::time::Instant;

    info!("New WebSocket connection from: {}", peer);

    send_state(&mut ws_stream, &global_state).await?;
    let mut last_state = Instant::now();

    loop {
        match ws_stream.next().await {
            Some(_) => {}
            None => return Ok(()),
        }

        if last_state.elapsed() > Duration::from_millis(SNAPSHOT_TICK_INTERVAL_MS) {
            send_state(&mut ws_stream, &global_state).await?;
            last_state = Instant::now();
        }

        // match select(msg_fut, tick_fut).await {
        //     Either::Left((msg, tick_fut_continue)) => {
        //         match msg {
        //             Some(msg) => {
        //                 let msg = msg?;
        //                 if msg.is_text() || msg.is_binary() {
        //                     ws_sender.send(msg).await?;
        //                 } else if msg.is_close() {
        //                     break;
        //                 }
        //                 tick_fut = tick_fut_continue; // Continue waiting for tick.
        //                 msg_fut = ws_receiver.next(); // Receive next WebSocket message.
        //             }
        //             None => break, // WebSocket stream terminated.
        //         };
        //     }
        //     Either::Right((_, msg_fut_continue)) => {
        //         let connected_bots = global_state.connected_bots().await;
        //         let markets = global_state.markets().await;
        //         ws_sender
        //             .send(Message::Text(
        //                 json!({
        //                     "connected_bots": connected_bots,
        //                     "markets": markets
        //                 })
        //                 .to_string(),
        //             ))
        //             .await?;
        //         msg_fut = msg_fut_continue; // Continue receiving the WebSocket message.
        //         tick_fut = interval.next(); // Wait for next tick.
        //     }
        // }
    }
}

async fn send_state(
    ws_stream: &mut async_tungstenite::WebSocketStream<glommio::net::TcpStream>,
    state: &state::GlobalState,
) -> Result<()> {
    let connected_bots = state.connected_bots().await;
    let markets = state.markets().await;
    Ok(ws_stream
        .send(Message::Text(
            json!({
                "connected_bots": connected_bots,
                "markets": markets
            })
            .to_string(),
        ))
        .await?)
}
