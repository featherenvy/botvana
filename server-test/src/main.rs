use async_std::net::ToSocketAddrs;

use async_codec::Framed;
use async_std::net::TcpStream;
use futures::prelude::*;
use tracing::{debug, error, info};

use botvana::net::codec::BotvanaCodec;
use botvana::net::msg::{BotId, Message};

async fn test_client<A: ToSocketAddrs>(
    addr: A,
    bot_id: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let stream = TcpStream::connect(addr).await.expect("Failed to connect");
    let mut framed = Framed::new(stream, BotvanaCodec);

    let msg = Message::hello(BotId(bot_id));
    if let Err(e) = framed.send(msg).await {
        error!("Error framing the message: {:?}", e);
    }

    if let Some(msg) = framed.next().await.transpose().unwrap() {
        debug!("received from server = {:?}", msg);
    }

    let msg = Message::ping();
    if let Err(e) = framed.send(msg).await {
        error!("Error framing the message: {:?}", e);
    }

    if let Some(msg) = framed.next().await.transpose().unwrap() {
        info!("received from server = {:?}", msg);
    }

    async_std::task::sleep(std::time::Duration::from_secs(10)).await;

    debug!("client done");

    Ok(())
}

async fn run_parallel_clients(n_clients: u16) {
    let mut handles = vec![];

    info!("Running {} parallel clients", n_clients);

    for n in 1..n_clients {
        handles.push(async_std::task::spawn(async move {
            test_client("127.0.0.1:7978", n)
                .await
                .expect("failed client");
        }));
    }

    handles
        .into_iter()
        .for_each(|f| async_std::task::block_on(async { f.await }));
}

#[async_std::main]
async fn main() {
    tracing_subscriber::fmt().init();

    let n_clients = 128;
    run_parallel_clients(n_clients).await;
}
