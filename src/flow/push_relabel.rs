use std::collections::VecDeque;

use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId, ResidualGraph},
};

use super::{FlowResult, MaxFlowSolver};

/// Push-relabel max-flow with FIFO selection and gap heuristic.
///
/// Complexity: O(V² √E).
pub struct PushRelabel;

impl MaxFlowSolver for PushRelabel {
    fn max_flow(
        &self,
        graph:  &DiGraph,
        source: NodeId,
        sink:   NodeId,
    ) -> Result<FlowResult, StanchionError> {
        let n   = graph.node_count();
        let src = source.0 as usize;
        let snk = sink.0 as usize;

        let mut res      = ResidualGraph::new(graph);
        let mut height   = vec![0u32; n];
        let mut excess   = vec![0.0f64; n];
        let mut cur_arc  = vec![0usize; n];
        let mut active   = VecDeque::<usize>::new();
        // gap_count[h] = nodes with height h (excl. source/sink)
        let mut gap_count = vec![0u32; 2 * n + 2];

        height[src] = n as u32;
        gap_count[0] = (n as u32).saturating_sub(2);

        for (v, idx) in graph.neighbors(source) {
            let cap = res.residual_capacity(idx);
            if cap > 0.0 && v.0 as usize != src {
                res.push_flow(idx, cap);
                excess[v.0 as usize] += cap;
                excess[src] -= cap;
                if v.0 as usize != snk && excess[v.0 as usize] == cap {
                    active.push_back(v.0 as usize);
                }
            }
        }

        while let Some(u) = active.pop_front() {
            let mut ctx = DischargeCtx {
                height:    &mut height,
                excess:    &mut excess,
                cur_arc:   &mut cur_arc,
                active:    &mut active,
                gap_count: &mut gap_count,
            };
            discharge(&mut res, u, snk, src, n, &mut ctx);
        }

        Ok(FlowResult {
            max_flow:     excess[snk].max(0.0),
            flow_on_edge: res.flow_vec().to_vec(),
        })
    }
}

/// Mutable algorithm state passed to `discharge` and `gap_relabel`.
struct DischargeCtx<'a> {
    height:    &'a mut [u32],
    excess:    &'a mut [f64],
    cur_arc:   &'a mut [usize],
    active:    &'a mut VecDeque<usize>,
    gap_count: &'a mut [u32],
}

fn discharge(
    res:    &mut ResidualGraph<'_>,
    u:      usize,
    sink:   usize,
    source: usize,
    n:      usize,
    ctx:    &mut DischargeCtx<'_>,
) {
    let adj_len = res.graph().adj[u].len();

    while ctx.excess[u] > 1e-12 {
        if ctx.cur_arc[u] == adj_len {
            let new_h = relabel(res, u, n, ctx.height);
            let old_h = ctx.height[u] as usize;
            if old_h < 2 * n && ctx.gap_count[old_h] == 0 {
                gap_relabel(ctx, old_h, n, source, sink);
            }
            ctx.height[u] = new_h;
            if (new_h as usize) < 2 * n {
                ctx.gap_count[new_h as usize] += 1;
            }
            ctx.cur_arc[u] = 0;
            if new_h >= n as u32 * 2 {
                return;
            }
        }

        let edge_idx = res.graph().adj[u][ctx.cur_arc[u]];
        let v        = res.graph().edges[edge_idx].to.0 as usize;
        let res_cap  = res.residual_capacity(edge_idx);

        if res_cap > 1e-12 && ctx.height[u] == ctx.height[v] + 1 {
            let delta = ctx.excess[u].min(res_cap);
            debug_assert!(delta > 0.0, "push amount must be positive");
            res.push_flow(edge_idx, delta);
            ctx.excess[u] -= delta;
            ctx.excess[v] += delta;
            if v != sink && v != source && ctx.excess[v] - delta < 1e-12 {
                ctx.active.push_back(v);
            }
        } else {
            ctx.cur_arc[u] += 1;
        }
    }
}

fn relabel(res: &ResidualGraph<'_>, u: usize, n: usize, height: &[u32]) -> u32 {
    let mut min_h = u32::MAX;
    for &idx in &res.graph().adj[u] {
        if res.residual_capacity(idx) > 1e-12 {
            let v = res.graph().edges[idx].to.0 as usize;
            min_h = min_h.min(height[v]);
        }
    }
    if min_h == u32::MAX { 2 * n as u32 } else { min_h + 1 }
}

/// Raise all nodes above gap_h to height 2n (disconnected from sink).
fn gap_relabel(ctx: &mut DischargeCtx<'_>, gap_h: usize, n: usize, source: usize, sink: usize) {
    for v in 0..n {
        if v == source || v == sink {
            continue;
        }
        if ctx.height[v] as usize > gap_h {
            if (ctx.height[v] as usize) < 2 * n {
                ctx.gap_count[ctx.height[v] as usize] =
                    ctx.gap_count[ctx.height[v] as usize].saturating_sub(1);
            }
            ctx.height[v] = 2 * n as u32;
        }
    }
    ctx.active.retain(|&v| {
        ctx.excess[v] > 1e-12 && ctx.height[v] < 2 * n as u32
    });
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;
    use crate::flow::dinic::Dinic;
    use crate::flow::MaxFlowSolver;

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
            .build().unwrap();
        (g, s, t)
    }

    #[test]
    fn diamond_matches_dinic() {
        let (g, s, t) = diamond_graph();
        let dr = Dinic.max_flow(&g, s, t).unwrap();
        let pr = PushRelabel.max_flow(&g, s, t).unwrap();
        assert_relative_eq!(dr.max_flow, pr.max_flow, epsilon = 1e-9);
    }

    #[test]
    fn bottleneck_matches_dinic() {
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, m) = b.add_stanchion("m");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, m, Capacity(10.0), Cost(1.0))
            .connect(m, t, Capacity(4.0),  Cost(1.0))
            .build().unwrap();
        let dr = Dinic.max_flow(&g, s, t).unwrap();
        let pr = PushRelabel.max_flow(&g, s, t).unwrap();
        assert_relative_eq!(dr.max_flow, pr.max_flow, epsilon = 1e-9);
    }
}
