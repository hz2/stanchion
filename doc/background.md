# Background: everything you need to understand stanchion

This document builds understanding from the physical domain up through the
mathematics, the algorithms, and the Rust implementation. `doc/theory.md` is
the condensed mathematical reference; this is the explanatory layer.

---

## 1. The physical domain

A stanchion is the upright post in a queue management system. Retractable
belts connect posts to form corridors that direct pedestrian flow. You see
them in airport security, bank lobbies, theme parks.

The network we model has:
- **Posts (nodes)**: physical stanchion positions. One is the entry point
  (source), one is the service point (sink).
- **Belts (edges)**: directed connections. Capacity is how many people per
  unit time can move along that corridor. Cost is transit time or discomfort
  (congestion, long detours).
- **Flow**: the movement of people through the network over time.

The two operational questions this repo answers:
1. When should you connect two posts (add a belt)?
2. When should you remove a belt?

Both are answered by solving max-flow and min-cost flow on the graph.

---

## 2. Graph theory foundations

### Directed graphs

A directed graph G = (V, E) is a set of nodes V and directed arcs E. Each
arc e = (u, v) goes FROM u TO v. In stanchion terms, people walk in the
direction the arc points.

We represent this with an adjacency list: `adj[u]` is the list of indices
into the edge array for arcs leaving u.

### The XOR back-edge trick

This is the most important implementation detail and everything depends on it.

For every forward arc at index i (always even), its **residual back-arc**
lives at index i ^ 1 = i + 1 (always odd). So:

```
edges[0]: s -> t, cap=3   (forward)
edges[1]: t -> s, cap=0   (back-arc, zero initial capacity)
edges[2]: s -> m, cap=5   (forward)
edges[3]: m -> s, cap=0   (back-arc)
```

When we push `d` units of flow on arc `i`, we simultaneously:
- increase flow on `edges[i]` by d (consuming capacity)
- decrease flow on `edges[i^1]` by d (adding "cancelable" capacity on back-arc)

This single operation with `flow[i] += d; flow[i^1] -= d` correctly maintains
the residual graph without any separate data structure. The residual capacity
of arc `i` is always `edges[i].capacity - flow[i]`.

The back-arc's cost is the negation of the forward arc's cost. This is
required for min-cost flow: "canceling" one unit of flow on a forward arc of
cost w is equivalent to sending one unit on the back-arc at cost -w.

Why is the forward arc always at an even index? Because we add arcs in pairs
(forward + back) and start with an empty edge list. So index 0, 2, 4, ... are
always forward arcs. The predicate `is_forward_edge` just checks `id % 2 == 0`.

The `forward_neighbors()` iterator filters to even indices only. This is
critical for routing algorithms (Dijkstra, Bellman-Ford): back-arcs have
negative costs from the XOR construction, which would break Dijkstra's
non-negative weight assumption.

---

## 3. Maximum flow

### The core problem

Find an assignment of flow values f(e) to each arc such that:
- f(e) <= c(e) for all e (capacity constraint)
- flow into v = flow out of v for all v != s, t (conservation)
- the total flow from s to t is maximised

### The residual graph

For a flow f, the residual graph G_f contains:
- Forward arc (u,v) with remaining capacity c(u,v) - f(u,v) > 0
- Back-arc (v,u) with capacity f(u,v) > 0 (we can cancel that much flow)

The XOR construction gives us G_f automatically: the residual capacity of
arc i is `edges[i].capacity - flow[i]`, whether i is a forward or back-arc.

### Max-flow min-cut theorem

The maximum flow value equals the minimum cut capacity. A cut (S, T) is a
partition of V with s in S and t in T; its capacity is the sum of capacities
of arcs going from S to T.

Physical meaning: the tightest bottleneck in the stanchion layout is a set
of belts whose total capacity equals the maximum throughput. Removing those
belts disconnects entry from exit.

### Why three max-flow algorithms?

Each has a different sweet spot:

**Dinic**: general workhorse. O(V^2 E) general, O(E sqrt(V)) for unit
capacities. Works by finding shortest augmenting paths via BFS (building a
"level graph") then finding a blocking flow in that level graph via DFS. The
key insight is that after a blocking flow, the shortest augmenting path gets
longer by at least 1, so there are at most V-1 BFS phases.

