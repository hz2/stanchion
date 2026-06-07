use crate::{
    error::StanchionError,
    graph::{Capacity, Cost, DiGraph, EdgeId, GraphBuilder, NodeId},
};

/// Number of discrete time steps in the expanded graph.
#[derive(Debug, Clone, Copy)]
pub struct TimeHorizon(pub u32);

/// Build a time-expanded graph for dynamic network flow.
///
/// For each original node v and timestep t in `[0, T)`, the expanded node
/// lives at index `v + t * n`.
///
/// Two arc types are added:
/// - **Transit**: for each original arc `(u,v)` with transit time `d`, add arc
///   `(u,t) -> (v, t+d)` for all `t` where `t + d < T`.
/// - **Holding**: `(v,t) -> (v, t+1)` with infinite capacity and zero cost
///   (waiting in place at a stanchion post).
///
/// Returns the expanded graph and the expanded source (t=0) and sink (t=T-1).
pub fn build_time_expanded(
    base:        &DiGraph,
    horizon:     TimeHorizon,
    transit_fn:  impl Fn(EdgeId) -> u32,
) -> Result<(DiGraph, NodeId, NodeId), StanchionError> {
    let t   = horizon.0 as usize;
    let n   = base.node_count();
    let src = base.source().ok_or(StanchionError::NoSource)?;
    let snk = base.sink().ok_or(StanchionError::NoSink)?;

    // expanded node (v, step) = NodeId(v + step * n)
    let total_nodes = n * t;
    if total_nodes > 1 << 20 {
        return Err(StanchionError::TimeHorizonTooLarge(horizon.0));
    }

    let mut builder = GraphBuilder::new();

    // Create n*T nodes.
    for step in 0..t {
        for v in 0..n {
            let label = format!("({v},{step})");
            let (b, _) = builder.add_stanchion(label.as_str());
            builder = b;
        }
    }

    // Helper: expanded NodeId for (original node v, timestep step).
    let exp_node = |v: usize, step: usize| NodeId((v + step * n) as u32);

    let exp_src = exp_node(src.0 as usize, 0);
    let exp_snk = exp_node(snk.0 as usize, t - 1);

    builder = builder.source(exp_src).sink(exp_snk);

    // Transit arcs.
    for step in 0..t {
        for (u, adj) in base.adj.iter().enumerate() {
            for &idx in adj {
                // Only forward (original) arcs.
                if idx % 2 != 0 {
                    continue;
                }
                let e      = &base.edges[idx];
                let d      = transit_fn(EdgeId(idx as u32)) as usize;
                let arrive = step + d;
                if arrive < t {
                    builder = builder.connect(
                        exp_node(u, step),
                        exp_node(e.to.0 as usize, arrive),
                        e.capacity,
                        e.cost,
                    );
                }
            }
        }
    }

    // Holding arcs: (v, step) → (v, step+1) with ∞ capacity, zero cost.
    let infinite_cap = Capacity(1e18);
    for step in 0..t - 1 {
        for v in 0..n {
            builder = builder.connect(
                exp_node(v, step),
                exp_node(v, step + 1),
                infinite_cap,
                Cost(0.0),
            );
        }
    }

    Ok((builder.build()?, exp_src, exp_snk))
}

#[cfg(test)]
mod tests {
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;

    #[test]
    fn two_node_horizon_3() {
        // s → t (cap 1, transit 1)
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, t) = b.add_stanchion("t");
        let base   = b.source(s).sink(t)
            .connect(s, t, Capacity(1.0), Cost(1.0))
            .build().unwrap();

        let (exp, _, _) = build_time_expanded(&base, TimeHorizon(3), |_| 1).unwrap();
        // 2 nodes * 3 steps = 6 expanded nodes
        assert_eq!(exp.node_count(), 6);
    }
}
