# stanchion

A Rust network flow simulator and stanchion queue optimizer.

Stanchion posts and retractable belts form directed networks where people
flow from entry to service.  This library models those networks using
classical network flow, queueing theory, and dynamic flow algorithms.

## Algorithms

**Max-flow**
- Dinic (O(V^2 E); O(E sqrt(V)) for unit graphs)
- Push-relabel FIFO with gap heuristic (O(V^2 sqrt(E)))
- Capacity scaling (O(m^2 log U); handles large integer capacities)

**Min-cost flow**
- Successive shortest paths with Johnson potentials
- Network simplex (best practical MCF; spanning-tree pivots)
- Cycle canceling via Bellman-Ford negative-cycle detection

**Routing**
- Dijkstra (O((V+E) log V); non-negative costs)
- Bellman-Ford (O(VE); handles negative costs, detects negative cycles)
- Floyd-Warshall all-pairs shortest paths (O(V^3))
- All simple s-t paths (DFS with cap)

**Queueing**
- Jackson network steady-state analysis (traffic equations, M/M/1)
- Event-driven simulation with Poisson arrivals and exponential service

## Getting started

```sh
# dev shell (latest nightly via fenix)
nix develop

# tests
cargo test

# clippy
cargo clippy --all-targets -- -D warnings

# benchmarks
cargo bench

# Kani formal verification
nix develop .#kani
cargo kani
```

## Configuration

Networks can be described in JSON:

```json
{
  "nodes": [
    { "id": 0, "label": "entry",    "service_rate": 10.0 },
    { "id": 1, "label": "lane_a",   "service_rate":  3.0 },
    { "id": 2, "label": "security", "service_rate":  5.0 }
  ],
  "edges": [
    { "from": 0, "to": 1, "capacity": 3.0, "cost": 1.0 },
    { "from": 1, "to": 2, "capacity": 3.0, "cost": 1.0 }
  ],
  "source": 0,
  "sink": 2,
  "simulation": {
    "max_time": 1000.0,
    "arrival_rate": 2.0,
    "seed": 42
  }
}
```

Load and run:

```rust
use stanchion::config::NetworkConfig;
use stanchion::flow::{MaxFlowSolver, dinic::Dinic};

let cfg = NetworkConfig::from_file("network.json")?;
let net = cfg.build()?;
let s = net.graph.source().unwrap();
let t = net.graph.sink().unwrap();
let result = Dinic.max_flow(&net.graph, s, t)?;
println!("max flow: {}", result.max_flow);
```

## Formal verification

Core graph and flow invariants are verified with [Kani](https://github.com/model-checking/kani):

- XOR back-edge pairing (forward at even index, back at `i ^ 1`)
- `push_flow` antisymmetry (pushing `d` on arc, `-d` on back-edge)
- Fresh residual capacity equals original capacity
- `is_forward_edge` correctly identifies even-indexed edges
- Source/sink presence checked at build time

Run proofs:

```sh
nix develop .#kani
cargo kani
```

## Project layout

```
src/
  graph/        DiGraph, GraphBuilder, ResidualGraph
  flow/         MaxFlowSolver trait + Dinic, PushRelabel, CapacityScaling
  flow/min_cost MinCostFlowSolver + SSP, NetworkSimplex, CycleCanceling
  routing/      Dijkstra, Bellman-Ford, Floyd-Warshall
  queue/        JacksonNetwork, traffic equations
  sim/          event-driven SimulationEngine
  scheduling/   MaxWeightScheduler (Tassiulas-Ephremides)
  dynamic/      time-expanded graph for dynamic flows
  opt/          min-cut identification, stanchion placement decisions
  config/       JSON config load/build
  kani_proofs/  Kani formal verification harnesses
doc/
  theory.md     mathematical background for all algorithms
```

## References

- Ahuja, Magnanti, Orlin. *Network Flows*. Prentice Hall, 1993.
- Williamson. *Network Flow Algorithms*. Cambridge, 2019.
- MIT 6.854 Advanced Algorithms lecture notes.
- Tassiulas and Ephremides (1992). Max-weight scheduling and network stability.
