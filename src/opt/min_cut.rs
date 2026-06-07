use std::collections::VecDeque;

use crate::{
    flow::FlowResult,
    graph::{DiGraph, EdgeId, NodeId},
};

/// Find all forward edges that form the min-cut.
///
/// Algorithm: BFS on the residual graph from the source, traversing only
/// arcs with positive residual capacity.  The min-cut consists of forward
/// edges (u, v) where `u` is reachable and `v` is not.
pub fn find_min_cut(
    graph:  &DiGraph,
    result: &FlowResult,
    source: NodeId,
) -> Vec<EdgeId> {
    let n   = graph.node_count();
    let mut reachable = vec![false; n];
    let mut queue     = VecDeque::new();

    reachable[source.0 as usize] = true;
    queue.push_back(source);

    while let Some(u) = queue.pop_front() {
        for &idx in &graph.adj[u.0 as usize] {
            let v       = graph.edges[idx].to;
            let cap     = graph.edges[idx].capacity.0;
            let flow    = result.flow_on_edge.get(idx).copied().unwrap_or(0.0);
            let res_cap = cap - flow;

            if !reachable[v.0 as usize] && res_cap > 1e-12 {
                reachable[v.0 as usize] = true;
                queue.push_back(v);
            }
        }
    }

    // Collect forward edges crossing from S-side to T-side.
    let mut cut = Vec::new();
    for idx in (0..graph.edges.len()).step_by(2) {
        let e = &graph.edges[idx];
        if reachable[e.from.0 as usize] && !reachable[e.to.0 as usize] {
            cut.push(EdgeId(idx as u32));
        }
    }
    cut
}