**Push-relabel**: better for dense graphs. O(V^2 sqrt(E)) with FIFO and gap.
Instead of augmenting along paths, it maintains a "preflow" (excess at nodes
is allowed) and pushes flow locally from higher-height nodes to lower-height
nodes. The height function h satisfies h(u) <= h(v) + 1 for all residual
arcs (u,v). This turns a global problem (find a path) into a local one (push
downhill). The gap heuristic prunes nodes that are disconnected from the sink.

**Capacity scaling**: best when capacities are large integers. O(m^2 log U)
where U is the max capacity. Augments only along arcs with residual capacity
>= delta, halving delta each phase. The bound comes from the fact that at
most 2m augmentations happen per phase (each pushes at least delta, and the
residual value at the start of a phase is at most 2*m*delta). Avoids the
O(U) augmentations that naive Ford-Fulkerson might need for large U.

---

## 4. Minimum cost flow

### The problem

Among all max-flows (or flows of a given value), find the one with minimum
total cost. Cost on arc (u,v) is w(u,v); total cost is sum of w(e)*f(e).

### Optimality condition: no negative cycles

A feasible flow is optimal iff the residual graph contains no negative-cost
cycle. Intuitively: if there were a negative cycle, we could push flow around
it and reduce total cost.

### Reduced costs and potentials

A potential function pi: V -> R transforms arc costs into "reduced costs":

    w_pi(u,v) = w(u,v) + pi(u) - pi(v)

If we augment along shortest paths and maintain pi[v] += dist[v] after each
augmentation, all reduced costs in the residual graph stay non-negative. This
is Johnson's reweighting trick, and it lets us use Dijkstra (which requires
non-negative weights) instead of Bellman-Ford for each subsequent phase.

Why reduced costs work: the potential transformation is conservative -- any
cycle's reduced-cost total equals its original cost total (potentials cancel).
So the "no negative reduced-cost cycle" condition is equivalent to the original
optimality condition.

### Three MCF algorithms

**Successive Shortest Paths (SSP)**: find the cheapest augmenting s-t path
(Dijkstra + potentials), push flow, repeat. O(F * (E + V log V)) where F is
total flow value. Simple and correct. Needs initial Bellman-Ford to handle
any negative-cost arcs in the original graph (SSP via Dijkstra only works
with non-negative reduced costs).

**Network Simplex**: maintains a spanning tree T of the residual graph. The
tree defines unique potentials (pi[v] satisfies w_pi(u,v) = 0 for all tree
arcs). Each pivot: find a non-tree arc with negative reduced cost, add it to
T forming a cycle, push along the cycle, remove the arc that hits its
capacity/lower-bound. Update tree and potentials. Best practical MCF
algorithm -- O(nm log n) empirically, though no known strongly polynomial
bound.

**Cycle Canceling**: first find a max-flow (ignoring costs), then repeatedly
find negative-cost cycles in the residual graph (via Bellman-Ford) and push
flow around them. Simplest to understand and verify correct; slowest in
practice (pseudo-polynomial). Good for formal verification targets.

---

## 5. Routing algorithms

These work on the original graph (not the residual graph) and find shortest
paths by arc cost. The key distinction from flow algorithms: routing ignores
capacity and uses forward arcs only.

**Why forward_neighbors()**: the back-arcs in our edge array have negated
costs (e.g., cost -1 for a forward arc of cost 1). If Dijkstra traverses
back-arcs, it finds paths with negative cost and breaks completely.
`forward_neighbors()` filters to even-indexed arcs only.

**Dijkstra**: O((V+E) log V) with a binary heap. Requires non-negative arc
costs. Works by greedily finalising nodes in order of distance from source.
The priority queue holds (distance, node) pairs; stale entries are discarded
by checking if the popped distance exceeds the recorded best.

**Bellman-Ford**: O(VE). Works with negative costs (but not negative cycles).
Relaxes all arcs V-1 times (sufficient because the shortest path has at most
V-1 arcs). If any arc can still be relaxed on round V, a negative cycle exists.
Used in SSP for initial potential computation.

