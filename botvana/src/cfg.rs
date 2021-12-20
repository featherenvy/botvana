//! Module holding all configuration types
use serde::{Deserialize, Serialize};

use crate::net::msg::BotId;

/// Configuration for botnode
///
/// This configuration is sent to the bot after
/// receiving correct `Hello` message.
#[derive(Serialize, Deserialize, Debug)]
pub struct BotConfiguration {
    pub bot_id: BotId,
    pub peer_bots: Vec<PeerBot>,
    pub market_data: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PeerBot {
    pub bot_id: BotId,
}
