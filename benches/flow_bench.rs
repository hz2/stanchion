use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use rand::{Rng, SeedableRng, rngs::SmallRng};

use stanchion::{
    flow::{MaxFlowSolver, dinic::Dinic, push_relabel::PushRelabel, scaling::CapacityScaling},
    graph::{Capacity, Cost, GraphBuilder, NodeId},
};

/// Build a random sparse graph with `n` nodes, `e` forward arcs, and random
/// capacities in [1, 10].  Source = node 0, sink = node n-1.
fn random_graph(n: usize, e: usize, seed: u64) -> stanchion::graph::DiGraph {
    let mut rng = SmallRng::seed_from_u64(seed);
    let mut b   = GraphBuilder::new();

    for i in 0..n {
        let (nb, _) = b.add_stanchion(format!("n{i}").as_str());
        b = nb;
    }

    b = b.source(NodeId(0)).sink(NodeId((n - 1) as u32));

    for _ in 0..e {
        let u   = rng.random_range(0..n as u32);
        let v   = rng.random_range(0..n as u32);
        let cap = rng.random_range(1..=10) as f64;
        if u != v {
            b = b.connect(NodeId(u), NodeId(v), Capacity(cap), Cost(1.0));
        }
    }

    b.build().unwrap_or_else(|_| {
        // fallback: add a direct source-to-sink arc so the graph is always valid
        let (b2, _) = GraphBuilder::new().add_stanchion("s");
        let (b2, _) = b2.add_stanchion("t");
        b2.source(NodeId(0)).sink(NodeId(1))
            .connect(NodeId(0), NodeId(1), Capacity(1.0), Cost(1.0))
            .build().unwrap()
    })
}

fn bench_dinic(c: &mut Criterion) {
    let mut group = c.benchmark_group("max_flow/dinic");
    for n in [50usize, 100, 200] {
        let g = random_graph(n, n * 3, 42);
        group.bench_with_input(BenchmarkId::from_parameter(n), &g, |b, graph| {
            b.iter(|| {
                let src = graph.source().unwrap();
                let snk = graph.sink().unwrap();
                Dinic.max_flow(graph, src, snk).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_push_relabel(c: &mut Criterion) {
    let mut group = c.benchmark_group("max_flow/push_relabel");
    for n in [50usize, 100, 200] {
        let g = random_graph(n, n * 3, 42);
        group.bench_with_input(BenchmarkId::from_parameter(n), &g, |b, graph| {
            b.iter(|| {
                let src = graph.source().unwrap();
                let snk = graph.sink().unwrap();
                PushRelabel.max_flow(graph, src, snk).unwrap()
            })
        });
    }
    group.finish();
}

fn bench_capacity_scaling(c: &mut Criterion) {
    let mut group = c.benchmark_group("max_flow/capacity_scaling");
    for n in [50usize, 100, 200] {
        // use larger capacities to stress the scaling advantage
        let g = random_graph_large_cap(n, n * 3, 42);
        group.bench_with_input(BenchmarkId::from_parameter(n), &g, |b, graph| {
            b.iter(|| {
                let src = graph.source().unwrap();
                let snk = graph.sink().unwrap();
                CapacityScaling.max_flow(graph, src, snk).unwrap()
            })
        });
    }
    group.finish();
}

/// Random graph with large capacities (1..=1000) to favour capacity scaling.
fn random_graph_large_cap(n: usize, e: usize, seed: u64) -> stanchion::graph::DiGraph {
    use rand::Rng;
    let mut rng = SmallRng::seed_from_u64(seed ^ 0xdead_beef);
    let mut b   = GraphBuilder::new();
    for i in 0..n {
        let (nb, _) = b.add_stanchion(format!("n{i}").as_str());
        b = nb;
    }
    b = b.source(NodeId(0)).sink(NodeId((n - 1) as u32));
    for _ in 0..e {
        let u   = rng.random_range(0..n as u32);
        let v   = rng.random_range(0..n as u32);
        let cap = rng.random_range(1..=1000) as f64;
        if u != v {
            b = b.connect(NodeId(u), NodeId(v), Capacity(cap), Cost(1.0));
        }
    }
    b.build().unwrap_or_else(|_| {
        let (b2, _) = GraphBuilder::new().add_stanchion("s");
        let (b2, _) = b2.add_stanchion("t");
        b2.source(NodeId(0)).sink(NodeId(1))
            .connect(NodeId(0), NodeId(1), Capacity(1.0), Cost(1.0))
            .build().unwrap()
    })
}

criterion_group!(benches, bench_dinic, bench_push_relabel, bench_capacity_scaling);
criterion_main!(benches);
