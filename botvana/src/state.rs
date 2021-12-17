use std::sync::Arc;

use async_std::sync::RwLock;

use crate::net::msg::BotId;

// Global state held by botvana-server
#[derive(Clone, Debug)]
pub struct GlobalState {
    connected_bots: Arc<RwLock<Vec<BotId>>>,
}

impl GlobalState {
    pub fn new() -> Self {
        Self {
            connected_bots: Arc::new(RwLock::new(Vec::with_capacity(1000))),
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
}
