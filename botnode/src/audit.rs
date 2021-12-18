//! Audit engine
use crate::prelude::*;

/// Auditing engine
pub struct AuditEngine {}

impl AuditEngine {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait(?Send)]
impl Engine for AuditEngine {
    type Data = ();

    async fn start(self, shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting audit engine");

        run_audit_loop(shutdown).await
    }

    /// Returns dummy data receiver
    fn data_rx(&self) -> ring_channel::RingReceiver<Self::Data> {
        let (_data_tx, data_rx) =
            ring_channel::ring_channel::<()>(NonZeroUsize::new(1024).unwrap());
        data_rx
    }
}

impl ToString for AuditEngine {
    fn to_string(&self) -> String {
        "audit-engine".to_string()
    }
}

/// Audit engine loop
pub async fn run_audit_loop(_shutdown: Shutdown) -> Result<(), EngineError> {
    loop {}
}
