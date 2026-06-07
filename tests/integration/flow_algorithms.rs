//! Cross-algorithm verification: all max-flow and MCF solvers must agree.

use approx::assert_relative_eq;
use rand::{Rng, SeedableRng, rngs::SmallRng};

use stanchion::{
    flow::{
        FlowResult, MaxFlowSolver, MinCostFlowSolver,
        dinic::Dinic,
        min_cost::{CycleCanceling, NetworkSimplex, SuccessiveShortest},
        push_relabel::PushRelabel,
        scaling::CapacityScaling,
    },
    graph::{Capacity, Cost, DiGraph, GraphBuilder, NodeId},
};

fn random_graph(n: usize, edges: usize, seed: u64) -> DiGraph {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut b   = GraphBuilder::new();
    for i in 0..n {
        let (nb, _) = b.add_stanchion(format!("n{i}").as_str());
        b = nb;
    }
    b = b.source(NodeId(0)).sink(NodeId((n - 1) as u32));
    for _ in 0..edges {
        let u = rng.random_range(0..n as u32);
        let v = rng.random_range(0..n as u32);
        if u != v {
            b = b.connect(
                NodeId(u),
                NodeId(v),
                Capacity(rng.random_range(1..=20) as f64),
                Cost(rng.random_range(1..=10) as f64),
            );
        }
    }
    b.build().unwrap_or_else(|_| {
        let (b, _) = GraphBuilder::new().add_stanchion("s");
        let (b, _) = b.add_stanchion("t");
        b.source(NodeId(0)).sink(NodeId(1))
            .connect(NodeId(0), NodeId(1), Capacity(1.0), Cost(1.0))
            .build().unwrap()
    })
}

#[test]
fn max_flow_algorithms_agree_small() {
    for seed in 0..20u64 {
        let g = random_graph(6, 12, seed);
        let s = g.source().unwrap();
        let t = g.sink().unwrap();

        let expected = Dinic.max_flow(&g, s, t).unwrap().max_flow;
        let pr = PushRelabel.max_flow(&g, s, t).unwrap().max_flow;
        let cs = CapacityScaling.max_flow(&g, s, t).unwrap().max_flow;

        assert!(
            (expected - pr).abs() < 1e-6,
            "PushRelabel disagrees on seed={seed}: Dinic={expected} PR={pr}"
        );
        assert!(
            (expected - cs).abs() < 1e-6,
            "CapacityScaling disagrees on seed={seed}: Dinic={expected} CS={cs}"
        );
    }
}

#[test]
fn max_flow_algorithms_agree_medium() {
    for seed in [42u64, 99, 1234, 9999] {
        let g = random_graph(12, 30, seed);
        let s = g.source().unwrap();
        let t = g.sink().unwrap();

        let expected = Dinic.max_flow(&g, s, t).unwrap().max_flow;
        let pr = PushRelabel.max_flow(&g, s, t).unwrap().max_flow;
        let cs = CapacityScaling.max_flow(&g, s, t).unwrap().max_flow;

        assert_relative_eq!(expected, pr, epsilon = 1e-6);
        assert_relative_eq!(expected, cs, epsilon = 1e-6);
    }
}

#[test]
fn mcf_algorithms_agree() {
    for seed in 0..15u64 {
        let g = random_graph(6, 10, seed);
        let s = g.source().unwrap();
        let t = g.sink().unwrap();

        let ssp = SuccessiveShortest.min_cost_flow(&g, s, t, None).unwrap();
        let ns  = NetworkSimplex.min_cost_flow(&g, s, t, None).unwrap();
        let cc  = CycleCanceling.min_cost_flow(&g, s, t, None).unwrap();

        // all must agree on flow value
        assert!(
            (ssp.max_flow - ns.max_flow).abs() < 1e-5,
            "flow disagrees SSP vs NS on seed={seed}"
        );
        assert!(
            (ssp.max_flow - cc.max_flow).abs() < 1e-5,
            "flow disagrees SSP vs CC on seed={seed}"
        );

        // all must agree on minimum cost
        assert!(
            (ssp.min_cost - ns.min_cost).abs() < 1e-4,
            "cost disagrees SSP={} NS={} on seed={seed}", ssp.min_cost, ns.min_cost
        );
        assert!(
            (ssp.min_cost - cc.min_cost).abs() < 1e-4,
            "cost disagrees SSP={} CC={} on seed={seed}", ssp.min_cost, cc.min_cost
        );
    }
}

#[test]
fn capacity_scaling_handles_large_capacities() {
    // Dinic would need many augmentations; capacity scaling handles this efficiently.
    let (b, s) = GraphBuilder::new().add_stanchion("s");
    let (b, m) = b.add_stanchion("m");
    let (b, t) = b.add_stanchion("t");
    let g = b.source(s).sink(t)
        .connect(s, m, Capacity(1_000_000.0), Cost(1.0))
        .connect(m, t, Capacity(999_999.0),   Cost(1.0))
        .build().unwrap();

    let FlowResult { max_flow: d, .. } = Dinic.max_flow(&g, s, t).unwrap();
    let FlowResult { max_flow: c, .. } = CapacityScaling.max_flow(&g, s, t).unwrap();
    assert_relative_eq!(d, c, epsilon = 1e-6);
    assert_relative_eq!(c, 999_999.0, epsilon = 1e-6);
}

#[test]
fn zero_capacity_graph_gives_zero_flow() {
    let (b, s) = GraphBuilder::new().add_stanchion("s");
    let (b, t) = b.add_stanchion("t");
    let g = b.source(s).sink(t)
        .connect(s, t, Capacity(0.0), Cost(1.0))
        .build().unwrap();
    let r = Dinic.max_flow(&g, s, t).unwrap();
    assert_relative_eq!(r.max_flow, 0.0, epsilon = 1e-9);
}
