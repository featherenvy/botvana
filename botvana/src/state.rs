use std::sync::Arc;

use async_std::sync::RwLock;

use crate::{market::MarketVec, net::msg::BotId};

// Global state held by botvana-server
#[derive(Clone, Debug)]
pub struct GlobalState {
    connected_bots: Arc<RwLock<Vec<BotId>>>,
    markets: Arc<RwLock<MarketVec>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            connected_bots: Arc::new(RwLock::new(Vec::with_capacity(10))),
            markets: Arc::new(RwLock::new(MarketVec::new())),
        }
    }

    pub async fn add_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write().await;

        bots.push(bot_id);
    }

    pub async fn remove_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write().await;

        bots.retain(|id| *id != bot_id);
    }

    pub async fn connected_bots(&self) -> Vec<BotId> {
        self.connected_bots.read().await.to_vec()
    }

    pub async fn markets(&self) -> MarketVec {
        self.markets.read().await.clone()
    }

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