**Floyd-Warshall**: O(V^3) all-pairs. Iterates over all intermediate nodes k:
for each pair (u,v), check if going via k improves the u->v distance. Works
for negative costs, fails if negative cycles exist.

**All simple paths**: DFS with a visited set. Exponential worst-case, but
useful for small graphs or with a cap on the number of paths returned.

---

## 6. Queueing theory

### Poisson arrivals

People arrive at the entry post according to a Poisson process with rate
lambda (people per unit time). The Poisson process is memoryless: the number
of arrivals in any interval is independent of arrivals in other intervals.
Interarrival times are exponentially distributed with mean 1/lambda.

### Exponential service

A stanchion node (server) takes an exponential amount of time to process each
person, with rate mu (people per unit time). Exponential service is also
memoryless: the remaining service time of the person currently being served is
independent of how long they have already been served.

### M/M/1 queue

"M/M/1" means: Markovian (Poisson) arrivals, Markovian (exponential) service,
1 server. This is the simplest non-trivial queue.

Utilisation rho = lambda/mu. The queue is stable iff rho < 1.

Steady-state distribution: P(n customers in system) = (1 - rho) * rho^n.
This is a geometric distribution. Intuitively, each additional customer reduces
probability by a factor of rho.

Mean queue length: L = rho / (1 - rho).
Mean sojourn time: W = L / lambda = 1 / (mu - lambda).

At rho = 0.5: L = 1. At rho = 0.9: L = 9. Queue length blows up as rho -> 1.

### Little's Law

L = lambda * W for any stable queueing system, regardless of arrival or
service distributions. This is a conservation law: rate in * time spent =
number in system. Used constantly to convert between L, lambda, and W.

### Jackson networks

A network of M/M/1 queues where customers route probabilistically between
nodes. After service at node i, a customer goes to node j with probability
r_ij, or exits with probability 1 - sum_j r_ij.

**Traffic equations**: the total arrival rate lambda_i at node i satisfies

    lambda_i = gamma_i + sum_j lambda_j * r_ji

where gamma_i is the external arrival rate at node i. In matrix form:
(I - R^T) * lambda = gamma. This is a linear system solved by Gaussian
elimination.

**Jackson's theorem**: the joint queue-length distribution factors as a
product of independent M/M/1 distributions:

    P(n_1, ..., n_M) = product_i (1 - rho_i) * rho_i^{n_i}

This is remarkable: even though queues share customers and influence each
other, in steady state each behaves as if it were an independent M/M/1 with
its own arrival rate lambda_i. The routing matrix enters only through the
traffic equations that determine lambda_i.

The network is stable iff rho_i < 1 for all i.

### Connection to network flow

The traffic equations are a network flow problem. lambda_i is the "flow"
through node i; gamma_i is external injection; r_ij is the routing fraction
on edge (i,j). Flow balance at each node gives the traffic equation. The
Jackson network is a probabilistic flow network with stochastic routing.

---

## 7. Dynamic flows and time-expanded graphs

### The problem

In static flow, all movement is instantaneous. In reality, walking from post
to post takes time. We want to route people efficiently over a time horizon T.

### Time-expanded graph

Create T copies of each node: (v, t) for t = 0, ..., T-1.
- For each arc (u,v) with transit time d: add arc ((u,t), (v,t+d)) for all t
  where t+d < T.
- For each node v: add holding arc ((v,t), (v,t+1)) with infinite capacity
  (waiting is always allowed).

Running any static max-flow algorithm on this expanded graph gives the maximum
dynamic flow over horizon T. The time-expanded graph has T*|V| nodes and
roughly T*|E| arcs, so it's much larger -- but the same algorithms apply.

### Earliest arrival flow

A stronger optimality: the cumulative flow arriving at t by time tau is
maximised simultaneously for all tau <= T. This is a much harder problem but
possible in single-commodity networks.

---

## 8. Max-weight scheduling

### The problem

At each time slot, the scheduler chooses which queue (stanchion node) to
serve. The goal is to keep all queues stable (bounded expected length) while
maximising throughput.

### The max-weight rule

