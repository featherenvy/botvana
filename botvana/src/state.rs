use std::sync::Arc;

use async_std::sync::RwLock;

use crate::{market::MarketVec, net::msg::BotId};

/// Global state held by botvana-server
#[derive(Clone, Debug)]
pub struct GlobalState {
    connected_bots: Arc<RwLock<Vec<BotId>>>,
    markets: Arc<RwLock<MarketVec>>,
}

impl GlobalState {
    /// Creates new instance
    pub fn new() -> Self {
        Self {
            connected_bots: Arc::new(RwLock::new(Vec::with_capacity(10))),
            markets: Arc::new(RwLock::new(MarketVec::new())),
        }
    }

    /// Adds online bot
    pub async fn add_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write().await;

        bots.push(bot_id);
    }

    /// Removes a bot (bot is offline)
    pub async fn remove_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write().await;

        bots.retain(|id| *id != bot_id);
    }

    /// Returns set of connected bots
    pub async fn connected_bots(&self) -> Vec<BotId> {
        self.connected_bots.read().await.to_vec()
    }

    /// Returns current known markets
    pub async fn markets(&self) -> MarketVec {
        self.markets.read().await.clone()
    }

    /// Updates markets by either adding new or updating existing
    ///
    /// Markets are matched by exchange and market name
    pub async fn update_markets(&self, markets: MarketVec) {
        let mut marketvec = self.markets.write().await;

        'outer: for market_update in markets.iter() {
            for existing_market in marketvec.iter_mut() {
                if existing_market.exchange == market_update.exchange
                    && existing_market.name == market_update.name
                {
                    *existing_market.size_increment = *market_update.size_increment;
                    *existing_market.price_increment = *market_update.price_increment;
                    continue 'outer;
                }
            }

            marketvec.push(market_update.into());
        }
    }
}

impl Default for GlobalState {
    fn default() -> Self {
        Self {
            markets: Arc::new(RwLock::new(MarketVec::new())),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_bot() {
        let state = GlobalState::new();

        smol::block_on(state.add_bot(BotId(0)));

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 1);

        smol::block_on(state.add_bot(BotId(1)));

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 2);
    }

    #[test]
    fn test_remove_bot() {
        let state = GlobalState::new();

        smol::block_on(async {
            let mut bots = state.connected_bots.write().await;
            bots.push(BotId(0));
            bots.push(BotId(1));
            bots.push(BotId(2));
        });

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 3);

        smol::block_on(state.remove_bot(BotId(1)));

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 2);

        smol::block_on(state.remove_bot(BotId(2)));

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 1);

        smol::block_on(state.remove_bot(BotId(0)));

        assert_eq!(smol::block_on(state.connected_bots.read()).len(), 0);
    }

    #[test]
    fn test_connected_bots() {
        let state = GlobalState::new();

        smol::block_on(async {
            let mut bots = state.connected_bots.write().await;
            bots.push(BotId(0));
            bots.push(BotId(1));
            bots.push(BotId(2));
        });

        assert_eq!(smol::block_on(state.connected_bots()).len(), 3);
    }

    #[test]
    fn test_markets() {
        use super::super::{exchange::*, market::*};

        let state = GlobalState::new();

        let mut markets = MarketVec::new();
        markets.push(Market {
            exchange: ExchangeId::Ftx,
            name: "BTC/USD".to_string(),
            native_symbol: "BTC/USD".to_string(),
            size_increment: 0.00000001,
            price_increment: 0.001,
            r#type: MarketType::Spot(SpotMarket {
                base: "BTC".to_string(),
                quote: "USD".to_string(),
            }),
        });

        smol::block_on(state.update_markets(markets.clone()));

        assert_eq!(smol::block_on(state.markets()).len(), 1);
    }

    #[test]
    fn test_update_markets() {
        use super::super::{exchange::*, market::*};

        let state = GlobalState::new();

        let mut markets = MarketVec::new();
        markets.push(Market {
            exchange: ExchangeId::Ftx,
            name: "BTC/USD".to_string(),
            native_symbol: "BTC/USD".to_string(),
            size_increment: 0.00000001,
            price_increment: 0.001,
            r#type: MarketType::Spot(SpotMarket {
                base: "BTC".to_string(),
                quote: "USD".to_string(),
            }),
        });

        smol::block_on(state.update_markets(markets.clone()));

        assert_eq!(smol::block_on(state.markets.read()).len(), 1);

        smol::block_on(state.update_markets(markets));

        assert_eq!(smol::block_on(state.markets.read()).len(), 1);
    }
}
