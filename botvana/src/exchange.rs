use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// Exchange identification enum
#[derive(Copy, Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum ExchangeId {
    Ftx,
    BinanceSpot,
    Serum,
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
            "serum" | "Serum" | "serum_dex" => Ok(ExchangeId::Serum),
            _ => Err(format!("Unknown exchange: {}", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exchange_id_from_str() {
        assert_eq!(ExchangeId::Ftx, "ftx".parse::<ExchangeId>().unwrap());
        assert_eq!(ExchangeId::Ftx, "ftx".parse::<ExchangeId>().unwrap());
        assert_eq!(
            ExchangeId::BinanceSpot,
            "binance".parse::<ExchangeId>().unwrap()
        );
        assert_eq!(
            ExchangeId::BinanceSpot,
            "binance_spot".parse::<ExchangeId>().unwrap()
        );
    }

    #[test]
    fn exchange_id_display() {
        assert_eq!("Ftx", format!("{}", ExchangeId::Ftx));
        assert_eq!("BinanceSpot", format!("{}", ExchangeId::BinanceSpot));
    }
}
