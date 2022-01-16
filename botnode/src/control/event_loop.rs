use std::time::SystemTime;

use glommio::{channels::local_channel, Latency, Local, Shares};

use super::engine::*;
use super::BotnodeStatus;
use crate::prelude::*;

pub(crate) async fn run_control_loop2(
    mut control: super::engine::ControlEngine,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    // Get a token to delay shutdown until the token is dropped
    let _token = shutdown
        .delay_shutdown_token()
        .map_err(EngineError::with_source)?;

    let tq1 = Local::create_task_queue(Shares::Static(1), Latency::NotImportant, "test1");
    let tq2 = Local::create_task_queue(Shares::Static(1), Latency::NotImportant, "test2");
    let (sender, mut receiver) = local_channel::new_unbounded();
    let ping_interval = control.ping_interval.clone();
    let mut framed = connect_botvana_server(&mut control).await?;

    let t1 = {
        let shutdown = shutdown.clone();
        Local::local_into(
            async move {
                loop {
                    futures::select! {
                        msg = framed.next().fuse() => {
                            debug!("msg = {:?}", msg);

                            match msg {
                                Some(Ok(msg)) => {
                                    sender.try_send(msg).unwrap();
                                }
                                Some(Err(e)) => {
                                    break;
                                }
                                None => {
                                    break;
                                }
                            }
                        }
                        _ = glommio::timer::sleep(ping_interval).fuse() => {
                            if let Err(e) = framed.send(Message::ping()).await {
                                error!("Failed to send ping message: {:?}", e);
                            }
                        }
                        _ = shutdown.wait_shutdown_triggered().fuse() => {
                            break;
                        }
                    };
                }
            },
            tq1,
        )
        .unwrap()
    };

    let t2 = Local::local_into(
        async move {
            let mut stream = receiver.stream();

            loop {
                let msg = stream.next().await.unwrap();

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

                    control
                        .spawn_engines(bot_config.clone(), shutdown.clone())
                        .unwrap();

                    control.push_value(bot_config);
                }
            }
        },
        tq2,
    )
    .unwrap();

    t1.await;
    t2.await;

    Ok(())
}

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
    let msg = framed.next().await;
    match msg {
        Some(Ok(Message::BotConfiguration(bot_config))) => {
            debug!("received config = {:?}", bot_config);

            if matches!(
                control.status,
                BotnodeStatus::Offline | BotnodeStatus::Connecting
            ) {
                control.status = BotnodeStatus::Online;
            }

            debug!("config = {:?}", bot_config);

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

    let mut last_activity = SystemTime::now();

    loop {
        let mut elapsed = last_activity.elapsed().unwrap();

        if shutdown.shutdown_started() {
            info!("shutting down control engine");

            break Ok(());
        }

        if elapsed > control.ping_interval {
            if let Err(e) = framed.send(Message::ping()).await {
                error!("Failed to send ping message: {:?}", e);
            }
            last_activity = SystemTime::now();
            elapsed = last_activity.elapsed().unwrap();
        }

        futures::select! {
            msg = framed.next().fuse() => {
                debug!("msg = {:?}", msg);
            }
            res = &control.market_data_rxs => {
                let (exchange, market_event) = res;
                //info!("exchange={} event={:?}", exchange, market_event);

                match market_event.r#type {
                    MarketEventType::Markets(markets) => {
                        framed.send(Message::market_list(*markets.clone())).await.unwrap();
                        last_activity = SystemTime::now();
                    }
                    _ => {}
                }
            }
            _ = glommio::timer::sleep(control.ping_interval - elapsed).fuse() => {
                if let Err(e) = framed.send(Message::ping()).await {
                    error!("Failed to send ping message: {:?}", e);
                }
                last_activity = SystemTime::now();
            }
            _ = shutdown.wait_shutdown_triggered().fuse() => {
                break Ok(());
            }
        }
    }
}

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
        error!("Error framing the message: {:?}", e);
    }

    Ok(framed)
}
