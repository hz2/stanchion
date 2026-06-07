use super::{DiGraph, NodeId};

/// Mutable flow state layered on top of an immutable `DiGraph`.
///
/// `graph` is borrowed with lifetime `'g`; `flow` is owned.  All flow
/// algorithms work exclusively through this type, never mutating `DiGraph`.
pub struct ResidualGraph<'g> {
    graph: &'g DiGraph,
    flow:  Vec<f64>,
}

impl<'g> ResidualGraph<'g> {
    pub fn new(graph: &'g DiGraph) -> Self {
        Self {
            graph,
            flow: vec![0.0; graph.edges.len()],
        }
    }

    /// Initialize from an existing flow vector (e.g., from a prior solver pass).
    pub fn from_flow(graph: &'g DiGraph, flow: Vec<f64>) -> Self {
        debug_assert_eq!(flow.len(), graph.edges.len());
        Self { graph, flow }
    }

    /// Remaining capacity available for pushing flow on this arc.
    #[inline]
    pub fn residual_capacity(&self, edge_idx: usize) -> f64 {
        debug_assert!(edge_idx < self.flow.len());
        self.graph.edges[edge_idx].capacity.0 - self.flow[edge_idx]
    }

    /// Push `amount` units of flow on arc `edge_idx` and deduct from its back-edge.
    #[inline]
    pub fn push_flow(&mut self, edge_idx: usize, amount: f64) {
        debug_assert!(amount >= -1e-12, "cannot push negative flow: {amount}");
        debug_assert!(edge_idx < self.flow.len());
        debug_assert!(edge_idx ^ 1 < self.flow.len());
        self.flow[edge_idx] += amount;
        self.flow[edge_idx ^ 1] -= amount;
    }

    /// Cost of arc `edge_idx` (original graph cost).
    #[inline]
    pub fn edge_cost(&self, edge_idx: usize) -> f64 {
        self.graph.edges[edge_idx].cost.0
    }

    /// Iterate over (destination, raw edge index) for arcs out of `u`.
    #[inline]
    pub fn neighbors(&self, u: NodeId) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.graph.neighbors(u)
    }

    /// Total flow leaving the source (sum of positive flows on source's forward arcs).
    pub fn flow_from_source(&self) -> f64 {
        if let Some(src) = self.graph.source() {
            self.graph.adj[src.0 as usize]
                .iter()
                .map(|&idx| self.flow[idx].max(0.0))
                .sum()
        } else {
            0.0
        }
    }

    /// Total cost of the current flow.
    pub fn total_cost(&self) -> f64 {
        self.flow
            .iter()
            .enumerate()
            .filter(|(i, _)| i % 2 == 0)
            .map(|(i, &f)| f.max(0.0) * self.graph.edges[i].cost.0)
            .sum()
    }

    pub fn flow_vec(&self) -> &[f64] {
        &self.flow
    }

    pub fn graph(&self) -> &'g DiGraph {
        self.graph
    }

    pub fn reset(&mut self) {
        self.flow.fill(0.0);
    }
}
