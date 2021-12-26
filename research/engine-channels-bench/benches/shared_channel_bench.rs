use std::num::NonZeroUsize;
use std::time::{Duration, Instant};

use async_shutdown::Shutdown;
use async_trait::async_trait;
use criterion::*;
use engine_channels_bench::*;
use glommio::channels::shared_channel;
use glommio::channels::shared_channel::*;
use glommio::channels::spsc_queue;
use ring_channel::*;

#[derive(Clone, Debug)]
struct SomeData {
    time: Instant,
    idx: u64,
}

#[derive(Debug)]
struct IngressEngine<Sender> {
    tx: Sender,
    input_rx: spsc_queue::Consumer<u64>,
}

#[derive(Debug)]
struct EgressEngine<Receiver> {
    rx: Receiver,
    result_tx: spsc_queue::Producer<(u64, Duration)>,
}

// ------------------------------------------------------------------------------------------------
// ring_channel implementation

impl<T> IngressEngine<RingSender<T>> {
    fn with_ring_sender(tx: RingSender<T>, input_rx: spsc_queue::Consumer<u64>) -> Self {
        Self { tx, input_rx }
    }
}

#[async_trait(?Send)]
impl StartEngine for IngressEngine<RingSender<SomeData>> {
    async fn start(mut self, _shutdown: Shutdown) -> Result<(), StartEngineError> {
        loop {
            match self.input_rx.try_pop() {
                Some(idx) => {
                    self.tx
                        .send(SomeData {
                            time: Instant::now(),
                            idx,
                        })
                        .unwrap();
                }
                None => {}
            }
        }
    }
}

impl<T> EgressEngine<RingReceiver<T>> {
    fn with_ring_receiver(
        rx: RingReceiver<T>,
        result_tx: spsc_queue::Producer<(u64, Duration)>,
    ) -> Self {
        Self { rx, result_tx }
    }
}

#[async_trait(?Send)]
impl StartEngine for EgressEngine<RingReceiver<SomeData>> {
    async fn start(mut self, _shutdown: Shutdown) -> Result<(), StartEngineError> {
        loop {
            let val = self.rx.recv().unwrap();
            while let Some(_) = self.result_tx.try_push((val.idx, val.time.elapsed())) {}
        }
    }
}

// ------------------------------------------------------------------------------------------------
// glommio::spsc_queue implementation

impl<T> IngressEngine<spsc_queue::Producer<T>> {
    fn with_spsc_queue(tx: spsc_queue::Producer<T>, input_rx: spsc_queue::Consumer<u64>) -> Self {
        Self { tx, input_rx }
    }
}

#[async_trait(?Send)]
impl StartEngine for IngressEngine<spsc_queue::Producer<SomeData>> {
    async fn start(self, shutdown: Shutdown) -> Result<(), StartEngineError> {
        loop {
            if shutdown.shutdown_started() || self.input_rx.producer_disconnected() {
                break Ok(());
            }

            match self.input_rx.try_pop() {
                Some(idx) => {
                    while let Some(_) = self.tx.try_push(SomeData {
                        time: Instant::now(),
                        idx,
                    }) {}
                }
                None => {}
            }
        }
    }
}

impl<T> EgressEngine<spsc_queue::Consumer<T>> {
    fn with_spsc_queue(
        rx: spsc_queue::Consumer<T>,
        result_tx: spsc_queue::Producer<(u64, Duration)>,
    ) -> Self {
        Self { rx, result_tx }
    }
}

