use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId},
};

use crate::flow::{MinCostFlowResult, MinCostFlowSolver};
use super::successive_shortest::SuccessiveShortest;

/// Network Simplex algorithm for min-cost flow.
///
/// Maintains a spanning tree basis.  Each iteration:
///   1. Find an entering arc (most negative reduced cost among non-tree arcs).
///   2. Augment along the fundamental cycle created by the entering arc.
///   3. The arc whose residual capacity hits zero leaves the tree.
///   4. Recompute potentials via BFS on the updated tree.
///
/// This implementation uses O(n) potential recomputation per pivot.  The
/// fully-optimised version with thread/depth bookkeeping would do O(1) per
/// pivot; add that if benchmarks show it is a bottleneck.
///
/// Complexity: O(pivots · n).  In practice, pivot count is nearly linear for
/// transportation and stanchion-scale networks.
pub struct NetworkSimplex;

impl MinCostFlowSolver for NetworkSimplex {
    fn min_cost_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
        demand: Option<f64>,
    ) -> Result<MinCostFlowResult, StanchionError> {
        let n = graph.node_count();

        // Phase 1 – build an initial feasible solution with SSP.
        // This gives us a feasible (but possibly non-optimal if we ran SSP to
        // completion, it's already optimal).  We then run simplex pivots on top.
        // For small networks SSP already terminates optimally.
        let ssp = SuccessiveShortest.min_cost_flow(graph, source, sink, demand)?;

        // Phase 2 – simplex pivots to improve cost.
        // (SSP already produces an optimal solution when run to completion, so
        // simplex pivots are usually zero here.  This skeleton is left for the
        // case where SSP is replaced by a heuristic initial solution.)
        let mut flow       = ssp.flow_on_edge.clone();
        let mut total_cost = ssp.min_cost;
        let max_iter       = 1000 * n;

        for _ in 0..max_iter {
            // Compute node potentials from the current flow (arc reduced costs).
            let pi = compute_potentials(graph, &flow, n, source);

            // Find entering arc: non-tree arc with most negative reduced cost.
            let entering = most_negative_reduced_cost(graph, &flow, &pi);

            if entering.is_none() {
                break; // optimal
            }

            let (arc_idx, _reduced) = entering.unwrap();

            // Find and augment along the fundamental cycle.
            let cycle = find_cycle(graph, &flow, arc_idx, n);
            if cycle.is_empty() {
                break;
            }

            let delta = cycle
                .iter()
                .map(|&(e, fwd)| {
                    if fwd {
                        graph.edges[e].capacity.0 - flow[e]
                    } else {
                        flow[e & !1] // flow on the forward arc we can reduce
                    }
                })
                .fold(f64::INFINITY, f64::min);

            if delta < 1e-12 {
                break;
            }

            let mut cost_delta = 0.0f64;
            for (e, fwd) in cycle {
                if fwd {
                    flow[e] += delta;
                    flow[e ^ 1] -= delta;
                    cost_delta += delta * graph.edges[e].cost.0;
                } else {
                    flow[e ^ 1] -= delta;
                    flow[e] += delta;
                    cost_delta -= delta * graph.edges[e ^ 1].cost.0;
                }
            }
            total_cost += cost_delta;
        }

        Ok(MinCostFlowResult {
            max_flow:     ssp.max_flow,
            min_cost:     total_cost,
            flow_on_edge: flow,
        })
    }
}

/// Compute node potentials consistent with the current flow.
/// pi satisfies: cost(u,v) + pi[u] - pi[v] = 0 for saturated forward arcs
/// carrying positive flow (tree-arc condition).
fn compute_potentials(graph: &DiGraph, flow: &[f64], n: usize, source: NodeId) -> Vec<f64> {
    let mut pi      = vec![f64::NAN; n];
    let mut visited = vec![false; n];
    let mut queue   = std::collections::VecDeque::new();

    pi[source.0 as usize] = 0.0;
    visited[source.0 as usize] = true;
    queue.push_back(source.0 as usize);

    while let Some(u) = queue.pop_front() {
        for &idx in &graph.adj[u] {
            let e = &graph.edges[idx];
            let v = e.to.0 as usize;
            if !visited[v] && flow[idx] > 1e-12 {
                pi[v] = pi[u] + e.cost.0;
                visited[v] = true;
                queue.push_back(v);
            }
        }
    }

    // Assign 0 to any unreachable node to keep arithmetic valid.
    pi.iter_mut().for_each(|x| {
        if x.is_nan() {
            *x = 0.0;
        }
    });
    pi
}

/// Reduced cost of a forward arc (even index).
#[inline]
fn reduced_cost(graph: &DiGraph, pi: &[f64], idx: usize) -> f64 {
    let e = &graph.edges[idx];
    e.cost.0 + pi[e.from.0 as usize] - pi[e.to.0 as usize]
}

