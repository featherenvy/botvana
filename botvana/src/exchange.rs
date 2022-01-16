use serde::{Deserialize, Serialize};

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
pub enum ExchangeRef {
    Ftx,
    BinanceSpot,
}
