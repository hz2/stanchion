/// All errors produced by this crate.
#[derive(Debug, thiserror::Error)]
pub enum StanchionError {
    #[error("graph has no source node")]
    NoSource,

    #[error("graph has no sink node")]
    NoSink,

    #[error("invalid node id: {0}")]
    InvalidNode(u32),

    #[error("invalid edge id: {0}")]
    InvalidEdge(u32),

    #[error("negative capacity on edge {0}")]
    NegativeCapacity(u32),

    #[error("routing matrix is not substochastic")]
    InvalidRoutingMatrix,

    #[error("unstable network: node {node} has utilization {utilization:.3}")]
    UnstableNetwork { node: u32, utilization: f64 },

    #[error("time horizon {0} would produce a graph too large")]
    TimeHorizonTooLarge(u32),

    #[error("infeasible demand: {demand:.3} exceeds max flow {max_flow:.3}")]
    InfeasibleDemand { demand: f64, max_flow: f64 },

    #[error("disconnected graph: node {0} is unreachable from source")]
    Disconnected(u32),

    #[error("config i/o error: {0}")]
    ConfigIo(String),

    #[error("config parse error: {0}")]
    ConfigParse(String),
}
