//! Orderbook

/// Trait representing an orderbook API
pub trait Orderbook<T> {
    fn update(&mut self, bids: PriceLevelsVec<T>, asks: PriceLevelsVec<T>);
}

/// Plain orderbook with bids and asks
#[derive(Clone, Debug)]
pub struct PlainOrderbook<T> {
    bids: PriceLevelsVec<T>,
    asks: PriceLevelsVec<T>,
}

impl<T> PlainOrderbook<T> {
    pub fn new() -> Self {
        Self {
            bids: PriceLevelsVec::new(),
            asks: PriceLevelsVec::new(),
        }
    }
}

impl Orderbook<f64> for PlainOrderbook<f64> {
    fn update(&mut self, bids: PriceLevelsVec<f64>, asks: PriceLevelsVec<f64>) {
        self.bids.update(bids);
        self.asks.update(asks);
    }
}

/// Columnar struct of price levels
#[derive(Clone, Debug)]
pub struct PriceLevelsVec<T> {
    price: Vec<T>,
    size: Vec<T>,
}

impl<T> PriceLevelsVec<T> {
    pub fn new() -> Self {
        PriceLevelsVec {
            price: Vec::new(),
            size: Vec::new(),
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
            price: price_vec,
            size: size_vec,
        }
    }
}

impl PriceLevelsVec<f64> {
    pub fn update(&mut self, update: PriceLevelsVec<f64>) {
        update.price.iter().enumerate().for_each(|(i, price)| {
            let new_size = *update.size.get(i).unwrap();

            match self
                .price
                .binary_search_by(|v| v.partial_cmp(&price).unwrap())
            {
                Ok(pos) => {
                    if new_size == 0.0 {
                        self.price.remove(pos);
                        self.size.remove(pos);
                    } else {
                        let old_size = self.size.get_mut(pos).unwrap();
                        *old_size = new_size;
                    }
                }
                Err(pos) => {
                    self.price.insert(pos, *price);
                    self.size.insert(pos, new_size);
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
        price_levels.update(PriceLevelsVec::new());
    }

    #[test]
    fn test_update_empty_price_levels_vec() {
        let mut price_levels = PriceLevelsVec::<f64>::new();
        price_levels.update(PriceLevelsVec::from_tuples_vec(vec![
            (1000.0, 0.3),
            (1250.0, 0.4),
            (1400.0, 0.25),
        ]));

        assert_eq!(price_levels.price[0], 1000.0);
        assert_eq!(price_levels.price[1], 1250.0);
        assert_eq!(price_levels.price[2], 1400.0);
    }

    #[test]
    fn test_update_that_removes_all_price_levels_vec() {
        let mut price_levels = PriceLevelsVec {
            price: vec![13.0, 13.05, 13.1],
            size: vec![120.0, 90.0, 20.0],
        };
        price_levels.update(PriceLevelsVec::from_tuples_vec(vec![
            (13.0, 0.0),
            (13.05, 0.0),
            (13.1, 0.0),
        ]));

        assert_eq!(price_levels.price.len(), 0);
        assert_eq!(price_levels.size.len(), 0);
    }

    #[test]
    fn test_update_price_levels_vec() {
        let mut price_levels = PriceLevelsVec {
            price: vec![13.0, 13.05, 13.1],
            size: vec![120.0, 90.0, 20.0],
        };
        price_levels.update(PriceLevelsVec::from_tuples_vec(vec![
            (13.0, 0.0),
            (13.02, 120.0),
            (13.01, 270.0),
        ]));

        assert_eq!(price_levels.price[0], 13.01);
        assert_eq!(price_levels.size[0], 270.0);
        assert_eq!(price_levels.price[1], 13.02);
        assert_eq!(price_levels.size[1], 120.0);
        assert_eq!(price_levels.price[2], 13.05);
        assert_eq!(price_levels.size[2], 90.0);
        assert_eq!(price_levels.price[3], 13.1);
        assert_eq!(price_levels.size[3], 20.0);
    }
}
