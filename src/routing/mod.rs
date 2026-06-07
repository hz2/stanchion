//! Shortest-path and routing algorithms over `DiGraph`.
//!
//! All algorithms here work on the *original* graph (costs are arc weights;
//! capacities are ignored).  For flow-aware routing see `flow::min_cost`.

use std::{cmp::Reverse, collections::BinaryHeap};

use ordered_float::OrderedFloat;

use crate::graph::{DiGraph, NodeId};

/// Result of a single-source shortest-path query.
#[derive(Debug, Clone)]
pub struct RouteResult {
    /// `distances[v]` = cheapest path cost from the source to node v.
    /// `f64::INFINITY` if unreachable.
    pub distances: Vec<f64>,
    /// `predecessors[v]` = edge index of the arc that last updated node v,
    /// used to reconstruct the path via `reconstruct_path`.
    pub predecessors: Vec<Option<usize>>,
}

/// Result of an all-pairs shortest-path query.
#[derive(Debug, Clone)]
pub struct AllPairsResult {
    /// `dist[u][v]` = cheapest path cost from u to v (`f64::INFINITY` if unreachable).
    pub dist: Vec<Vec<f64>>,
    /// `next[u][v]` = first hop node on the shortest path from u to v.
    pub next: Vec<Vec<Option<NodeId>>>,
}

/// Dijkstra's algorithm.  Requires all arc costs >= 0.
///
/// Complexity: O((V + E) log V).
pub fn dijkstra(graph: &DiGraph, source: NodeId) -> RouteResult {
    let n = graph.node_count();
    debug_assert!((source.0 as usize) < n, "source node out of bounds");
    let mut dist = vec![f64::INFINITY; n];
    let mut pred = vec![None::<usize>; n];
    dist[source.0 as usize] = 0.0;

    let mut heap: BinaryHeap<Reverse<(OrderedFloat<f64>, u32)>> = BinaryHeap::new();
    heap.push(Reverse((OrderedFloat(0.0), source.0)));

    while let Some(Reverse((d, u32_u))) = heap.pop() {
        let u = u32_u as usize;
        if d.0 > dist[u] + 1e-12 {
            continue;
        }
        for (v, idx) in graph.forward_neighbors(NodeId(u32_u)) {
            let w  = graph.edges[idx].cost.0;
            debug_assert!(w >= 0.0, "dijkstra requires non-negative arc costs; got {w}");
            let nd = dist[u] + w;
            let vi = v.0 as usize;
            if nd < dist[vi] - 1e-12 {
                dist[vi] = nd;
                pred[vi] = Some(idx);
                heap.push(Reverse((OrderedFloat(nd), v.0)));
            }
        }
    }

    RouteResult { distances: dist, predecessors: pred }
}

/// Bellman-Ford algorithm.  Handles negative arc costs.
///
/// Returns `None` if a negative-cost cycle reachable from `source` is detected.
/// Complexity: O(VE).
pub fn bellman_ford(graph: &DiGraph, source: NodeId) -> Option<RouteResult> {
    let n = graph.node_count();
    let mut dist = vec![f64::INFINITY; n];
    let mut pred = vec![None::<usize>; n];
    dist[source.0 as usize] = 0.0;

    for _ in 0..n.saturating_sub(1) {
        let mut updated = false;
        for (u, adj) in graph.adj.iter().enumerate() {
            if dist[u].is_infinite() {
                continue;
            }
            for &idx in adj {
                // skip back-edges (odd indices) since they have negative cost
                // from the XOR construction; use only original arcs
                if idx % 2 != 0 {
                    continue;
                }
                let e  = &graph.edges[idx];
                let nd = dist[u] + e.cost.0;
                let v  = e.to.0 as usize;
                if nd < dist[v] - 1e-12 {
                    dist[v] = nd;
                    pred[v] = Some(idx);
                    updated = true;
                }
            }
        }
        if !updated {
            break;
        }
    }

    // Negative cycle check: one more relaxation round
    for (u, adj) in graph.adj.iter().enumerate() {
        if dist[u].is_infinite() {
            continue;
        }
        for &idx in adj {
            if idx % 2 != 0 {
                continue;
            }
            let e  = &graph.edges[idx];
            let nd = dist[u] + e.cost.0;
            if nd < dist[e.to.0 as usize] - 1e-12 {
                return None; // negative cycle
            }
        }
    }

    Some(RouteResult { distances: dist, predecessors: pred })
}

/// Floyd-Warshall all-pairs shortest paths.
///
/// Complexity: O(V^3).  `dist[u][v]` is cheapest s-t path cost, `f64::INFINITY` if unreachable.
/// Does not handle negative cycles (result is undefined if they exist).
pub fn floyd_warshall(graph: &DiGraph) -> AllPairsResult {
    let n = graph.node_count();
    let mut dist = vec![vec![f64::INFINITY; n]; n];
    let mut next: Vec<Vec<Option<NodeId>>> = vec![vec![None; n]; n];

    for (i, row) in dist.iter_mut().enumerate() {
        row[i] = 0.0;
    }

    // initialise direct arcs (forward edges only)
    for idx in (0..graph.edges.len()).step_by(2) {
        let e = &graph.edges[idx];
        let u = e.from.0 as usize;
        let v = e.to.0 as usize;
        if e.cost.0 < dist[u][v] {
            dist[u][v] = e.cost.0;
            next[u][v] = Some(e.to);
        }
    }

    // relax via intermediate nodes
    for k in 0..n {
        for u in 0..n {
            for v in 0..n {
                let via = dist[u][k] + dist[k][v];
                if via < dist[u][v] - 1e-12 {
                    dist[u][v] = via;
                    next[u][v] = next[u][k];
                }
            }
        }
    }

    AllPairsResult { dist, next }
}

