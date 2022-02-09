use serde::Deserialize;

use botvana::exchange::ExchangeId;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Market<'a> {
    name: &'a str,
    base_currency: &'a str,
    quote_currency: &'a str,
    version: u64,
    address: &'a str,
    program_id: &'a str,
    base_mint_address: &'a str,
    quote_mint_address: &'a str,
    tick_size: f64,
    min_order_size: f64,
    deprecated: bool,
}

impl<'a> TryFrom<&'a Market<'a>> for botvana::market::Market {
    type Error = String;

    fn try_from(market: &'a Market<'a>) -> Result<Self, Self::Error> {
        let r#type = botvana::market::MarketType::Spot(botvana::market::SpotMarket {
            base: market.base_currency.to_string(),
            quote: market.quote_currency.to_string(),
        });

        Ok(Self {
            exchange: ExchangeId::Serum,
            name: market.name.to_string(),
            native_symbol: market.name.to_string(),
            size_increment: 0.0000001,
            price_increment: market.tick_size,
            r#type,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_markets() {
        let sample = r#"[{
    "name": "BTC/USDC",
    "baseCurrency": "BTC",
    "quoteCurrency": "USDC",
    "version": 3,
    "address": "A8YFbxQYFVqKZaoYJLLUVcQiWP7G2MeEgW5wsAQgMvFw",
    "programId": "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
    "baseMintAddress": "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E",
    "quoteMintAddress": "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
    "tickSize": 0.1,
    "minOrderSize": 0.0001,
    "deprecated": false
  }]"#;

        serde_json::from_slice::<Vec<Market>>(sample.as_bytes()).unwrap();
    }

    #[test]
    fn test_try_from_market() {
        let serum_market = super::Market {
            name: "BTC/USD",
            base_currency: "BTC",
            quote_currency: "USDC",
            version: 3,
            address: "A8YFbxQYFVqKZaoYJLLUVcQiWP7G2MeEgW5wsAQgMvFw",
            program_id: "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",
            base_mint_address: "9n4nbM75f5Ui33ZbPYXn59EwSgE8CGsHtAeTH5YFeJ9E",
            quote_mint_address: "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v",
            tick_size: 0.1,
            min_order_size: 0.0001,
            deprecated: false,
        };

        <botvana::market::Market as TryFrom<&'_ super::Market<'_>>>::try_from(&serum_market)
            .unwrap();
    }
}
