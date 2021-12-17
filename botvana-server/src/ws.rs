use std::net::SocketAddr;
use std::time::Duration;

use async_std::net::{TcpListener, TcpStream};
use async_std::task;
use async_tungstenite::{
    accept_async,
    tungstenite::{Error, Message, Result},
};
use futures::future::{select, Either};
use futures::prelude::*;
use serde_json::json;
use tracing::*;

use crate::config;
use botvana::state;

// Run TCP listener loop and spawn new task for each incoming connection.
pub async fn run_listener(
    ws_config: config::WebsocketServerConfig,
    global_state: state::GlobalState,
) {
    let listener = TcpListener::bind(&ws_config.listen_address)
        .await
        .expect("Can't listen");
    info!("Listening on: {}", ws_config.listen_address);

    while let Ok((stream, _)) = listener.accept().await {
        let peer = stream
            .peer_addr()
            .expect("connected streams should have a peer address");
        info!("Peer address: {}", peer);

        task::spawn(accept_connection(peer, stream, global_state.clone()));
    }
}

// Accept a connection
async fn accept_connection(peer: SocketAddr, stream: TcpStream, global_state: state::GlobalState) {
    if let Err(e) = handle_connection(peer, stream, global_state).await {
        match e {
            Error::ConnectionClosed | Error::Protocol(_) | Error::Utf8 => (),
            err => error!("Error processing connection: {}", err),
        }
    }
}

async fn handle_connection(
    peer: SocketAddr,
    stream: TcpStream,
    global_state: state::GlobalState,
) -> Result<()> {
    let ws_stream = accept_async(stream).await.expect("Failed to accept");
    info!("New WebSocket connection: {}", peer);
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let mut interval = async_std::stream::interval(Duration::from_millis(1000));

    // Echo incoming WebSocket messages and send a message periodically every second.

    let mut msg_fut = ws_receiver.next();
    let mut tick_fut = interval.next();
    loop {
        match select(msg_fut, tick_fut).await {
            Either::Left((msg, tick_fut_continue)) => {
                match msg {
                    Some(msg) => {
                        let msg = msg?;
                        if msg.is_text() || msg.is_binary() {
                            ws_sender.send(msg).await?;
                        } else if msg.is_close() {
                            break;
                        }
                        tick_fut = tick_fut_continue; // Continue waiting for tick.
                        msg_fut = ws_receiver.next(); // Receive next WebSocket message.
                    }
                    None => break, // WebSocket stream terminated.
                };
            }
            Either::Right((_, msg_fut_continue)) => {
                let connected_bots = global_state.connected_bots().await;
                ws_sender
                    .send(Message::Text(
                        json!({
                            "connected_bots": connected_bots,
                        })
                        .to_string(),
                    ))
                    .await?;
                msg_fut = msg_fut_continue; // Continue receiving the WebSocket message.
                tick_fut = interval.next(); // Wait for next tick.
            }
        }
    }

    Ok(())
}
