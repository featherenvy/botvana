use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ExchangeRef {
    Ftx,
    BinanceSpot,
}

impl std::fmt::Display for ExchangeRef {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        std::fmt::Debug::fmt(self, f)
    }
}

impl FromStr for ExchangeRef {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ftx" | "Ftx" | "FTX" => Ok(ExchangeRef::Ftx),
            "binance" | "Binance" | "binance_spot" | "BinanceSpot" => Ok(ExchangeRef::BinanceSpot),
            _ => Err(format!("Unknown exchange: {}", s)),
        }
    }
}
