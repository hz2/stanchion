use std::{cmp::Reverse, collections::BinaryHeap};

use ordered_float::OrderedFloat;

use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId, ResidualGraph},
};

use crate::flow::{MinCostFlowResult, MinCostFlowSolver};

/// Successive Shortest Paths algorithm for min-cost flow.
///
/// Uses Dijkstra with Johnson's potential reweighting so that all reduced
/// costs on the residual graph are non-negative.  Augments along the
/// cheapest s-t path until demand is met or no path exists.
///
/// Complexity: O(F · (E + V log V)) where F is the total integer flow value.
pub struct SuccessiveShortest;

impl MinCostFlowSolver for SuccessiveShortest {
    fn min_cost_flow(
        &self,
        graph:   &DiGraph,
        source:  NodeId,
        sink:    NodeId,
        demand:  Option<f64>,
    ) -> Result<MinCostFlowResult, StanchionError> {
        let n          = graph.node_count();
        let max_demand = demand.unwrap_or(f64::INFINITY);
        let mut res    = ResidualGraph::new(graph);
        let mut total_flow = 0.0f64;
        let mut total_cost = 0.0f64;

        // Bellman-Ford to get initial potentials (handles original negative costs).
        let mut pi = bellman_ford(graph, source, n);

        loop {
            if total_flow >= max_demand - 1e-12 {
                break;
            }

            let (dist, prev) = dijkstra(&res, source, &pi, n);

            if dist[sink.0 as usize].is_infinite() {
                break;
            }

            let path = trace_back(&prev, graph, source, sink);
            if path.is_empty() {
                break;
            }

            let bottle = path
                .iter()
                .map(|&e| res.residual_capacity(e))
                .fold(f64::INFINITY, f64::min)
                .min(max_demand - total_flow);

            debug_assert!(bottle > 0.0, "SSP bottleneck must be positive");
            debug_assert!(!path.is_empty());

            for &e in &path {
                total_cost += bottle * res.edge_cost(e);
                res.push_flow(e, bottle);
            }
            total_flow += bottle;

            // Update potentials with Dijkstra distances.
            for v in 0..n {
                if !dist[v].is_infinite() {
                    pi[v] += dist[v];
                }
            }
        }

        if let Some(d) = demand
            && total_flow < d - 1e-6
        {
            return Err(StanchionError::InfeasibleDemand {
                demand:   d,
                max_flow: total_flow,
            });
        }

        Ok(MinCostFlowResult {
            max_flow:     total_flow,
            min_cost:     total_cost,
            flow_on_edge: res.flow_vec().to_vec(),
        })
    }
}

/// Bellman-Ford shortest paths from source over forward arcs with positive capacity.
fn bellman_ford(graph: &DiGraph, source: NodeId, n: usize) -> Vec<f64> {
    let mut d = vec![f64::INFINITY; n];
    d[source.0 as usize] = 0.0;

    for _ in 0..n.saturating_sub(1) {
        let mut updated = false;
        for (u, adj) in graph.adj.iter().enumerate() {
            if d[u].is_infinite() {
                continue;
            }
            for &idx in adj {
                let e = &graph.edges[idx];
                if e.capacity.0 > 0.0 {
                    let nd = d[u] + e.cost.0;
                    let v  = e.to.0 as usize;
                    if nd < d[v] {
                        d[v] = nd;
                        updated = true;
                    }
                }
            }
        }
        if !updated {
            break;
        }
    }

    d.iter().map(|&x| if x.is_infinite() { 0.0 } else { x }).collect()
}

/// Dijkstra with Johnson's reduced costs.  Returns (distances, predecessor arc index).
fn dijkstra(
    res:    &ResidualGraph<'_>,
    source: NodeId,
    pi:     &[f64],
    n:      usize,
) -> (Vec<f64>, Vec<Option<usize>>) {
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![None::<usize>; n];
    dist[source.0 as usize] = 0.0;

    let mut heap: BinaryHeap<Reverse<(OrderedFloat<f64>, u32)>> = BinaryHeap::new();
    heap.push(Reverse((OrderedFloat(0.0), source.0)));

    while let Some(Reverse((d, u32_idx))) = heap.pop() {
        let u = u32_idx as usize;
        if d.0 > dist[u] + 1e-12 {
            continue;
        }
        for (v, idx) in res.neighbors(NodeId(u32_idx)) {
            if res.residual_capacity(idx) <= 1e-12 {
                continue;
            }
            // Reduced cost: w(u,v) + pi[u] - pi[v] >= 0 by Johnson's invariant.
            let reduced = res.edge_cost(idx) + pi[u] - pi[v.0 as usize];
            let nd      = dist[u] + reduced;
            if nd < dist[v.0 as usize] - 1e-12 {
                dist[v.0 as usize] = nd;
                prev[v.0 as usize] = Some(idx);
                heap.push(Reverse((OrderedFloat(nd), v.0)));
            }
        }
    }

    (dist, prev)
}

/// Trace predecessor arcs from sink to source, returning arc indices in forward order.
fn trace_back(prev: &[Option<usize>], graph: &DiGraph, source: NodeId, sink: NodeId) -> Vec<usize> {
    let mut path = Vec::new();
    let mut cur  = sink.0 as usize;
    while cur != source.0 as usize {
        match prev[cur] {
            None      => return Vec::new(),
            Some(arc) => {
                path.push(arc);
                cur = graph.edges[arc].from.0 as usize;
            }
        }
    }
    path.reverse();
    path
}

#[cfg(test)]
mod tests {
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;

    fn two_path_graph() -> (DiGraph, NodeId, NodeId) {
        // s → a (cap 2, cost 1) → t (cap 2, cost 1)   total cost per unit = 2
        // s → b (cap 3, cost 4) → t (cap 3, cost 4)   total cost per unit = 8
        let (b, s)  = GraphBuilder::new().add_stanchion("s");
        let (b, a)  = b.add_stanchion("a");
        let (b, bv) = b.add_stanchion("b");
        let (b, t)  = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a,  Capacity(2.0), Cost(1.0))
            .connect(s, bv, Capacity(3.0), Cost(4.0))
            .connect(a, t,  Capacity(2.0), Cost(1.0))
            .connect(bv, t, Capacity(3.0), Cost(4.0))
            .build().unwrap();
        (g, s, t)
    }

    #[test]
    fn prefers_cheap_path_for_demand_2() {
        let (g, s, t) = two_path_graph();
        let r = SuccessiveShortest.min_cost_flow(&g, s, t, Some(2.0)).unwrap();
        assert!((r.min_cost - 4.0).abs() < 1e-6, "cost={}", r.min_cost);
    }

    #[test]
    fn max_flow_no_demand() {
        let (g, s, t) = two_path_graph();
        let r = SuccessiveShortest.min_cost_flow(&g, s, t, None).unwrap();
        assert!((r.max_flow - 5.0).abs() < 1e-6);
    }

    #[test]
    fn infeasible_demand_errors() {
        let (g, s, t) = two_path_graph();
        assert!(SuccessiveShortest.min_cost_flow(&g, s, t, Some(100.0)).is_err());
    }
}