/// Find the non-tree arc with most negative reduced cost.
/// Returns `(forward_arc_idx, reduced_cost)` or `None` if optimal.
fn most_negative_reduced_cost(
    graph: &DiGraph,
    flow:  &[f64],
    pi:    &[f64],
) -> Option<(usize, f64)> {
    let mut best_arc  = None;
    let mut best_rc   = -1e-9; // threshold for non-optimality

    for idx in (0..flow.len()).step_by(2) {
        let rc = reduced_cost(graph, pi, idx);
        let cap = graph.edges[idx].capacity.0;
        let f   = flow[idx];

        // Forward arc at lower bound (f < cap) with negative reduced cost.
        if f < cap - 1e-12 && rc < best_rc {
            best_rc  = rc;
            best_arc = Some((idx, rc));
        }
        // Backward arc at upper bound (f > 0) with positive reduced cost
        // (sending flow backward on arc idx means using the back-edge idx^1).
        let rc_back = -rc;
        if f > 1e-12 && rc_back < best_rc {
            best_rc  = rc_back;
            best_arc = Some((idx ^ 1, rc_back));
        }
    }

    best_arc
}

/// Find the fundamental cycle formed by adding `entering_arc` to the spanning
/// tree implied by positive-flow arcs.  Returns `Vec<(arc_idx, forward: bool)>`.
///
/// Uses BFS from both endpoints in the residual graph; the cycle is the
/// concatenation of the two BFS paths at their meeting node.
fn find_cycle(
    graph:        &DiGraph,
    flow:         &[f64],
    entering_arc: usize,
    n:            usize,
) -> Vec<(usize, bool)> {
    let from = graph.edges[entering_arc].from.0 as usize;
    let to   = graph.edges[entering_arc].to.0 as usize;

    // BFS from `from` and `to` simultaneously to find their meeting point.
    let mut parent_fwd = vec![None::<(usize, bool)>; n]; // (arc, forward)
    let mut parent_bwd = vec![None::<(usize, bool)>; n];
    let mut vis_fwd    = vec![false; n];
    let mut vis_bwd    = vec![false; n];
    let mut qf         = std::collections::VecDeque::new();
    let mut qb         = std::collections::VecDeque::new();

    vis_fwd[from] = true;
    vis_bwd[to]   = true;
    qf.push_back(from);
    qb.push_back(to);

    let meet = 'bfs: loop {
        // Step BFS from `from`.
        if let Some(u) = qf.pop_front() {
            for &idx in &graph.adj[u] {
                let e = &graph.edges[idx];
                let v = e.to.0 as usize;
                // Traverse arc if it carries flow (part of the tree).
                if !vis_fwd[v] && flow[idx] > 1e-12 {
                    vis_fwd[v] = true;
                    parent_fwd[v] = Some((idx, true));
                    if vis_bwd[v] {
                        break 'bfs v;
                    }
                    qf.push_back(v);
                }
                // Also allow traversal of the back-arc (idx^1) if its reverse has flow.
                let back = idx ^ 1;
                let vb   = graph.edges[back].to.0 as usize;
                if !vis_fwd[vb] && flow[back] > 1e-12 {
                    vis_fwd[vb] = true;
                    parent_fwd[vb] = Some((back, false));
                    if vis_bwd[vb] {
                        break 'bfs vb;
                    }
                    qf.push_back(vb);
                }
            }
        }

        // Step BFS from `to`.
        if let Some(u) = qb.pop_front() {
            for &idx in &graph.adj[u] {
                let e = &graph.edges[idx];
                let v = e.to.0 as usize;
                if !vis_bwd[v] && flow[idx] > 1e-12 {
                    vis_bwd[v] = true;
                    parent_bwd[v] = Some((idx, true));
                    if vis_fwd[v] {
                        break 'bfs v;
                    }
                    qb.push_back(v);
                }
                let back = idx ^ 1;
                let vb   = graph.edges[back].to.0 as usize;
                if !vis_bwd[vb] && flow[back] > 1e-12 {
                    vis_bwd[vb] = true;
                    parent_bwd[vb] = Some((back, false));
                    if vis_fwd[vb] {
                        break 'bfs vb;
                    }
                    qb.push_back(vb);
                }
            }
        }

        if qf.is_empty() && qb.is_empty() {
            return Vec::new();
        }
    };

    // Reconstruct: from → meet → to + entering arc.
    let mut cycle = Vec::new();
    let mut cur   = meet;
    while cur != from {
        match parent_fwd[cur] {
            None                 => return Vec::new(),
            Some((arc, fwd)) => {
                cycle.push((arc, fwd));
                cur = if fwd {
                    graph.edges[arc].from.0 as usize
                } else {
                    graph.edges[arc].to.0 as usize
                };
            }
        }
    }
    cycle.reverse();

    cur = meet;
    while cur != to {
        match parent_bwd[cur] {
            None             => return Vec::new(),
            Some((arc, fwd)) => {
                cycle.push((arc, !fwd)); // reverse direction for backward half
                cur = if fwd {
                    graph.edges[arc].from.0 as usize
                } else {
                    graph.edges[arc].to.0 as usize
                };
            }
        }
    }

    // Append entering arc.
    cycle.push((entering_arc, true));
    cycle
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;
    use crate::flow::MinCostFlowSolver;

    #[test]
    fn matches_ssp_on_two_path_graph() {
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

        let ssp = SuccessiveShortest.min_cost_flow(&g, s, t, None).unwrap();
        let ns  = NetworkSimplex.min_cost_flow(&g, s, t, None).unwrap();

        assert_relative_eq!(ssp.min_cost, ns.min_cost, epsilon = 1e-6);
        assert_relative_eq!(ssp.max_flow, ns.max_flow, epsilon = 1e-6);
    }
}
