pub mod engine;
pub(crate) mod event_loop;

/// Botnode status
#[derive(Clone, PartialEq)]
enum BotnodeStatus {
    /// Connecting to botvana-server
    Connecting,
    /// Connected to botvana-server
    Online,
    /// Not connected to botvana-server
    Offline,
}
