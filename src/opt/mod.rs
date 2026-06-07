pub mod decision;
pub mod min_cut;

pub use decision::{DecisionReason, StanchionDecision};
pub use min_cut::find_min_cut;

use crate::{
    error::StanchionError,
    flow::{FlowResult, MaxFlowSolver},
    graph::{Capacity, Cost, DiGraph, NodeId},
};

/// Analyses a stanchion graph and recommends layout changes.
///
/// Uses a max-flow solve to identify the min-cut (bottleneck arcs) and
/// zero-flow arcs, then produces `StanchionDecision` values sorted by
/// estimated impact.
pub struct OptimizationEngine<F: MaxFlowSolver> {
    solver:                  F,
    /// Suggested capacity for a newly added belt.
    pub default_new_capacity: f64,
    /// Suggested cost (transit time) for a newly added belt.
    pub default_new_cost:     f64,
}

impl<F: MaxFlowSolver> OptimizationEngine<F> {
    pub fn new(solver: F) -> Self {
        Self {
            solver,
            default_new_capacity: 1.0,
            default_new_cost:     1.0,
        }
    }

    /// Return recommended decisions sorted by impact (highest flow gain first).
    pub fn recommend(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
    ) -> Result<Vec<StanchionDecision>, StanchionError> {
        let result  = self.solver.max_flow(graph, source, sink)?;
        let mut decisions = Vec::new();

        self.bottleneck_expansions(graph, &result, source, &mut decisions);
        self.zero_flow_prunings(graph, &result, &mut decisions);

        // Sort by descending delta_flow impact.
        decisions.sort_by(|a, b| impact(b).partial_cmp(&impact(a)).unwrap_or(std::cmp::Ordering::Equal));
        Ok(decisions)
    }

    fn bottleneck_expansions(
        &self,
        graph:     &DiGraph,
        result:    &FlowResult,
        source:    NodeId,
        decisions: &mut Vec<StanchionDecision>,
    ) {
        let cut_edges = find_min_cut(graph, result, source);
        for eid in cut_edges {
            let e = match graph.edge(eid) {
                Some(e) => e,
                None    => continue,
            };
            decisions.push(StanchionDecision::AddEdge {
                from:     e.from,
                to:       e.to,
                capacity: Capacity(self.default_new_capacity),
                cost:     Cost(self.default_new_cost),
                reason:   DecisionReason::BottleneckExpansion {
                    delta_flow: self.default_new_capacity,
                },
            });
        }
    }

    fn zero_flow_prunings(
        &self,
        graph:     &DiGraph,
        result:    &FlowResult,
        decisions: &mut Vec<StanchionDecision>,
    ) {
        for idx in (0..graph.edges.len()).step_by(2) {
            let flow = result.flow_on_edge.get(idx).copied().unwrap_or(0.0);
            if flow.abs() < 1e-12 {
                decisions.push(StanchionDecision::RemoveEdge {
                    edge_id: crate::graph::EdgeId(idx as u32),
                    reason:  DecisionReason::ZeroFlowPruning,
                });
            }
        }
    }
}

fn impact(d: &StanchionDecision) -> f64 {
    match d {
        StanchionDecision::AddEdge { reason: DecisionReason::BottleneckExpansion { delta_flow }, .. } => *delta_flow,
        StanchionDecision::AddEdge { .. }    => 0.0,
        StanchionDecision::RemoveEdge { .. } => 0.0,
    }
}
