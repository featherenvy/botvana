use std::time::SystemTime;

use super::engine::*;
use super::BotnodeStatus;
use crate::prelude::*;

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
    let mut framed = connect_botvana_server(control).await?;

    // Await the first message expected to be bot configuration
    let msg = framed.next().await;
    let mut last_activity = SystemTime::now();

    process_bot_configuration(control, msg, shutdown.clone())?;

    loop {
        if shutdown.shutdown_started() {
            info!("shutting down control engine");

            break Ok(());
        }

        let elapsed = last_activity.elapsed().unwrap();

        if elapsed > control.ping_interval {
            if let Err(e) = framed.send(Message::ping()).await {
                error!("Failed to send ping message: {e:?}");
            }
            last_activity = SystemTime::now();
        }

        for (engine, status_rx) in control.status_rxs.iter() {
            if status_rx.producer_disconnected() {
                warn!("Engine {engine:?} disconnected!");
            }

            let status = status_rx.try_pop();
            if let Some(status) = status {
                info!("EngineStatus: {engine:?} = {status:?}");
            }
        }

        for (_exchange, rx) in control.market_data_rxs.iter() {
            match rx.try_pop() {
                Some(MarketEvent {
                    r#type: MarketEventType::Markets(markets),
                    ..
                }) => {
                    framed
                        .send(Message::market_list(*markets.clone()))
                        .await
                        .unwrap();
                    last_activity = SystemTime::now();
                }
                Some(_) | None => {}
            }
        }

        let msg =
            glommio::timer::timeout(Duration::from_micros(50), async { Ok(framed.next().await) });

        // Check if the stream has yielded a value
        if let Ok(msg) = msg.await {
            debug!("got msg from botvana-server: {msg:?}");
        }
    }
}

/// Opens a connection to botvana server
async fn connect_botvana_server(
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
        error!("Error framing the message: {e:?}");
    }

    Ok(framed)
}

fn process_bot_configuration<E: 'static + std::error::Error>(
    control: &mut super::engine::ControlEngine,
    msg: Option<Result<Message, E>>,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    match msg {
        Some(Ok(Message::BotConfiguration(bot_config))) => {
            debug!("received config = {bot_config:?}");

            if matches!(
                control.status,
                BotnodeStatus::Offline | BotnodeStatus::Connecting
            ) {
                control.status = BotnodeStatus::Online;
            }

            debug!("config = {bot_config:?}");

            control.bot_configuration = Some(bot_config.clone());

            control
                .spawn_engines(bot_config.clone(), shutdown.clone())
                .unwrap();

            control.push_value(bot_config);
        }
        Some(Err(e)) => {
            return Err(EngineError::with_source(e));
        }
        _ => {
            return Err(EngineError::with_source(ControlEngineError {
                msg: "Disconnected from botvana-server",
            }));
        }
    }

    Ok(())
}
