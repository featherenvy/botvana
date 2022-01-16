//! Audit engine
use crate::prelude::*;

/// Auditing engine
#[derive(Default)]
pub struct AuditEngine {
    market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>,
}

impl AuditEngine {
    pub fn new(market_data_rxs: HashMap<Box<str>, spsc_queue::Consumer<MarketEvent>>) -> Self {
        Self { market_data_rxs }
    }
}

#[async_trait(?Send)]
impl Engine for AuditEngine {
    type Data = ();

    fn name(&self) -> String {
        "audit-engine".to_string()
    }

    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting audit engine");

        run_audit_loop(shutdown).await
    }

    /// Returns dummy data receiver
    fn data_rx(&mut self) -> spsc_queue::Consumer<Self::Data> {
        let (_data_tx, data_rx) = spsc_queue::make::<()>(1024);
        data_rx
    }
}

/// Audit engine loop
pub async fn run_audit_loop(shutdown: Shutdown) -> Result<(), EngineError> {
    // noop for now
    loop {
        if shutdown.shutdown_started() {
            return Ok(());
        }

        glommio::timer::sleep(Duration::from_secs(10)).await;
    }
}
