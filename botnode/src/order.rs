use crate::prelude::*;

/// Control engine for Botnode
///
/// The control engine maintains the connection to Botvana server.
pub struct OrderEngine {}

impl OrderEngine {}

#[async_trait(?Send)]
impl Engine for OrderEngine {
    const NAME: &'static str = "order-engine";

    type Data = ();

    async fn start(self, _shutdown: Shutdown) -> Result<(), EngineError> {
        info!("Starting order engine");

        Ok(())
    }
}
