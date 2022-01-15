use serde::{Deserialize, Deserializer};

use botvana::market::orderbook::*;

#[derive(Debug, Deserialize)]
pub struct OrderbookSnapshot {
    #[serde(rename = "lastUpdateId")]
    pub last_update_id: u64,
    #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
    pub bids: PriceLevelsVec<f64>,
    #[serde(deserialize_with = "deserialize_into_price_levels_vec")]
    pub asks: PriceLevelsVec<f64>,
}

fn deserialize_into_price_levels_vec<'de, D>(
    deserializer: D,
) -> Result<PriceLevelsVec<f64>, D::Error>
where
    D: Deserializer<'de>,
{
    let buf = Box::<[(String, String)]>::deserialize(deserializer)?;

    let mut levels: Vec<(f64, f64)> = buf
        .iter()
        .map(|(price, size)| (price.parse::<f64>().unwrap(), size.parse::<f64>().unwrap()))
        .collect();

    Ok(PriceLevelsVec::from_tuples_vec_unsorted(&mut levels))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExchangeInfo<'a> {
    _server_time: f64,
    #[serde(borrow)]
    pub symbols: Box<[SymbolInfo<'a>]>,
}

impl<'a> TryFrom<&SymbolInfo<'a>> for botvana::market::Market {
    type Error = Box<dyn std::error::Error>;

    fn try_from(symbol_info: &SymbolInfo<'a>) -> Result<Self, Self::Error> {
        let size_increment = 1.0 / 10_i32.pow(symbol_info.base_asset_precision as u32) as f64;
        let price_increment = 1.0 / 10_i32.pow(symbol_info.quote_asset_precision as u32) as f64;

        Ok(Self {
            name: symbol_info.symbol.to_string(),
            native_symbol: symbol_info.symbol.to_string(),
            size_increment,
            price_increment,
            r#type: botvana::market::MarketType::Spot(botvana::market::SpotMarket {
                base: symbol_info.base_asset.to_string(),
                quote: symbol_info.quote_asset.to_string(),
            }),
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SymbolInfo<'a> {
    pub symbol: &'a str,
    pub status: &'a str,
    pub base_asset: &'a str,
    pub base_asset_precision: u8,
    pub quote_asset: &'a str,
    pub quote_asset_precision: u8,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_orderbook_snapshot_parse() {
        let json: &str = include_str!("../../../tests/binance_api_v3_depth.json");

        let _: OrderbookSnapshot = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_try_from_symbol_info_for_markets() {
        let symbol_info = SymbolInfo {
            symbol: "BTCUSDT",
            status: "TRADING",
            base_asset: "BTC",
            base_asset_precision: 8,
            quote_asset: "USDT",
            quote_asset_precision: 2,
        };

        let market = botvana::market::Market::try_from(&symbol_info).unwrap();

        assert_eq!(market.price_increment, 0.01);
        assert_eq!(market.size_increment, 0.00000001);
    }
}
