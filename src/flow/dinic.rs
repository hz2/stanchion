use std::collections::VecDeque;

use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId, ResidualGraph},
};

use super::{FlowResult, MaxFlowSolver};

/// Dinic's max-flow algorithm.
///
/// Complexity: O(V² E) general; O(E √V) for unit-capacity graphs.
/// Uses BFS to build a level graph and DFS with current-arc optimisation
/// to find blocking flows.
pub struct Dinic;

impl MaxFlowSolver for Dinic {
    fn max_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
    ) -> Result<FlowResult, StanchionError> {
        let n = graph.node_count();
        let mut residual = ResidualGraph::new(graph);
        let mut level = vec![-1i32; n];
        let mut iter  = vec![0usize; n];
        let mut total = 0.0f64;

        while bfs_level(&residual, source, sink, &mut level) {
            iter.fill(0);
            loop {
                let pushed = dfs_blocking(&mut residual, source, sink, f64::INFINITY, &level, &mut iter);
                debug_assert!(pushed >= 0.0);
                if pushed == 0.0 {
                    break;
                }
                total += pushed;
            }
        }

        Ok(FlowResult {
            max_flow:     total,
            flow_on_edge: residual.flow_vec().to_vec(),
        })
    }
}

/// BFS to compute level labels in the residual graph.
/// Returns true if the sink is reachable from the source.
fn bfs_level(
    residual: &ResidualGraph<'_>,
    source:   NodeId,
    sink:     NodeId,
    level:    &mut [i32],
) -> bool {
    let n = level.len();
    level.fill(-1);
    level[source.0 as usize] = 0;
    let mut queue = VecDeque::with_capacity(n);
    queue.push_back(source);

    while let Some(u) = queue.pop_front() {
        for (v, idx) in residual.graph().neighbors(u) {
            if level[v.0 as usize] < 0 && residual.residual_capacity(idx) > 0.0 {
                level[v.0 as usize] = level[u.0 as usize] + 1;
                queue.push_back(v);
            }
        }
    }

    level[sink.0 as usize] >= 0
}

/// DFS blocking flow with current-arc (iter pointer) optimisation.
/// Returns flow pushed, 0.0 if no augmenting path from u.
fn dfs_blocking(
    residual: &mut ResidualGraph<'_>,
    u:        NodeId,
    sink:     NodeId,
    pushed:   f64,
    level:    &[i32],
    iter:     &mut Vec<usize>,
) -> f64 {
    if u == sink {
        return pushed;
    }
    let u_idx = u.0 as usize;
    let adj_len = residual.graph().adj[u_idx].len();

    while iter[u_idx] < adj_len {
        let edge_idx = residual.graph().adj[u_idx][iter[u_idx]];
        let v        = residual.graph().edges[edge_idx].to;
        let res_cap  = residual.residual_capacity(edge_idx);

        if level[v.0 as usize] == level[u_idx] + 1 && res_cap > 0.0 {
            let d = dfs_blocking(residual, v, sink, pushed.min(res_cap), level, iter);
            debug_assert!(d >= 0.0);
            if d > 0.0 {
                residual.push_flow(edge_idx, d);
                return d;
            }
        }
        iter[u_idx] += 1;
    }
    0.0
}

#[cfg(test)]
mod tests {
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;

    fn diamond_graph() -> (DiGraph, NodeId, NodeId) {
        let (b, s)  = GraphBuilder::new().add_stanchion("s");
        let (b, a)  = b.add_stanchion("a");
        let (b, bv) = b.add_stanchion("b");
        let (b, t)  = b.add_stanchion("t");
        let g = b
            .source(s).sink(t)
            .connect(s, a,  Capacity(1.0), Cost(1.0))
            .connect(s, bv, Capacity(1.0), Cost(1.0))
            .connect(a, t,  Capacity(1.0), Cost(1.0))
            .connect(bv, t, Capacity(1.0), Cost(1.0))
            .build()
            .unwrap();
        (g, s, t)
    }

    #[test]
    fn diamond_max_flow_is_2() {
        let (g, s, t) = diamond_graph();
        let r = Dinic.max_flow(&g, s, t).unwrap();
        assert!((r.max_flow - 2.0).abs() < 1e-9);
    }

    #[test]
    fn single_path() {
        // s → m → t with capacity 3
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(3.0), Cost(1.0))
            .connect(m, t, Capacity(3.0), Cost(1.0))
            .build()
            .unwrap();
        let r = Dinic.max_flow(&g, s, t).unwrap();
        assert!((r.max_flow - 3.0).abs() < 1e-9);
    }

    #[test]
    fn bottleneck_limits_flow() {
        // s → m (cap 5) → t (cap 2): bottleneck is the second edge
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(5.0), Cost(1.0))
            .connect(m, t, Capacity(2.0), Cost(1.0))
            .build()
            .unwrap();
        let r = Dinic.max_flow(&g, s, t).unwrap();
        assert!((r.max_flow - 2.0).abs() < 1e-9);
    }
}
