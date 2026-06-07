# stanchion

A Rust network flow simulator and stanchion queue optimizer.

Stanchion posts and retractable belts form directed networks where people
flow from entry to service. This library models those networks using
classical network flow, queueing theory, and dynamic flow algorithms,
and answers two operational questions: when should you connect two posts
(add a belt), and when should you remove one?

## Algorithms

**Max-flow** &mdash; find $f^* = \max |f|$ subject to capacity and conservation

| Algorithm | Complexity | Notes |
|---|---|---|
| Dinic | $O(V^2 E)$; $O(E\sqrt{V})$ unit graphs | general workhorse |
| Push-relabel (FIFO + gap) | $O(V^2\sqrt{E})$ | better on dense graphs |
| Capacity scaling | $O(m^2 \log U)$ | best when $U \gg V$ |

**Min-cost flow** &mdash; minimise $\sum_e w(e)\,f(e)$ subject to $|f| = F^*$

| Algorithm | Complexity | Notes |
|---|---|---|
| Successive shortest paths | $O(F(E + V\log V))$ | Dijkstra + Johnson potentials |
| Network simplex | $O(nm\log n)$ empirical | best in practice |
| Cycle canceling | pseudo-polynomial | simplest to verify |

**Routing** &mdash; shortest paths on the original arc-cost graph

- Dijkstra: $O((V+E)\log V)$, non-negative costs
- Bellman-Ford: $O(VE)$, handles negative costs, detects negative cycles
- Floyd-Warshall: $O(V^3)$ all-pairs
- All simple $s$-$t$ paths via DFS

**Queueing**

- Jackson network steady-state: solve $(I - R^\top)\lambda = \gamma$, apply product-form theorem
- Event-driven simulation: Poisson arrivals, exponential service, validates against Jackson $L = \rho/(1-\rho)$

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

Networks are described in JSON. Each node carries a service rate $\mu_i$;
each edge carries a capacity $c_{ij}$ and cost $w_{ij}$.

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

## Theory

The core result is the **max-flow min-cut theorem**:

$$\max_f |f| \;=\; \min_{(S,T)} \sum_{\substack{(u,v)\in E \\ u\in S,\, v\in T}} c(u,v)$$

The min-cut identifies saturated belts; adding capacity to any cut arc
directly raises throughput.

A flow is **min-cost** iff the residual graph $G_f$ contains no negative-cost
cycle. The reduced cost under potentials $\pi$ is

$$\bar{w}(u,v) = w(u,v) + \pi(u) - \pi(v) \ge 0$$

for all arcs in $G_f$ at optimality (complementary slackness).

For the stochastic layer, Jackson's theorem gives the product-form steady state
of an open queueing network: each node $i$ with utilisation $\rho_i = \lambda_i/\mu_i < 1$
behaves as an independent $M/M/1$ queue with mean length

$$L_i = \frac{\rho_i}{1 - \rho_i}, \qquad W_i = \frac{1}{\mu_i - \lambda_i}$$

where $\lambda_i$ solves the traffic equations
$\lambda_i = \gamma_i + \sum_j \lambda_j r_{ji}$.

The max-weight scheduler at each slot picks

$$i^* = \arg\max_i\; w_i Q_i(t)$$

and is throughput-optimal by the Lyapunov drift argument of
Tassiulas and Ephremides (1992).

Full derivations are in `doc/theory.md` and `doc/background.md`.

## Formal verification

Core invariants are verified with [Kani](https://github.com/model-checking/kani):

- XOR back-edge pairing: forward arc at index $i$ (even), back-arc at $i \oplus 1$
- `push_flow` antisymmetry: pushing $\delta$ on arc $i$ and $-\delta$ on $i \oplus 1$
- Residual capacity of a fresh graph equals $c(e)$
- `is_forward_edge` $\iff$ even index
- `build()` without source or sink returns an error

```sh
nix develop .#kani
cargo kani
```

## Project layout

```
src/
  graph/        DiGraph, GraphBuilder, ResidualGraph (XOR back-edge store)
  flow/         MaxFlowSolver + Dinic, PushRelabel, CapacityScaling
  flow/min_cost MinCostFlowSolver + SSP, NetworkSimplex, CycleCanceling
  routing/      Dijkstra, Bellman-Ford, Floyd-Warshall, all simple paths
  queue/        JacksonNetwork, traffic equations, steady-state analysis
  sim/          event-driven SimulationEngine, Poisson/exponential processes
  scheduling/   MaxWeightScheduler (Tassiulas-Ephremides)
  dynamic/      time-expanded graph for flows with transit times
  opt/          min-cut identification, stanchion placement decisions
  config/       JSON network config (serde)
  kani_proofs   Kani verification harnesses
doc/
  theory.md     condensed mathematical reference
  background.md explanatory background, intuition, and architecture guide
examples/
  airport_security.json   6-node example network
```

## References

- Ahuja, Magnanti, Orlin. *Network Flows*. Prentice Hall, 1993.
- Williamson. *Network Flow Algorithms*. Cambridge, 2019.
- MIT 6.854 Advanced Algorithms lecture notes (Karger, 2008).
- Tassiulas and Ephremides. "Stability properties of constrained queueing systems
  and scheduling policies for maximum throughput in multihop radio networks."
  *IEEE Transactions on Automatic Control* 37(12), 1992.
