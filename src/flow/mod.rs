pub mod dinic;
pub mod min_cost;
pub mod push_relabel;
pub mod scaling;

use crate::{error::StanchionError, graph::{DiGraph, NodeId}};

/// Result of a maximum-flow computation.
#[derive(Debug, Clone)]
pub struct FlowResult {
    pub max_flow:     f64,
    /// Flow on each raw edge index (parallel to the graph's internal edge list).
    pub flow_on_edge: Vec<f64>,
}

/// Result of a minimum-cost flow computation.
#[derive(Debug, Clone)]
pub struct MinCostFlowResult {
    pub max_flow:     f64,
    pub min_cost:     f64,
    pub flow_on_edge: Vec<f64>,
}

/// Implemented by every max-flow algorithm.
pub trait MaxFlowSolver {
    fn max_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
    ) -> Result<FlowResult, StanchionError>;
}

/// Implemented by every min-cost flow algorithm.
pub trait MinCostFlowSolver {
    /// Find min-cost flow.  If `demand` is `None`, maximise flow subject to cost.
    fn min_cost_flow(
        &self,
        graph:   &DiGraph,
        source:  NodeId,
        sink:    NodeId,
        demand:  Option<f64>,
    ) -> Result<MinCostFlowResult, StanchionError>;
}
