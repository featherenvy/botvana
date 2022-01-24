use serde::Deserialize;

/// Configuration for the bot server
#[derive(Deserialize)]
pub struct BotServerConfig {
    pub listen_address: String,
}

/// Configuration for Websocket gateway
#[derive(Deserialize)]
pub struct WebsocketServerConfig {
    pub listen_address: String,
}

/// Generic configuration for botnodes
#[derive(Clone, Deserialize)]
pub struct BotnodeConfig {
    pub markets: Box<[Box<str>]>,
    pub exchanges: Box<[Box<str>]>,
}

/// botvana-server configuration
#[derive(Deserialize)]
pub struct ServerConfig {
    pub bot_server: BotServerConfig,
    pub ws_server: WebsocketServerConfig,
    pub botnode: Box<[BotnodeConfig]>,
}
