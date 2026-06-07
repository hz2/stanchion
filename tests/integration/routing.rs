//! Routing algorithm integration tests.

use approx::assert_relative_eq;

use stanchion::{
    graph::{Capacity, Cost, GraphBuilder, NodeId},
    routing::{all_simple_paths, bellman_ford, dijkstra, floyd_warshall, reconstruct_path},
};

fn line_graph() -> (stanchion::graph::DiGraph, NodeId, NodeId, NodeId) {
    // s -1-> m -2-> t
    let (b, s) = GraphBuilder::new().add_stanchion("s");
    let (b, m) = b.add_stanchion("m");
    let (b, t) = b.add_stanchion("t");
    let g = b.source(s).sink(t)
        .connect(s, m, Capacity(1.0), Cost(1.0))
        .connect(m, t, Capacity(1.0), Cost(2.0))
        .build().unwrap();
    (g, s, m, t)
}

fn diamond() -> (stanchion::graph::DiGraph, NodeId, NodeId, NodeId, NodeId) {
    // s -1-> a -2-> t
    // s -4-> b -1-> t
    let (b, s)  = GraphBuilder::new().add_stanchion("s");
    let (b, a)  = b.add_stanchion("a");
    let (b, bv) = b.add_stanchion("b");
    let (b, t)  = b.add_stanchion("t");
    let g = b.source(s).sink(t)
        .connect(s, a,  Capacity(1.0), Cost(1.0))
        .connect(a, t,  Capacity(1.0), Cost(2.0))
        .connect(s, bv, Capacity(1.0), Cost(4.0))
        .connect(bv, t, Capacity(1.0), Cost(1.0))
        .build().unwrap();
    (g, s, a, bv, t)
}

#[test]
fn dijkstra_line() {
    let (g, s, m, t) = line_graph();
    let r = dijkstra(&g, s);
    assert_relative_eq!(r.distances[m.0 as usize], 1.0, epsilon = 1e-9);
    assert_relative_eq!(r.distances[t.0 as usize], 3.0, epsilon = 1e-9);
    let path = reconstruct_path(&r, &g, t);
    assert_eq!(path, vec![s, m, t]);
}

#[test]
fn dijkstra_shortest_of_two_paths() {
    let (g, s, a, _bv, t) = diamond();
    let r = dijkstra(&g, s);
    // s->a->t = 1+2 = 3; s->b->t = 4+1 = 5; shortest is s->a->t
    assert_relative_eq!(r.distances[t.0 as usize], 3.0, epsilon = 1e-9);
    let path = reconstruct_path(&r, &g, t);
    assert_eq!(path, vec![s, a, t]);
}

#[test]
fn bellman_ford_matches_dijkstra() {
    let (g, s, _, t) = line_graph();
    let bfr = bellman_ford(&g, s).expect("no negative cycle");
    let dij = dijkstra(&g, s);
    assert_relative_eq!(
        bfr.distances[t.0 as usize],
        dij.distances[t.0 as usize],
        epsilon = 1e-9,
    );
}

#[test]
fn floyd_warshall_all_pairs() {
    let (g, s, a, bv, t) = diamond();
    let ap = floyd_warshall(&g);
    let si = s.0 as usize;
    let ai = a.0 as usize;
    let bi = bv.0 as usize;
    let ti = t.0 as usize;

    assert_relative_eq!(ap.dist[si][ai], 1.0, epsilon = 1e-9);
    assert_relative_eq!(ap.dist[si][bi], 4.0, epsilon = 1e-9);
    assert_relative_eq!(ap.dist[si][ti], 3.0, epsilon = 1e-9);
    assert_relative_eq!(ap.dist[ai][ti], 2.0, epsilon = 1e-9);
    // no path back from t to s
    assert!(ap.dist[ti][si].is_infinite());
}

#[test]
fn all_simple_paths_diamond() {
    let (g, s, _, _, t) = diamond();
    let paths = all_simple_paths(&g, s, t, 10);
    assert_eq!(paths.len(), 2);
    // paths must start at s and end at t
    for p in &paths {
        assert_eq!(*p.first().unwrap(), s);
        assert_eq!(*p.last().unwrap(), t);
    }
}

#[test]
fn dijkstra_unreachable_node() {
    let (g, _, m, t) = line_graph();
    // starting from m: s is not reachable (edges are directed s->m->t)
    let r = dijkstra(&g, m);
    assert!(r.distances[NodeId(0).0 as usize].is_infinite());
    assert_relative_eq!(r.distances[t.0 as usize], 2.0, epsilon = 1e-9);
}

#[test]
fn config_roundtrip() {
    use stanchion::config::NetworkConfig;
    let json = r#"{
        "nodes": [
            {"id":0,"label":"s","service_rate":5.0},
            {"id":1,"label":"t","service_rate":5.0}
        ],
        "edges": [{"from":0,"to":1,"capacity":2.0,"cost":1.0}],
        "source":0, "sink":1
    }"#;
    let cfg = NetworkConfig::from_json_str(json).unwrap();
    let net = cfg.build().unwrap();
    assert_eq!(net.graph.node_count(), 2);
    assert_eq!(net.graph.edge_count(), 1);
}
