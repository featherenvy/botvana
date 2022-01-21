use std::cell::RefCell;

use metered::{clear::Clear, time_source::StdInstant, *};

use crate::prelude::*;

/// Auditing engine
#[derive(Debug)]
pub struct AuditEngine {
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    status_tx: spsc_queue::Producer<EngineStatus>,
    status_rx: spsc_queue::Consumer<EngineStatus>,
    metrics: AuditMetrics,
}

#[derive(Default, Debug)]
pub struct AuditMetrics {
    throughput: Throughput<StdInstant, RefCell<metered::common::TxPerSec>>,
}

impl AuditEngine {
    pub fn new(market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>) -> Self {
        let (status_tx, status_rx) = spsc_queue::make(1);
        let metrics = AuditMetrics::default();

        Self {
            market_data_rxs,
            status_tx,
            status_rx,
            metrics,
        }
    }
}

#[async_trait(?Send)]
impl Engine for AuditEngine {
    fn name(&self) -> String {
        "audit-engine".to_string()
    }

    fn status_rx(&self) -> spsc_queue::Consumer<EngineStatus> {
        self.status_rx.clone()
    }

    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting audit engine");

        self.status_tx.try_push(EngineStatus::Booting);

        run_audit_loop(self.status_tx, self.market_data_rxs, self.metrics, shutdown).await
    }
}

/// Audit engine loop
pub async fn run_audit_loop(
    status_tx: spsc_queue::Producer<EngineStatus>,
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
    audit_metrics: AuditMetrics,
    shutdown: Shutdown,
) -> Result<(), EngineError> {
    status_tx.try_push(EngineStatus::Running);

    info!("{:?}", market_data_rxs);

    let throughput = &audit_metrics.throughput;
    let mut start = std::time::Instant::now();

    loop {
        measure!(throughput, {
            for (_, market_data_rx) in market_data_rxs.iter() {
                if let Some(event) = market_data_rx.try_pop() {
                    let elapsed = event.timestamp.elapsed().unwrap();
                    trace!("market_event = {:?}", event);

                    if elapsed > std::time::Duration::from_millis(1) {
                        warn!("Late market event!");
                    }
                }
            }
        });

        if start.elapsed().as_secs() >= 5 {
            if shutdown.shutdown_started() {
                info!("shutting down audit engine");

                status_tx.try_push(EngineStatus::ShuttingDown);

                return Ok(());
            }

            start = std::time::Instant::now();
            info!(
                "max throughput over last 5s = {:?}",
                throughput.0.borrow().hdr_histogram.mean()
            );
            throughput.clear();
        }
    }
}
