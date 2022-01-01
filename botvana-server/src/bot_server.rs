use std::{rc::Rc, time::Duration};

use async_codec::Framed;
use async_std::net::ToSocketAddrs;
use futures::prelude::*;
use futures::stream::StreamExt;
use glommio::{enclose, net::TcpListener, net::TcpStream, sync::Semaphore, timer::sleep, Task};
use tracing::{debug, error, info, warn};

use botvana::{
    cfg::{BotConfiguration, PeerBot},
    net::{
        codec,
        msg::{BotId, Message},
    },
    state,
};

use crate::config::BotnodeConfig;

const ACTIVITY_TIMEOUT_SECS: u64 = 15;

#[derive(thiserror::Error, Debug)]
pub enum BotServerError {
    #[error("error while reading socket")]
    ReadError,
    #[error("error while reading socket")]
    WriteError,
    #[error("timeout: no activity")]
    Timeout,
    #[error("duplicate hello sent by the client")]
    DuplicateHello,
    #[error("unknown bot id supplied")]
    UnknownBotID,
}

/// Bot server loop
pub async fn serve<A>(
    addr: A,
    max_connections: usize,
    global_state: state::GlobalState,
    botnode_configs: Box<[BotnodeConfig]>,
) -> std::io::Result<()>
where
    A: ToSocketAddrs + std::net::ToSocketAddrs,
{
    let listener = TcpListener::bind(addr)?;
    let conn_control = Rc::new(Semaphore::new(max_connections as _));

    loop {
        let stream = match listener.accept().await {
            Ok(stream) => stream,
            Err(x) => {
                error!("Error accepting stream: {:?}", x);

                return Err(x.into());
            }
        };
        let global_state = global_state.clone();

        {
            let botnode_configs = botnode_configs.clone();
            Task::local(enclose! { (conn_control) async move {
                let _permit = conn_control
                    .acquire_permit(1)
                    .await
                    .expect("failed to acquire permit");
                let mut stream = Framed::new(stream, codec::BotvanaCodec);

                if let Err(e) = handle_connection(&mut stream, global_state, botnode_configs).await {
                    error!("Error while handling the connection: {}", e);
                }
            }})
            .detach();
        }
    }
}

/// Handle an incoming connection from the bot
pub async fn handle_connection(
    stream: &mut Framed<TcpStream, codec::BotvanaCodec>,
    global_state: state::GlobalState,
    botnode_configs: Box<[BotnodeConfig]>,
) -> Result<(), BotServerError> {
    let mut conn_bot_id = None;

    loop {
        futures::select! {
            frame = stream.next().fuse() => {
                let frame = match frame {
                    Some(Ok(frame)) => frame,
                    _ => {
                        if let Some(conn_bot_id) = conn_bot_id {
                            global_state.remove_bot(conn_bot_id).await;
                        }

                        if frame.is_none() {
                            return Ok(())
                        } else {
                            return Err(BotServerError::ReadError)
                        }
                    }
                };

                debug!("received frame={:?} botid={:?}", frame, conn_bot_id);

                process_bot_message(stream, &mut conn_bot_id, global_state.clone(), &botnode_configs, frame)
                    .await
                    .expect("Failed to process");
            }
            _ = sleep(Duration::from_secs(ACTIVITY_TIMEOUT_SECS)).fuse() => {
                warn!("Timeout while waiting for activity");

                break Err(BotServerError::Timeout)
            }
        }
    }
}

/// Process one message coming from the bot over the network
pub async fn process_bot_message(
    stream: &mut Framed<TcpStream, codec::BotvanaCodec>,
    conn_bot_id: &mut Option<BotId>,
    global_state: state::GlobalState,
    botnode_configs: &[BotnodeConfig],
    msg: Message,
) -> Result<(), BotServerError> {
    match msg {
        Message::Hello(bot_id, bot_metadata) => {
            // If the bot has sent Hello message before, we don't accept it and
            // break the current connection
            if let Some(conn_bot_id) = conn_bot_id {
                warn!("Bot {:?} sending duplicate Hello message", conn_bot_id);
                global_state.remove_bot(conn_bot_id.clone()).await;
                return Err(BotServerError::DuplicateHello);
            }

            if bot_id.0 >= botnode_configs.len() as u16 {
                warn!("Invalid bot id supplied: {:?}", bot_id);
                return Err(BotServerError::UnknownBotID);
            }

            // Save the bot_id in local variable that's scoped for this
            // connection only
            *conn_bot_id = Some(bot_id.clone());

            let bots = global_state.connected_bots().await;

            global_state.add_bot(bot_id.clone()).await;

            let peer_bots = bots
                .iter()
                .map(|id| PeerBot { bot_id: id.clone() })
                .collect();

            info!(
                "Hello from bot id = {:?}, metadata = {:?}; total = {}",
                bot_id,
                bot_metadata,
                bots.len()
            );

            let markets = botnode_configs
                .get(bot_id.0 as usize)
                .unwrap()
                .markets
                .clone();
            let out_msg = Message::BotConfiguration(BotConfiguration {
                bot_id: bot_id.clone(),
                peer_bots,
                markets,
                indicators: Box::new([]),
            });
            info!("Sending bot configuration {:?}", out_msg);

            stream
                .send(out_msg)
                .await
                .map_err(|_| BotServerError::WriteError)?;
        }
        Message::Ping(timestamp) => {
            debug!("received ping {}", timestamp);

            stream
                .send(Message::pong())
                .await
                .map_err(|_| BotServerError::WriteError)?;
        }
        msg => {
            warn!("Unhandled message = {:?} from bot {:?}", msg, conn_bot_id);
        }
    }

    Ok(())
}
