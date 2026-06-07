use std::collections::VecDeque;

use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId, ResidualGraph},
};

use super::{FlowResult, MaxFlowSolver};

/// Capacity-scaling max-flow (Gabow 1985; Ahuja-Magnanti-Orlin).
///
/// Complexity: O(m^2 log U) where U = max arc capacity.
///
/// # Algorithm
///
/// Maintains a scaling parameter `delta` halved each phase:
/// `delta = 2^floor(log2(U))`, ..., 4, 2, 1.
/// Each phase only uses arcs with residual capacity >= `delta`,
/// forming the *delta-residual subgraph* G_f(delta).  At most 2m
/// augmentations occur per phase (Lemma 2.26 in Williamson 2019),
/// giving O(m log U) augmentations total, each taking O(m) BFS.
pub struct CapacityScaling;

impl MaxFlowSolver for CapacityScaling {
    fn max_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
    ) -> Result<FlowResult, StanchionError> {
        let n_edges = graph.edges.len();
        if n_edges == 0 {
            return Ok(FlowResult { max_flow: 0.0, flow_on_edge: vec![] });
        }

        let u_max = graph
            .edges
            .iter()
            .step_by(2)
            .map(|e| e.capacity.0)
            .fold(0.0f64, f64::max);

        if u_max <= 0.0 {
            return Ok(FlowResult { max_flow: 0.0, flow_on_edge: vec![0.0; n_edges] });
        }

        let mut res   = ResidualGraph::new(graph);
        // largest power of 2 that does not exceed u_max
        debug_assert!(u_max > 0.0);
        let mut delta = (2.0f64).powi(u_max.log2().floor() as i32);
        let mut total = 0.0f64;

        let n = graph.node_count();
        let mut parent = vec![usize::MAX; n];

        while delta >= 1.0 {
            loop {
                // BFS in G_f(delta); fills parent[] with the arriving edge index
                if !bfs_delta(&res, source, sink, delta, &mut parent) {
                    break;
                }
                total += augment(&mut res, &parent, source, sink);
            }
            delta /= 2.0;
        }

        Ok(FlowResult {
            max_flow:     total,
            flow_on_edge: res.flow_vec().to_vec(),
        })
    }
}

/// BFS over G_f(delta): arcs with residual_capacity >= delta.
/// Stores the arriving edge index in parent[v] for each reached node.
/// Returns true iff the sink is reached.
fn bfs_delta(
    res:    &ResidualGraph<'_>,
    source: NodeId,
    sink:   NodeId,
    delta:  f64,
    parent: &mut [usize],
) -> bool {
    let n = res.graph().node_count();
    // reuse parent as visited sentinel (usize::MAX = not visited)
    parent.fill(usize::MAX);

    let src = source.0 as usize;
    let snk = sink.0 as usize;
    parent[src] = usize::MAX - 1; // mark source visited without a real edge
    let mut queue = VecDeque::with_capacity(n);
    queue.push_back(source);

    while let Some(u) = queue.pop_front() {
        if u.0 as usize == snk {
            return true;
        }
        for (v, idx) in res.graph().neighbors(u) {
            let vi = v.0 as usize;
            if parent[vi] == usize::MAX && res.residual_capacity(idx) >= delta {
                parent[vi] = idx;
                queue.push_back(v);
            }
        }
    }
    false
}

/// Trace path from sink back to source via parent[], find bottleneck, push it.
fn augment(
    res:    &mut ResidualGraph<'_>,
    parent: &[usize],
    source: NodeId,
    sink:   NodeId,
) -> f64 {
    // find bottleneck
    let mut bottleneck = f64::INFINITY;
    let mut v = sink;
    while v != source {
        let idx = parent[v.0 as usize];
        debug_assert!(idx != usize::MAX, "parent not set for node on path");
        bottleneck = bottleneck.min(res.residual_capacity(idx));
        v = res.graph().edges[idx].from;
    }
    debug_assert!(bottleneck > 0.0, "bottleneck must be positive after BFS found a path");
    // push along path
    let mut v = sink;
    while v != source {
        let idx = parent[v.0 as usize];
        res.push_flow(idx, bottleneck);
        v = res.graph().edges[idx].from;
    }
    bottleneck
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use crate::flow::{MaxFlowSolver, dinic::Dinic};
    use super::*;

    fn five_node_graph() -> (DiGraph, NodeId, NodeId) {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, a) = b.add_stanchion("a");
        let (b, bv) = b.add_stanchion("b");
        let (b, c) = b.add_stanchion("c");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a,  Capacity(10.0), Cost(1.0))
            .connect(s, bv, Capacity(10.0), Cost(1.0))
            .connect(a, c,  Capacity(4.0),  Cost(1.0))
            .connect(bv, c, Capacity(8.0),  Cost(1.0))
            .connect(a, t,  Capacity(6.0),  Cost(1.0))
            .connect(c, t,  Capacity(10.0), Cost(1.0))
            .build().unwrap();
        (g, s, t)
    }

    #[test]
    fn matches_dinic_five_node() {
        let (g, s, t) = five_node_graph();
        let dr = Dinic.max_flow(&g, s, t).unwrap();
        let cs = CapacityScaling.max_flow(&g, s, t).unwrap();
        assert_relative_eq!(dr.max_flow, cs.max_flow, epsilon = 1e-9);
    }

    #[test]
    fn single_bottleneck() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(100.0), Cost(1.0))
            .connect(m, t, Capacity(7.0),   Cost(1.0))
            .build().unwrap();
        let cs = CapacityScaling.max_flow(&g, s, t).unwrap();
        assert_relative_eq!(cs.max_flow, 7.0, epsilon = 1e-9);
    }

    #[test]
    fn large_capacity_matches_dinic() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, a) = b.add_stanchion("a");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a, Capacity(1_000_000.0), Cost(1.0))
            .connect(a, t, Capacity(999_999.0),   Cost(1.0))
            .build().unwrap();
        let dr = Dinic.max_flow(&g, s, t).unwrap();
        let cs = CapacityScaling.max_flow(&g, s, t).unwrap();
        assert_relative_eq!(dr.max_flow, cs.max_flow, epsilon = 1e-6);
    }
}