/// Reconstruct the node sequence of the shortest path from source to `sink`
/// using the predecessor arc indices from a `RouteResult`.
///
/// Returns an empty vec if `sink` is unreachable.
pub fn reconstruct_path(result: &RouteResult, graph: &DiGraph, sink: NodeId) -> Vec<NodeId> {
    if result.distances[sink.0 as usize].is_infinite() {
        return vec![];
    }
    let mut path = vec![sink];
    let mut cur  = sink;
    loop {
        match result.predecessors[cur.0 as usize] {
            None => break,
            Some(idx) => {
                let prev = graph.edges[idx].from;
                path.push(prev);
                cur = prev;
            }
        }
    }
    path.reverse();
    path
}

/// Enumerate all simple s-t paths via DFS.  Exponential in the worst case;
/// use only on small graphs or with a depth/count cap.
pub fn all_simple_paths(
    graph:     &DiGraph,
    source:    NodeId,
    sink:      NodeId,
    max_paths: usize,
) -> Vec<Vec<NodeId>> {
    let mut result  = Vec::new();
    let mut visited = vec![false; graph.node_count()];
    let mut path    = vec![source];
    visited[source.0 as usize] = true;
    dfs_paths(graph, source, sink, &mut visited, &mut path, &mut result, max_paths);
    result
}

fn dfs_paths(
    graph:   &DiGraph,
    u:       NodeId,
    sink:    NodeId,
    visited: &mut [bool],
    path:    &mut Vec<NodeId>,
    result:  &mut Vec<Vec<NodeId>>,
    cap:     usize,
) {
    if u == sink {
        result.push(path.clone());
        return;
    }
    if result.len() >= cap {
        return;
    }
    for (v, idx) in graph.neighbors(u) {
        // forward edges only
        if idx % 2 != 0 {
            continue;
        }
        let vi = v.0 as usize;
        if !visited[vi] {
            visited[vi] = true;
            path.push(v);
            dfs_paths(graph, v, sink, visited, path, result, cap);
            path.pop();
            visited[vi] = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use approx::assert_relative_eq;
    use crate::graph::{Capacity, Cost, GraphBuilder};
    use super::*;

    fn triangle() -> (DiGraph, NodeId, NodeId, NodeId) {
        // s -1-> a -2-> t
        //  \----3---->/
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, a) = b.add_stanchion("a");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a, Capacity(1.0), Cost(1.0))
            .connect(a, t, Capacity(1.0), Cost(2.0))
            .connect(s, t, Capacity(1.0), Cost(3.0))
            .build().unwrap();
        (g, s, a, t)
    }

    #[test]
    fn dijkstra_shortest_path() {
        let (g, s, _, t) = triangle();
        let r = dijkstra(&g, s);
        // both s->a->t (cost 3) and s->t (cost 3) are shortest; check distance only
        assert_relative_eq!(r.distances[t.0 as usize], 3.0, epsilon = 1e-9);
        let path = reconstruct_path(&r, &g, t);
        // path must start at s and end at t
        assert_eq!(*path.first().unwrap(), s);
        assert_eq!(*path.last().unwrap(), t);
    }

    #[test]
    fn dijkstra_unreachable() {
        // start from 'a'; there is no arc from a back to s in the forward graph
        let (b, s) = GraphBuilder::new().add_stanchion("s");
        let (b, a) = b.add_stanchion("a");
        let (b, t) = b.add_stanchion("t");
        let g = b.source(s).sink(t)
            .connect(s, a, Capacity(1.0), Cost(1.0))
            .connect(a, t, Capacity(1.0), Cost(2.0))
            .build().unwrap();
        let r = dijkstra(&g, a);
        // s is not reachable from a in the forward graph
        assert!(r.distances[s.0 as usize].is_infinite());
        assert_relative_eq!(r.distances[t.0 as usize], 2.0, epsilon = 1e-9);
    }

    #[test]
    fn bellman_ford_matches_dijkstra() {
        let (g, s, _, t) = triangle();
        let bfd = bellman_ford(&g, s).expect("no neg cycle");
        let dij = dijkstra(&g, s);
        assert_relative_eq!(
            bfd.distances[t.0 as usize],
            dij.distances[t.0 as usize],
            epsilon = 1e-9
        );
    }

    #[test]
    fn floyd_warshall_all_pairs() {
        let (g, s, a, t) = triangle();
        let ap = floyd_warshall(&g);
        assert_relative_eq!(ap.dist[s.0 as usize][t.0 as usize], 3.0, epsilon = 1e-9);
        assert_relative_eq!(ap.dist[s.0 as usize][a.0 as usize], 1.0, epsilon = 1e-9);
        assert_relative_eq!(ap.dist[a.0 as usize][t.0 as usize], 2.0, epsilon = 1e-9);
    }

    #[test]
    fn all_simple_paths_found() {
        let (g, s, _, t) = triangle();
        let paths = all_simple_paths(&g, s, t, 10);
        assert_eq!(paths.len(), 2); // s->a->t and s->t
    }
}