At time t, serve the node i* that maximises:

    i* = argmax_i  w_i * Q_i(t)

where Q_i(t) is the current queue length at node i and w_i is a priority
weight. For equal weights this is "longest queue first."

### Throughput optimality (Tassiulas-Ephremides 1992)

This rule is throughput-optimal: if ANY scheduler can stabilise the network
under a given arrival rate vector lambda, the max-weight scheduler also
stabilises it. This is the strongest possible stability guarantee.

**Proof sketch via Lyapunov drift**: define the quadratic Lyapunov function

    L(Q) = sum_i Q_i^2

The drift at time t is E[L(Q(t+1)) - L(Q(t)) | Q(t)]. Max-weight minimises
this drift (it greedily minimises the expected increase in sum of squared
queue lengths). By Foster-Lyapunov theory, a negative drift outside a compact
set implies positive recurrence (stability).

### Stanchion interpretation

For a single-server system (only one post can be served at once), max-weight
reduces to: "serve the most-backed-up lane, weighted by priority." This is
exactly what a supervisor does intuitively when managing a stanchion queue.

---

## 9. Rust implementation architecture

### Trait hierarchy

The library is built around two solver traits:

```rust
trait MaxFlowSolver {
    fn max_flow(&self, graph, source, sink) -> Result<FlowResult>;
}
trait MinCostFlowSolver {
    fn min_cost_flow(&self, graph, source, sink, demand) -> Result<MinCostFlowResult>;
}
```

Implementations (Dinic, PushRelabel, CapacityScaling, etc.) are zero-sized
structs that implement these traits. There is no `Box<dyn Solver>` in the hot
path -- generics give zero-cost dispatch. The simulation engine and optimizer
are also generic over solver type.

### DiGraph and ResidualGraph

`DiGraph` is immutable once built. It owns:
- `nodes: Vec<NodeData>` -- labels, service rates
- `edges: Vec<Edge>` -- all arcs including back-arcs (XOR paired)
- `adj: Vec<Vec<usize>>` -- adjacency lists of edge indices

`ResidualGraph<'g>` borrows `&'g DiGraph` and owns `flow: Vec<f64>`. All
algorithm mutation happens through `ResidualGraph`. This design lets the same
`DiGraph` be used concurrently for routing queries while a solver runs on its
own `ResidualGraph`.

`push_flow(i, d)` does: `flow[i] += d; flow[i^1] -= d`. That single line is
the entire residual graph update.

### GraphBuilder

Fluent builder pattern. Each method takes `self` by value and returns the
modified builder, enabling method chaining:

```rust
let g = GraphBuilder::new()
    .add_stanchion("entry")  // returns (builder, NodeId)
    ...
```

`build()` validates source and sink are set before returning `DiGraph`.

### Error handling

Single `StanchionError` enum using `thiserror`. All public functions return
`Result<T, StanchionError>`. Error variants include `NoSource`, `NoSink`,
`InfeasibleDemand`, `ConfigIo`, `ConfigParse`.

### Performance choices

- `rustc-hash` (FxHashMap/FxHashSet) for any hash maps in hot paths
- `VecDeque::with_capacity(n)` for BFS queues to avoid reallocation
- `#[inline]` on `push_flow`, `residual_capacity`, `edge_cost`, `neighbors`
- Release profile: `lto = "thin"`, `codegen-units = 1` for better
  inter-procedural optimisation
- `DischargeCtx` struct in push-relabel groups mutable slice references to
  avoid Rust's borrow checker complaints about multiple `&mut` borrows, and
  also satisfies clippy's "too many arguments" lint

### Config system

`NetworkConfig` is a serde-deserializable struct. `from_json_str` parses JSON;
`build()` constructs the `DiGraph` and extracts service rates, arrival rates,
and routing matrices for Jackson/simulation use. The method is called
`from_json_str` (not `from_str`) to avoid a clippy lint that warns when a
method named `from_str` does not implement the `FromStr` trait.

---

## 10. Simulation engine

The event-driven simulation uses a priority queue of timed events:
- `Arrival`: a new person enters the network at the source
- `Departure`: a person finishes service and either routes to the next node
  or exits at the sink

