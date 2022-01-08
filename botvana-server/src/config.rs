use serde::Deserialize;

#[derive(Deserialize)]
pub struct BotServerConfig {
    pub listen_address: String,
}

#[derive(Deserialize)]
pub struct WebsocketServerConfig {
    pub listen_address: String,
}

#[derive(Clone, Deserialize)]
pub struct BotnodeConfig {
    pub markets: Box<[String]>,
    pub exchanges: Box<[Box<str>]>,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bot_server: BotServerConfig,
    pub ws_server: WebsocketServerConfig,
    pub botnode: Box<[BotnodeConfig]>,
}
