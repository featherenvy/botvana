use serde::Deserialize;

#[derive(Deserialize)]
pub struct BotServerConfig {
    pub listen_address: String,
}

#[derive(Deserialize)]
pub struct WebsocketServerConfig {
    pub listen_address: String,
}

#[derive(Deserialize)]
pub struct ServerConfig {
    pub bot_server: BotServerConfig,
    pub ws_server: WebsocketServerConfig,
}
