use std::hash::Hash;

use crate::prelude::*;

const FAIL_LIMIT: usize = 100;

/// Array of producers for inter-engine channel
#[derive(Debug)]
pub struct ProducersArray<T, const N: usize>(pub(super) ArrayVec<spsc_queue::Producer<T>, N>);

impl<T, const N: usize> ProducersArray<T, N>
where
    T: Clone + std::fmt::Debug,
{
    /// Pushes value onto all data transmitters
    ///
    /// Returns `Ok` only when the value was pushed onto all producers.
    pub(crate) fn push_value(&self, event: T) -> Result<(), PushValueError<N>> {
        let mut err = PushValueError::<N>::default();

        self.0.iter().enumerate().for_each(|(idx, tx)| {
            if tx.consumer_disconnected() {
                error!("Producer {idx} disconnected");
                err.push_failed(idx);
                return;
            }

            let val = event.clone();
            let mut fail_cnt = 0;
            let mut res = tx.try_push(val);

            while let Some(value) = res {
                res = tx.try_push(value);
                warn!("Retried to push to channel {idx}: {res:?}");

                if fail_cnt > FAIL_LIMIT {
                    if tx.consumer_disconnected() {
                        warn!("Producer {idx} disconnected.");
                    }
                    warn!("Failed to push value onto producer {idx} {:?}", tx);
                    err.push_failed(idx);
                    return;
                }
                fail_cnt += 1;
            }
        });

        if err.len() > 0 {
            Err(err)
        } else {
            Ok(())
        }
    }
}

#[derive(Debug, Default, thiserror::Error)]
#[error("Failed to push value to SPSC queue")]
pub struct PushValueError<const N: usize> {
    failed_consumers: ArrayVec<usize, N>,
}

impl<const N: usize> PushValueError<N> {
    fn push_failed(&mut self, idx: usize) {
        self.failed_consumers.push(idx);
    }

    fn len(&self) -> usize {
        self.failed_consumers.len()
    }
}

impl<T, const N: usize> Default for ProducersArray<T, N> {
    fn default() -> Self {
        Self(ArrayVec::<_, N>::new())
    }
}

/// Map of consumers for inter-engine channel
#[derive(Debug)]
pub struct ConsumersMap<K, V>(HashMap<K, spsc_queue::Consumer<V>>);

impl<K, V> ConsumersMap<K, V>
where
    K: Hash + Eq,
{
    pub fn new(inner: HashMap<K, spsc_queue::Consumer<V>>) -> Self {
        Self(inner)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self(HashMap::with_capacity(capacity))
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<K, spsc_queue::Consumer<V>> {
        self.0.iter()
    }

    pub fn insert(
        &mut self,
        key: K,
        value: spsc_queue::Consumer<V>,
    ) -> Option<spsc_queue::Consumer<V>> {
        self.0.insert(key, value)
    }

    pub fn poll_values(&self) -> Option<(&K, V)> {
        for (k, rx) in self.iter() {
            if let Some(v) = rx.try_pop() {
                return Some((k, v));
            }
        }

        None
    }
}

impl<K, V> Default for ConsumersMap<K, V> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_producers_array_default() {
        let producers = ProducersArray::<(), 1>::default();
        assert_eq!(producers.0.len(), 0);
    }

    #[test]
    fn test_producers_push_value() {
        let (tx, rx) = spsc_queue::make(1);
        let mut producers = ProducersArray::<(), 1>::default();
        producers.0.push(tx);

        producers.push_value(()).unwrap();

        assert_eq!(1, producers.0.len());
        assert_eq!(Some(()), rx.try_pop());
    }

    #[test]
    fn test_producers_push_value_queue_full() {
        let (tx, rx) = spsc_queue::make(1);
        drop(rx);

        let mut producers = ProducersArray::<(), 1>::default();
        producers.0.push(tx);

        producers.push_value(()).unwrap();
        assert!(producers.push_value(()).is_err());
    }

    #[test]
    fn test_consumers_map_default() {
        let consumers = ConsumersMap::<(), ()>::default();
        assert_eq!(consumers.0.len(), 0);
    }

    #[test]
    fn test_consumers_poll_values() {
        let mut consumers = ConsumersMap::<&'static str, ()>::default();
        let (tx, rx) = spsc_queue::make(1);
        consumers.insert("test", rx);
        tx.try_push(());

        let (k, v) = consumers.poll_values().unwrap();

        assert_eq!(&"test", k);
        assert_eq!((), v);
    }
}
