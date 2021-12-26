//! Orderbook

use rust_decimal::prelude::*;

/// Trait representing an orderbook API
pub trait UpdateOrderbook<T> {
    fn update(&mut self, bids: &PriceLevelsVec<T>, asks: &PriceLevelsVec<T>);
}

/// Plain orderbook with bids and asks
#[derive(Clone, Debug)]
pub struct PlainOrderbook<T> {
    pub bids: PriceLevelsVec<T>,
    pub asks: PriceLevelsVec<T>,
}

impl<T> PlainOrderbook<T> {
    pub fn new() -> Self {
        Self {
            bids: PriceLevelsVec::new(),
            asks: PriceLevelsVec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        Self {
            bids: PriceLevelsVec::with_capacity(cap),
            asks: PriceLevelsVec::with_capacity(cap),
        }
    }
}

impl UpdateOrderbook<f64> for PlainOrderbook<f64> {
    fn update(&mut self, bids: &PriceLevelsVec<f64>, asks: &PriceLevelsVec<f64>) {
        self.bids.update(bids);
        self.asks.update(asks);
    }
}

impl UpdateOrderbook<Decimal> for PlainOrderbook<Decimal> {
    fn update(&mut self, bids: &PriceLevelsVec<Decimal>, asks: &PriceLevelsVec<Decimal>) {
        self.bids.update(bids);
        self.asks.update(asks);
    }
}

/// Columnar struct of price levels
#[derive(Clone, Debug)]
pub struct PriceLevelsVec<T> {
    pub price_vec: Vec<T>,
    pub size_vec: Vec<T>,
}

impl<T> PriceLevelsVec<T> {
    pub fn new() -> Self {
        PriceLevelsVec {
            price_vec: Vec::new(),
            size_vec: Vec::new(),
        }
    }

    pub fn with_capacity(cap: usize) -> Self {
        PriceLevelsVec {
            price_vec: Vec::with_capacity(cap),
            size_vec: Vec::with_capacity(cap),
        }
    }

    /// Returns new `PriceLevelsVec` built from given Vec of price and size tuple
    pub fn from_tuples_vec(data: Vec<(T, T)>) -> Self {
        let mut price_vec = Vec::with_capacity(data.len());
        let mut size_vec = Vec::with_capacity(data.len());

        data.into_iter().for_each(|(price, size)| {
            price_vec.push(price);
            size_vec.push(size);
        });

        PriceLevelsVec {
            price_vec,
            size_vec,
        }
    }

    pub fn len(&self) -> usize {
        self.price_vec.len()
    }
}

impl PriceLevelsVec<Decimal> {
    pub fn update(&mut self, update: &PriceLevelsVec<Decimal>) {
        update
            .price_vec
            .iter()
            .zip(update.size_vec.iter())
            .for_each(|(price, new_size)| {
                match self
                    .price_vec
                    .binary_search_by(|v| v.partial_cmp(&price).unwrap())
                {
                    Ok(pos) => {
                        if *new_size == Decimal::ZERO {
                            self.price_vec.remove(pos);
                            self.size_vec.remove(pos);
                        } else {
                            let old_size = self.size_vec.get_mut(pos).unwrap();
                            *old_size = *new_size;
                        }
                    }
                    Err(pos) => {
                        self.price_vec.insert(pos, *price);
                        self.size_vec.insert(pos, *new_size);
                    }
                }
            });
    }
}

impl PriceLevelsVec<f64> {
    pub fn update(&mut self, update: &PriceLevelsVec<f64>) {
        update
            .price_vec
            .iter()
            .zip(update.size_vec.iter())
            .for_each(|(price, new_size)| {
                match self
                    .price_vec
                    .binary_search_by(|v| v.partial_cmp(&price).unwrap())
                {
                    Ok(pos) => {
                        if *new_size == 0.0 {
                            self.price_vec.remove(pos);
                            self.size_vec.remove(pos);
                        } else {
                            let old_size = self.size_vec.get_mut(pos).unwrap();
                            *old_size = *new_size;
                        }
                    }
                    Err(pos) => {
                        self.price_vec.insert(pos, *price);
                        self.size_vec.insert(pos, *new_size);
                    }
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_update_price_levels_vec() {
        let mut price_levels = PriceLevelsVec::<f64>::new();
        price_levels.update(&PriceLevelsVec::new());
    }

    #[test]
    fn test_update_empty_price_levels_vec() {
        let mut price_levels = PriceLevelsVec::<f64>::new();
        price_levels.update(&PriceLevelsVec::from_tuples_vec(vec![
            (1000.0, 0.3),
            (1250.0, 0.4),
            (1400.0, 0.25),
        ]));

        assert_eq!(price_levels.price_vec[0], 1000.0);
        assert_eq!(price_levels.price_vec[1], 1250.0);
        assert_eq!(price_levels.price_vec[2], 1400.0);
    }

    #[test]
    fn test_update_that_removes_all_price_levels_vec() {
        let mut price_levels = PriceLevelsVec {
            price_vec: vec![13.0, 13.05, 13.1],
            size_vec: vec![120.0, 90.0, 20.0],
        };
        price_levels.update(&PriceLevelsVec::from_tuples_vec(vec![
            (13.0, 0.0),
            (13.05, 0.0),
            (13.1, 0.0),
        ]));

        assert_eq!(price_levels.price_vec.len(), 0);
        assert_eq!(price_levels.size_vec.len(), 0);
    }

    #[test]
    fn test_update_price_levels_vec() {
        let mut price_levels = PriceLevelsVec {
            price_vec: vec![13.0, 13.05, 13.1],
            size_vec: vec![120.0, 90.0, 20.0],
        };
        price_levels.update(&PriceLevelsVec::from_tuples_vec(vec![
            (13.0, 0.0),
            (13.02, 120.0),
            (13.01, 270.0),
        ]));

        assert_eq!(price_levels.price_vec[0], 13.01);
        assert_eq!(price_levels.size_vec[0], 270.0);
        assert_eq!(price_levels.price_vec[1], 13.02);
        assert_eq!(price_levels.size_vec[1], 120.0);
        assert_eq!(price_levels.price_vec[2], 13.05);
        assert_eq!(price_levels.size_vec[2], 90.0);
        assert_eq!(price_levels.price_vec[3], 13.1);
        assert_eq!(price_levels.size_vec[3], 20.0);
    }
}
