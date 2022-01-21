use crate::prelude::*;

const FAIL_LIMIT: usize = 100;

/// Array of producers for inter-engine channel
#[derive(Debug)]
pub struct ProducersArray<T, const N: usize>(pub(super) ArrayVec<spsc_queue::Producer<T>, N>);

impl<T, const N: usize> Default for ProducersArray<T, N> {
    fn default() -> Self {
        Self(ArrayVec::<_, N>::new())
    }
}

impl<T, const N: usize> ProducersArray<T, N>
where
    T: Clone + std::fmt::Debug,
{
    /// Pushes value onto all data transmitters
    pub(crate) fn push_value(&self, event: T) {
        self.0.iter().enumerate().for_each(|(idx, tx)| {
            if tx.consumer_disconnected() {
                error!("Producer {idx} disconnected");
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
                    panic!("Failed to push value onto producer {idx} {:?}", tx);
                }
                fail_cnt += 1;
            }
        });
    }
}

/// Map of consumers for inter-engine channel
pub struct ConsumersMap<K, V>(HashMap<K, spsc_queue::Consumer<V>>);

impl<K, V> ConsumersMap<K, V> {
    pub fn new(inner: HashMap<K, spsc_queue::Consumer<V>>) -> Self {
        Self(inner)
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<K, spsc_queue::Consumer<V>> {
        self.0.iter()
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
