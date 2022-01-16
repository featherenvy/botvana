use std::num::ParseIntError;
use std::str::FromStr;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::cfg::BotConfiguration;
use crate::market::MarketsVec;

/// Botvana protocol message
#[derive(Serialize, Deserialize, Debug)]
pub enum Message {
    /// Hello message
    ///
    /// Bot sends this message when it connects to the server
    Hello(BotId, BotMetadata),
    /// Bot configuration
    ///
    /// This is first send by the server upon receiving `Hello`
    BotConfiguration(BotConfiguration),
    /// Bot error
    ///
    /// Sent by bot when it encounters unrecoverable error.
    BotError(BotError),
    /// Ping
    ///
    /// Prompts a pong response. Inner value is expected to
    /// be timestamp of pong going out.
    Ping(u128),
    /// Pong
    ///
    /// Sent in response to ping.
    Pong(u128),
    /// List of markets that the bot has access to
    MarketList(MarketsVec),
}

impl Message {
    /// Creates new Hello message
    pub fn hello(bot_id: BotId) -> Self {
        Message::Hello(bot_id, BotMetadata::new(1))
    }

    pub fn ping() -> Self {
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => Message::Ping(n.as_nanos()),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }

    pub fn pong() -> Self {
        match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
            Ok(n) => Message::Pong(n.as_nanos()),
            Err(_) => panic!("SystemTime before UNIX EPOCH!"),
        }
    }

    pub fn market_list(markets: MarketsVec) -> Self {
        Self::MarketList(markets)
    }
}

/// Unique ID representing bot
#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
pub struct BotId(pub u16);

impl FromStr for BotId {
    type Err = ParseIntError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(BotId(s.parse::<u16>()?))
    }
}

/// Metadata associated to connected bot
#[derive(Serialize, Deserialize, Debug)]
pub struct BotMetadata {
    pub bot_version: u32,
}

impl BotMetadata {
    pub fn new(bot_version: u32) -> Self {
        Self { bot_version }
    }
}

/// Enum of possible errors reported by botnode
#[derive(Serialize, Deserialize, Debug)]
pub enum BotError {
    ConfigurationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ser_deser_hello() {
        let hello = Message::Hello(BotId(0), BotMetadata::new(1));
        let encoded = bincode::serialize(&hello).unwrap();
        let decoded: Message = bincode::deserialize(&encoded).unwrap();

        match decoded {
            Message::Hello(BotId(bot_id), BotMetadata { bot_version }) => {
                assert_eq!(bot_id, 0);
                assert_eq!(bot_version, 1);
            }
            _ => {
                panic!("unexpected message deserialized");
            }
        }
    }

    #[test]
    fn ser_deser_configuration() {
        let hello = Message::BotConfiguration(BotConfiguration {
            bot_id: BotId(1),
            peer_bots: Box::new([]),
            exchanges: Box::from([Box::from("ftx")]),
            markets: Box::from([Box::from("BTC/USD")]),
            indicators: Box::new([]),
        });
        let encoded = bincode::serialize(&hello).unwrap();
        let decoded: Message = bincode::deserialize(&encoded).unwrap();

        match decoded {
            Message::BotConfiguration(BotConfiguration { bot_id, .. }) => {
                assert_eq!(bot_id.0, 1);
            }
            _ => {
                panic!("unexpected message deserialized");
            }
        }
    }
}
