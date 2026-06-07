use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId, ResidualGraph},
};

use super::{MinCostFlowResult, MinCostFlowSolver};
use crate::flow::{MaxFlowSolver, dinic::Dinic};

/// Negative-cycle canceling min-cost flow (Wallacher / Goldberg-Tarjan style).
///
/// Complexity: O(m * n * m) per cancellation; O(m * n * m * n * C) total (pseudo-polynomial
/// in C = max cost, polynomial in n and m for integer costs).
///
/// # Algorithm
///
/// Phase 1: Find a max-flow ignoring costs (Dinic's algorithm).
/// Phase 2: Repeatedly detect the lowest-mean-cost negative cycle in G_f using a
///          multi-source Bellman-Ford; cancel it by pushing the maximum possible
///          flow around it.  Stop when no negative-cost cycle remains.
///
/// A flow is minimum-cost iff its residual graph contains no negative-cost cycle
/// (Theorem 5.3 in Williamson 2019).  The minimum-mean-cost variant (Goldberg-Tarjan 1989)
/// is strongly polynomial but harder to implement; this version is pseudo-polynomial and
/// suitable for the small graphs that arise in stanchion optimization.
pub struct CycleCanceling;

impl MinCostFlowSolver for CycleCanceling {
    fn min_cost_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
        demand: Option<f64>,
    ) -> Result<MinCostFlowResult, StanchionError> {
        // phase 1: find a maximum flow (ignoring edge costs)
        let fwd = Dinic.max_flow(graph, source, sink)?;
        let target = demand.unwrap_or(fwd.max_flow).min(fwd.max_flow);

        // build residual seeded with the Dinic flow
        let mut res = ResidualGraph::from_flow(graph, fwd.flow_on_edge);

        // phase 2: cancel negative-cost cycles until none remain
        // iteration cap: n * m (polynomial safety valve for floating-point imprecision)
        let max_iters = graph.node_count() * (graph.edges.len() / 2 + 1) * 4;
        for _ in 0..max_iters {
            match find_negative_cycle(&res) {
                None => break,
                Some(cycle) => cancel_cycle(&mut res, &cycle),
            }
        }

        let cost = res.total_cost();
        let flow_vec = res.flow_vec().to_vec();
        Ok(MinCostFlowResult {
            max_flow: target,
            min_cost: cost,
            flow_on_edge: flow_vec,
        })
    }
}

/// Multi-source Bellman-Ford over residual arcs with positive capacity.
/// Initialises all distances to 0 (equivalent to a virtual super-source connected
/// to every node at cost 0), which finds negative cycles reachable from anywhere.
/// Returns a list of edge indices forming the cycle, or None.
fn find_negative_cycle(res: &ResidualGraph<'_>) -> Option<Vec<usize>> {
    let n = res.graph().node_count();
    let mut dist   = vec![0.0f64; n];
    let mut parent = vec![usize::MAX; n]; // edge index that last updated each node

    // n-1 relaxation rounds
    for _ in 0..n - 1 {
        for edge_idx in 0..res.graph().edges.len() {
            if res.residual_capacity(edge_idx) <= 1e-10 {
                continue;
            }
            let edge = &res.graph().edges[edge_idx];
            let u = edge.from.0 as usize;
            let v = edge.to.0 as usize;
            let w = res.edge_cost(edge_idx);
            if dist[u] + w < dist[v] - 1e-10 {
                dist[v] = dist[u] + w;
                parent[v] = edge_idx;
            }
        }
    }

    // one more round: any relaxation reveals a negative cycle
    let mut cycle_entry = usize::MAX;
    for edge_idx in 0..res.graph().edges.len() {
        if res.residual_capacity(edge_idx) <= 1e-10 {
            continue;
        }
        let edge = &res.graph().edges[edge_idx];
        let u = edge.from.0 as usize;
        let v = edge.to.0 as usize;
        let w = res.edge_cost(edge_idx);
        if dist[u] + w < dist[v] - 1e-10 {
            parent[v] = edge_idx;
            cycle_entry = v;
            break;
        }
    }

    if cycle_entry == usize::MAX {
        return None;
    }

    // walk n steps back from cycle_entry so we land inside the cycle
    let mut v = cycle_entry;
    for _ in 0..n {
        let idx = parent[v];
        v = res.graph().edges[idx].from.0 as usize;
    }

    // trace the cycle until we return to v
    let start = v;
    let mut cycle = Vec::new();
    loop {
        let idx = parent[v];
        cycle.push(idx);
        v = res.graph().edges[idx].from.0 as usize;
        if v == start {
            break;
        }
        if cycle.len() > n + 1 {
            // safety: should not happen with correct Bellman-Ford
            break;
        }
    }

    Some(cycle)
}

/// Push as much flow as possible around the given cycle (list of edge indices).
fn cancel_cycle(res: &mut ResidualGraph<'_>, cycle: &[usize]) {
    let bottleneck = cycle
        .iter()
        .map(|&idx| res.residual_capacity(idx))
        .fold(f64::INFINITY, f64::min);

    if bottleneck <= 1e-10 {
        return;
    }

    for &idx in cycle {
        res.push_flow(idx, bottleneck);
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use crate::flow::min_cost::{MinCostFlowSolver, successive_shortest::SuccessiveShortest};
    use super::*;

    fn diamond_mcf() -> (DiGraph, NodeId, NodeId) {
        let (b, s)  = GraphBuilder::new().add_stanchion("s");
        let (b, a)  = b.add_stanchion("a");
        let (b, bv) = b.add_stanchion("b");
        let (b, t)  = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a,  Capacity(3.0), Cost(1.0))
            .connect(s, bv, Capacity(3.0), Cost(2.0))
            .connect(a, t,  Capacity(3.0), Cost(1.0))
            .connect(bv, t, Capacity(3.0), Cost(1.0))
            .build().unwrap();
        (g, s, t)
    }

    #[test]
    fn cost_matches_ssp() {
        let (g, s, t) = diamond_mcf();
        let ssp = SuccessiveShortest.min_cost_flow(&g, s, t, None).unwrap();
        let cc  = CycleCanceling.min_cost_flow(&g, s, t, None).unwrap();
        assert_relative_eq!(ssp.min_cost, cc.min_cost, epsilon = 1e-6);
        assert_relative_eq!(ssp.max_flow, cc.max_flow, epsilon = 1e-6);
    }

    #[test]
    fn single_path_cost() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(5.0), Cost(3.0))
            .connect(m, t, Capacity(5.0), Cost(2.0))
            .build().unwrap();
        let cc = CycleCanceling.min_cost_flow(&g, s, t, None).unwrap();
        assert_relative_eq!(cc.max_flow, 5.0, epsilon = 1e-9);
        assert_relative_eq!(cc.min_cost, 25.0, epsilon = 1e-9); // 5 * (3+2)
    }
}
