use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Exchange identification enum
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ExchangeId {
    Ftx,
    BinanceSpot,
}

impl std::fmt::Display for ExchangeId {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl FromStr for ExchangeId {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ftx" | "Ftx" | "FTX" => Ok(ExchangeId::Ftx),
            "binance" | "Binance" | "binance_spot" | "BinanceSpot" => Ok(ExchangeId::BinanceSpot),
            _ => Err(format!("Unknown exchange: {}", s)),
        }
    }
}
