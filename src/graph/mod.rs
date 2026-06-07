pub mod edge;
pub mod node;
pub mod residual;

pub use edge::{Capacity, Cost, Edge, EdgeId};
pub use node::{NodeData, NodeId};
pub use residual::ResidualGraph;

use crate::error::StanchionError;

/// Directed graph using a flat edge list with XOR back-edge pairing.
///
/// For every forward edge at index `i` (even), its residual back-edge lives at `i ^ 1`.
/// Flow algorithms operate on `ResidualGraph`, not directly on `DiGraph`.
pub struct DiGraph {
    pub(crate) adj:    Vec<Vec<usize>>,
    pub(crate) edges:  Vec<Edge>,
    pub(crate) nodes:  Vec<NodeData>,
    source: Option<NodeId>,
    sink:   Option<NodeId>,
}

impl DiGraph {
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Number of forward edges (excludes residual back-edges).
    pub fn edge_count(&self) -> usize {
        self.edges.len() / 2
    }

    pub fn source(&self) -> Option<NodeId> {
        self.source
    }

    pub fn sink(&self) -> Option<NodeId> {
        self.sink
    }

    pub fn node_data(&self, id: NodeId) -> Option<&NodeData> {
        self.nodes.get(id.0 as usize)
    }

    pub fn edge(&self, id: EdgeId) -> Option<&Edge> {
        self.edges.get(id.0 as usize)
    }

    /// Add a node and return its id.
    pub fn add_node(&mut self, data: NodeData) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(data);
        self.adj.push(Vec::new());
        id
    }

    /// Add a directed forward edge plus its zero-capacity residual back-edge.
    /// Returns the EdgeId of the forward edge (always even).
    pub fn add_edge(&mut self, from: NodeId, to: NodeId, cap: Capacity, cost: Cost) -> EdgeId {
        debug_assert!(cap.0 >= 0.0, "capacity must be non-negative");
        debug_assert!((from.0 as usize) < self.nodes.len());
        debug_assert!((to.0 as usize) < self.nodes.len());

        let fwd_idx = self.edges.len();
        // XOR pairing: forward at even index, back-edge at odd (fwd ^ 1)
        debug_assert!(fwd_idx.is_multiple_of(2), "edges vec must always grow in pairs");
        let id = EdgeId(fwd_idx as u32);

        self.edges.push(Edge { from, to, capacity: cap, cost });
        // back-edge: zero capacity, negated cost (for min-cost residual correctness)
        self.edges.push(Edge {
            from:     to,
            to:       from,
            capacity: Capacity(0.0),
            cost:     Cost(-cost.0),
        });

        self.adj[from.0 as usize].push(fwd_idx);
        self.adj[to.0 as usize].push(fwd_idx ^ 1);
        id
    }

    /// Iterate over (destination NodeId, raw edge index) for all arcs out of `u`,
    /// including residual back-edges.  Flow algorithms use this.
    pub fn neighbors(&self, u: NodeId) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.adj[u.0 as usize]
            .iter()
            .map(|&idx| (self.edges[idx].to, idx))
    }

    /// Iterate over (destination, edge index) for **forward** arcs only (even indices).
    /// Use this for routing and path-finding algorithms that operate on the original graph.
    pub fn forward_neighbors(&self, u: NodeId) -> impl Iterator<Item = (NodeId, usize)> + '_ {
        self.adj[u.0 as usize]
            .iter()
            .copied()
            .filter(|idx| idx % 2 == 0)
            .map(|idx| (self.edges[idx].to, idx))
    }

    /// True if this edge index is a forward (original) edge, not a residual back-edge.
    pub fn is_forward_edge(&self, id: EdgeId) -> bool {
        id.0.is_multiple_of(2)
    }
}

/// Fluent builder for `DiGraph`.
pub struct GraphBuilder {
    graph:  DiGraph,
}

impl Default for GraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl GraphBuilder {
    pub fn new() -> Self {
        Self {
            graph: DiGraph {
                adj:    Vec::new(),
                edges:  Vec::new(),
                nodes:  Vec::new(),
                source: None,
                sink:   None,
            },
        }
    }

    /// Add a stanchion post node with a label. Returns `(builder, NodeId)`.
    pub fn add_stanchion(mut self, label: impl Into<Box<str>>) -> (Self, NodeId) {
        let id = self.graph.add_node(NodeData { label: Some(label.into()) });
        (self, id)
    }

    pub fn source(mut self, id: NodeId) -> Self {
        self.graph.source = Some(id);
        self
    }

    pub fn sink(mut self, id: NodeId) -> Self {
        self.graph.sink = Some(id);
        self
    }

    /// Connect two stanchion posts with a belt of given capacity and cost.
    pub fn connect(mut self, from: NodeId, to: NodeId, cap: Capacity, cost: Cost) -> Self {
        self.graph.add_edge(from, to, cap, cost);
        self
    }

    pub fn build(self) -> Result<DiGraph, StanchionError> {
        if self.graph.source.is_none() {
            return Err(StanchionError::NoSource);
        }
        if self.graph.sink.is_none() {
            return Err(StanchionError::NoSink);
        }
        Ok(self.graph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diamond() -> DiGraph {
        // s → a → t
        // s → b → t
        // max flow should be 2 if each edge has capacity 1
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, a) = b.add_stanchion("a");
        let (b, bnode) = b.add_stanchion("b");
        let (b, t) = b.add_stanchion("t");
        b.source(s)
            .sink(t)
            .connect(s, a, Capacity(1.0), Cost(1.0))
            .connect(s, bnode, Capacity(1.0), Cost(1.0))
            .connect(a, t, Capacity(1.0), Cost(1.0))
            .connect(bnode, t, Capacity(1.0), Cost(1.0))
            .build()
            .unwrap()
    }

    #[test]
    fn xor_back_edge_pairing() {
        let g = diamond();
        // Every even-indexed edge should have a matching odd back-edge
        for i in (0..g.edges.len()).step_by(2) {
            let fwd = &g.edges[i];
            let back = &g.edges[i ^ 1];
            assert_eq!(fwd.to, back.from);
            assert_eq!(fwd.from, back.to);
            assert_eq!(back.capacity.0, 0.0);
        }
    }

    #[test]
    fn adjacency_counts() {
        let g = diamond();
        // s has 2 outgoing + 0 incoming back-edges visible to it
        assert_eq!(g.adj[0].len(), 2);
        // t has 0 outgoing + 2 incoming (2 back-edges pointing back toward s)
        assert_eq!(g.adj[3].len(), 2);
    }
}