Time is `OrderedFloat<f64>` wrapped in a newtype so it can be put in a
`BinaryHeap` (which requires `Ord`, which `f64` doesn't implement because of
NaN).

The simulation validates against Jackson theory: run for long enough (e.g.,
10,000 time units) and the empirical throughput should match the theoretical
Jackson lambda within ~5%.

---

## 11. Formal verification

### Kani (model checking)

Kani compiles normal Rust to CBMC (a C model checker) and exhaustively
explores all execution paths up to a bounded depth. It handles floats via
bitvector models. Harnesses use `kani::any()` to introduce symbolic inputs
and `kani::assume()` to constrain them.

What we verify in `src/kani_proofs.rs`:
- XOR back-edge pairing: for any valid capacity and cost, the second edge in
  the pair reverses the first and has zero capacity and negated cost
- `push_flow` antisymmetry: pushing d on forward arc, residual capacity drops
  by d; back-arc residual rises by d
- Fresh residual capacity equals original capacity
- `is_forward_edge` returns true iff index is even
- `build()` without source/sink returns errors

Kani runs at test time: `cargo kani`. The CI job runs selected harnesses.

### Verus (SMT-based deductive verification)

Verus is a modified rustc that understands `verus! { }` blocks containing
spec functions, proof functions, and requires/ensures clauses. Z3 proves
properties for all inputs, unboundedly.

Status: not yet integrated. The Verus binary needs to be built from source
Float arithmetic is not feasible in
Verus -- Z3's float theory is incomplete. Good targets once built:
- XOR pairing as an unbounded proof (complement to Kani's bounded check)
- Adjacency list index bounds
- `is_forward_edge` <=> even index
- Height function validity invariant in push-relabel

---

## 12. How the pieces connect

```
JSON config
    |
    v
NetworkConfig::build()
    |-- DiGraph (nodes + XOR-paired edges)
    |-- service_rates (for Jackson)
    |-- routing matrix (for Jackson + simulation)
    |-- SimConfig (for SimulationEngine)
    |
    +-- flow algorithms (MaxFlowSolver / MinCostFlowSolver)
    |       DiGraph -> ResidualGraph -> FlowResult
    |
    +-- routing algorithms
    |       DiGraph (forward edges only) -> RouteResult
    |
    +-- Jackson analysis
    |       JacksonNetwork -> traffic equations -> SteadyStateResult
    |       (validates simulation convergence)
    |
    +-- SimulationEngine
    |       event-driven BinaryHeap -> SimStats
    |       (should converge to Jackson steady state)
    |
    +-- OptimizationEngine
    |       FlowResult -> min-cut -> Vec<StanchionDecision>
    |       (which belts to add or remove)
    |
    +-- MaxWeightScheduler
            QueueState -> which node to serve next
            (throughput-optimal scheduling)
```

The central abstraction is `DiGraph`. Everything else is stateless computation
on top of it: solvers borrow it immutably, ResidualGraph layers mutable flow
state on top of it, routing reads its forward edges, Jackson reads node
service rates, simulation uses its topology to route departing customers.

---

## 13. Key references

- Ahuja, Magnanti, Orlin. *Network Flows*. Prentice Hall, 1993.
  Definitive textbook. Chapters 1-5 cover everything in this repo.
- Williamson. *Network Flow Algorithms*. Cambridge, 2019.
  Modern treatment. Chapter 2.6 is capacity scaling; chapters 5.1-5.3
  are cycle canceling. Located at ~/Downloads/network-flows.pdf.
- MIT 6.854 lecture notes (2008, Karger).
  Lectures 2-5 cover max-flow and min-cost flow.
  https://ocw.mit.edu/courses/6-854j-advanced-algorithms-fall-2008/pages/lecture-notes/
- MIT network algorithms scribe notes (Shah et al.).
- Tassiulas and Ephremides (1992). "Stability properties of constrained
  queueing systems and scheduling policies for maximum throughput in multihop
  radio networks." IEEE TAC 37(12).
- Kleinrock. *Queueing Systems* vol. 1-2. Wiley, 1975.
  M/M/1 derivation and Little's Law.