#[async_trait(?Send)]
impl StartEngine for EgressEngine<spsc_queue::Consumer<SomeData>> {
    async fn start(self, shutdown: Shutdown) -> Result<(), StartEngineError> {
        loop {
            if shutdown.shutdown_started() {
                break Ok(());
            }

            match self.rx.try_pop() {
                Some(val) => {
                    while let Some(_) = self.result_tx.try_push((val.idx, val.time.elapsed())) {}
                }
                None => {}
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// glommio::shared_channel implementation

impl<T: Send> IngressEngine<SharedSender<T>> {
    fn new(tx: SharedSender<T>, input_rx: spsc_queue::Consumer<u64>) -> Self {
        Self { tx, input_rx }
    }
}

#[async_trait(?Send)]
impl StartEngine for IngressEngine<SharedSender<SomeData>> {
    async fn start(self, shutdown: Shutdown) -> Result<(), StartEngineError> {
        let tx = self.tx.connect().await;

        loop {
            if shutdown.shutdown_started() || self.input_rx.producer_disconnected() {
                break Ok(());
            }

            match self.input_rx.try_pop() {
                Some(idx) => {
                    tx.send(SomeData {
                        time: Instant::now(),
                        idx,
                    })
                    .await
                    .unwrap();
                }
                None => {}
            }
        }
    }
}

impl<T: Send> EgressEngine<SharedReceiver<T>> {
    fn new(rx: SharedReceiver<T>, result_tx: spsc_queue::Producer<(u64, Duration)>) -> Self {
        Self { rx, result_tx }
    }
}

#[async_trait(?Send)]
impl StartEngine for EgressEngine<SharedReceiver<SomeData>> {
    async fn start(self, shutdown: Shutdown) -> Result<(), StartEngineError> {
        let rx = self.rx.connect().await;

        loop {
            if shutdown.shutdown_started() {
                break Ok(());
            }

            match rx.recv().await {
                Some(val) => {
                    while let Some(_) = self.result_tx.try_push((val.idx, val.time.elapsed())) {}
                }
                None => {}
            }
        }
    }
}

// ------------------------------------------------------------------------------------------------
// benchmark

pub fn channel_bench(c: &mut Criterion) {
    let mut g = c.benchmark_group("engine_channels");

    g.bench_function("spsc_queue", |b| {
        let shutdown = Shutdown::new();
        let (input_tx, input_rx) = spsc_queue::make(100);
        let (result_tx, result_rx) = spsc_queue::make(100);
        let (tx, rx) = spsc_queue::make(10);
        let ingress_engine = IngressEngine::with_spsc_queue(tx, input_rx);
        let egress_engine = EgressEngine::with_spsc_queue(rx, result_tx);

        start_engine(0, ingress_engine, shutdown.clone()).unwrap();
        start_engine(2, egress_engine, shutdown.clone()).unwrap();

        b.iter_custom(|iters| {
            (0..iters)
                .map(|i| {
                    while let Some(_) = input_tx.try_push(i) {}

                    loop {
                        match result_rx.try_pop() {
                            Some(dur) => {
                                break dur.1;
                            }
                            _ => {}
                        }
                    }
                })
                .sum()
        });

        shutdown.shutdown();
    });

    g.bench_function("shared_channel", |b| {
        let shutdown = Shutdown::new();
        let (input_tx, input_rx) = spsc_queue::make(100);
        let (result_tx, result_rx) = spsc_queue::make(100);
        let (tx, rx) = shared_channel::new_bounded(10);
        let ingress_engine = IngressEngine::new(tx, input_rx);
        let egress_engine = EgressEngine::new(rx, result_tx);

        start_engine(0, ingress_engine, shutdown.clone()).unwrap();
        start_engine(2, egress_engine, shutdown.clone()).unwrap();

        b.iter_custom(|iters| {
            (0..iters)
                .map(|i| {
                    while let Some(_) = input_tx.try_push(i) {}

                    loop {
                        match result_rx.try_pop() {
                            Some(dur) => {
                                break dur.1;
                            }
                            _ => {}
                        }
                    }
                })
                .sum()
        });

        shutdown.shutdown();
    });

    g.bench_function("ring_channel", |b| {
        let shutdown = Shutdown::new();
        let (input_tx, input_rx) = spsc_queue::make(100);
        let (result_tx, result_rx) = spsc_queue::make(100);
        let (tx, rx) = ring_channel(NonZeroUsize::new(10).unwrap());
        let ingress_engine = IngressEngine::with_ring_sender(tx, input_rx);
        let egress_engine = EgressEngine::with_ring_receiver(rx, result_tx);

        start_engine(0, ingress_engine, shutdown.clone()).unwrap();
        start_engine(2, egress_engine, shutdown.clone()).unwrap();

        b.iter_custom(|iters| {
            (0..iters)
                .map(|i| {
                    while let Some(_) = input_tx.try_push(i) {}

                    loop {
                        match result_rx.try_pop() {
                            Some(dur) => {
                                break dur.1;
                            }
                            _ => {}
                        }
                    }
                })
                .sum()
        });

        shutdown.shutdown();
    });
}

criterion_group!ebenches, channel_bench);
criterion_main!(benches);
