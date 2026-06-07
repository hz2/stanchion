use crate::graph::{Capacity, Cost, EdgeId, NodeId};

/// A recommended stanchion layout change with its justification.
#[derive(Debug, Clone, PartialEq)]
pub enum StanchionDecision {
    /// Connect two posts with a new belt.
    AddEdge {
        from:     NodeId,
        to:       NodeId,
        capacity: Capacity,
        cost:     Cost,
        reason:   DecisionReason,
    },
    /// Remove an existing belt.
    RemoveEdge {
        edge_id: EdgeId,
        reason:  DecisionReason,
    },
}

/// Why a decision was recommended.
#[derive(Debug, Clone, PartialEq)]
pub enum DecisionReason {
    /// This edge is in the min-cut; adding capacity here raises max-flow by `delta_flow`.
    BottleneckExpansion { delta_flow: f64 },
    /// This edge carries zero flow in the current max-flow solution and can be pruned.
    ZeroFlowPruning,
    /// Node utilisation exceeds the configured threshold; routing relief is needed.
    UtilizationRelief { node: NodeId, utilization: f64 },
}
