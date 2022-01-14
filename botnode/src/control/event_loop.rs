use crate::prelude::*;
use super::engine::*;
use super::BotnodeStatus;

/// Runs the Botnode control engine that runs the connection to Botvana
///
/// This connects to Botvana server on a given address, sends the Hello
/// message and runs the loop.
pub(crate) async fn run_control_loop(
    control: &mut super::engine::ControlEngine,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    // Get a token to delay shutdown until the token is dropped
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(EngineError::with_source)?;

    let mut framed = connect_websocket(control).await?;

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

async fn connect_websocket(
    control: &mut ControlEngine,
) -> Result<
    async_codec::Framed<glommio::net::TcpStream, botvana::net::codec::BotvanaCodec>,
    EngineError,
> {
    control.status = BotnodeStatus::Connecting;

    let stream = TcpStream::connect(control.server_addr.clone())
        .await
        .map_err(EngineError::with_source)?;

    let mut framed = Framed::new(stream, BotvanaCodec);

    let msg = Message::hello(control.bot_id.clone());
    if let Err(e) = framed.send(msg).await {
        error!("Error framing the message: {:?}", e);
    }

    Ok(framed)
}
