use std::{collections::HashMap, sync::Arc};

use parking_lot::RwLock;

use crate::{
    exchange::*,
    market::{orderbook::*, MarketVec},
    net::msg::BotId,
};

const SYMBOL_TABLE_CAP: u32 = 1024;

/// Global state held by botvana-server
#[derive(Clone, Debug)]
pub struct GlobalState {
    connected_bots: Arc<RwLock<Vec<BotId>>>,
    markets: Arc<RwLock<MarketVec>>,
    symbol_table: Arc<RwLock<MarketSymbolTable>>,
    orderbooks: Arc<RwLock<HashMap<(ExchangeId, u32), PlainOrderbook<f64>>>>,
}

impl GlobalState {
    /// Creates new instance
    pub fn new() -> Self {
        Self {
            connected_bots: Arc::new(RwLock::new(Vec::with_capacity(10))),
            markets: Arc::new(RwLock::new(MarketVec::new())),
            symbol_table: Arc::new(RwLock::new(MarketSymbolTable::with_capacity(
                SYMBOL_TABLE_CAP,
            ))),
            orderbooks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Adds online bot
    pub fn add_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write();

        bots.push(bot_id);
    }

    /// Removes a bot (bot is offline)
    pub fn remove_bot(&self, bot_id: BotId) {
        let mut bots = self.connected_bots.write();

        bots.retain(|id| *id != bot_id);
    }

    /// Returns set of connected bots
    pub fn connected_bots(&self) -> Vec<BotId> {
        self.connected_bots.read().to_vec()
    }

    /// Returns current known markets
    pub fn markets(&self) -> MarketVec {
        self.markets.read().clone()
    }

    /// Updates markets by either adding new or updating existing
    ///
    /// Markets are matched by exchange and market name
    pub fn update_markets(&self, markets: MarketVec) {
        let mut marketvec = self.markets.write();

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

    /// Update orderbook of a given exchange market
    pub fn update_orderbook(
        &self,
        exchange: ExchangeId,
        market: &str,
        orderbook: PlainOrderbook<f64>,
    ) {
        let mut symbol_table = self.symbol_table.write();
        let symbol = symbol_table.get(market);

        let mut orderbooks = self.orderbooks.write();
        orderbooks.insert((exchange, symbol), orderbook);
    }

    pub fn get_orderbook(&self, exchange: ExchangeId, market: &str) -> Option<PlainOrderbook<f64>> {
        let mut symbol_table = self.symbol_table.write();
        let symbol = symbol_table.get(market);
        drop(symbol_table);

        let orderbooks = self.orderbooks.read();
        orderbooks.get(&(exchange, symbol)).cloned()
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

/// Interning table for market symbols
#[derive(Debug, Default)]
pub struct MarketSymbolTable {
    table: Vec<Box<str>>,
}

impl MarketSymbolTable {
    fn with_capacity(cap: u32) -> Self {
        Self {
            table: Vec::with_capacity(cap as usize),
        }
    }

    /// # Examples
    ///
    /// ```
    /// # use botvana::state::MarketSymbolTable;
    /// let mut symbol_table = MarketSymbolTable::default();
    ///
    /// let s1 = symbol_table.get("BTC/USD");
    /// assert_eq!(0, s1);
    ///
    /// let s2 = symbol_table.get("BTC/USD");
    /// assert_eq!(s1, s2);
    ///
    /// let s3 = symbol_table.get("btc/usd");
    /// assert_eq!(s1, s3);
    ///
    /// let s4 = symbol_table.get("BTC/EUR");
    /// assert_ne!(s1, s4);
    /// assert_ne!(s2, s4);
    /// assert_ne!(s3, s4);
    /// ```
    pub fn get(&mut self, symbol: &str) -> u32 {
        let symbol = symbol.to_ascii_lowercase();
        match self
            .table
            .iter()
            .enumerate()
            .find(|(_idx, existing)| symbol == existing.as_ref())
        {
            Some((idx, _)) => idx as u32,
            None => {
                let len = self.table.len();
                self.table.push(Box::from(symbol));
                len as u32
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::{exchange::*, market::*};
    use super::*;

    #[test]
    fn market_symbol_table_different_case() {
        let mut symbol_table = MarketSymbolTable::default();
        let s1 = symbol_table.get("BTC/USD");
        let s2 = symbol_table.get("btc/usd");
        let s3 = symbol_table.get("Btc/Usd");

        assert_eq!(s1, s2);
        assert_eq!(s2, s3);
        assert_eq!(s1, s3);
    }

    #[test]
    fn test_add_bot() {
        let state = GlobalState::new();

        state.add_bot(BotId(0));

        assert_eq!(state.connected_bots.read().len(), 1);

        state.add_bot(BotId(1));

        assert_eq!(state.connected_bots.read().len(), 2);
    }

    #[test]
    fn test_remove_bot() {
        let state = GlobalState::new();

        let mut bots = state.connected_bots.write();
        bots.push(BotId(0));
        bots.push(BotId(1));
        bots.push(BotId(2));
        drop(bots);

        assert_eq!(state.connected_bots.read().len(), 3);

        state.remove_bot(BotId(1));

        assert_eq!(state.connected_bots.read().len(), 2);

        state.remove_bot(BotId(2));

        assert_eq!(state.connected_bots.read().len(), 1);

        state.remove_bot(BotId(0));

        assert_eq!(state.connected_bots.read().len(), 0);
    }

    #[test]
    fn test_connected_bots() {
        let state = GlobalState::new();

        let mut bots = state.connected_bots.write();
        bots.push(BotId(0));
        bots.push(BotId(1));
        bots.push(BotId(2));
        drop(bots);

        assert_eq!(state.connected_bots().len(), 3);
    }

    #[test]
    fn test_markets() {
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

        state.update_markets(markets.clone());

        assert_eq!(state.markets().len(), 1);
    }

    #[test]
    fn test_update_markets() {
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

        state.update_markets(markets.clone());

        assert_eq!(state.markets.read().len(), 1);

        state.update_markets(markets);

        assert_eq!(state.markets.read().len(), 1);
    }

    #[test]
    fn test_update_orderbook() {
        let state = GlobalState::new();
        let market = "BTC/USD";
        let mut orderbook = PlainOrderbook::<f64>::new();

        state.update_orderbook(ExchangeId::Ftx, market, orderbook.clone());

        assert_eq!(state.orderbooks.read().len(), 1);

        let price_levels = PriceLevelsVec::<f64>::from_tuples_vec_unsorted(&mut [
            (1000.0, 0.3),
            (1250.0, 0.4),
            (1400.0, 0.25),
        ]);
        orderbook.bids.update(&price_levels);

        state.update_orderbook(ExchangeId::Ftx, market, orderbook.clone());

        assert_eq!(state.orderbooks.read().len(), 1);
    }

    #[test]
    fn test_get_orderbook() {
        let market = "BTC/USD";
        let mut orderbooks = HashMap::new();
        let orderbook = PlainOrderbook::<f64>::new();
        orderbooks.insert((ExchangeId::Ftx, 0 as u32), orderbook.clone());
        let state = GlobalState {
            orderbooks: Arc::new(RwLock::new(orderbooks)),
            ..GlobalState::default()
        };

        state.get_orderbook(ExchangeId::Ftx, market).unwrap();
    }

    #[test]
    fn test_get_orderbook_unknown() {
        let market = "BTC/USD";
        let state = GlobalState::new();

        assert!(state.get_orderbook(ExchangeId::Ftx, market).is_none());
    }
}
