//! Module holding all configuration types
use serde::{Deserialize, Serialize};

use crate::net::msg::BotId;

/// Configuration for botnode
///
/// This configuration is sent to the bot after
/// receiving correct `Hello` message.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BotConfiguration {
    pub bot_id: BotId,
    pub peer_bots: Box<[PeerBot]>,
    pub markets: Box<[Box<str>]>,
    pub indicators: Box<[IndicatorConfig]>,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct PeerBot {
    pub bot_id: BotId,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub enum IndicatorConfig {
    Midprice,
}
