# Stanchion Network Flow: Mathematical Theory

Stanchion posts connected by retractable belts form a directed graph.
People enter at a source post and flow toward a service exit (sink).
This document derives the algorithms that answer two operational questions:

1. **When is it optimal to connect two posts?** (add an edge)
2. **When is it optimal to remove a belt?** (delete an edge)

---

## 1. Graph Formalism

A stanchion layout is a directed graph $G = (V, E)$ where

- $V$ is the set of posts (nodes),
- $E \subseteq V \times V$ is the set of belts (arcs),
- $c : E \to \mathbb{R}_{> 0}$ is the **capacity** function (people per unit time),
- $w : E \to \mathbb{R}$ is the **cost** function (transit time or discomfort),
- $s \in V$ is the **source** (entry post),
- $t \in V$ is the **sink** (exit / service desk).

A **flow** is a function $f : E \to \mathbb{R}$ satisfying:

**Capacity**: $0 \le f(e) \le c(e)$ for all $e \in E$.

**Conservation**: For every $v \ne s, t$,
$$\sum_{(u,v) \in E} f(u,v) = \sum_{(v,w) \in E} f(v,w).$$

The **value** of a flow is $|f| = \sum_{(s,v) \in E} f(s,v)$.

---

## 2. Maximum Flow

### 2.1 Residual Graph

For a flow $f$, the **residual graph** $G_f$ contains arc $(u,v)$ with
residual capacity $r(u,v) = c(u,v) - f(u,v) > 0$.
Back-arcs $(v,u)$ with $r(v,u) = f(u,v)$ represent the ability to cancel flow.

In code, the XOR trick stores the forward arc at index $i$ (even) and the
back-arc at $i \oplus 1$ (odd), so $r$ updates are always paired.

### 2.2 Max-flow / Min-cut Theorem (Ford–Fulkerson 1956)

$$\max_f |f| \;=\; \min_{(S,T)} \sum_{(u,v) \in E,\; u \in S,\; v \in T} c(u,v)$$

where the minimum ranges over all **cuts** $(S, T)$ with $s \in S$ and $t \in T$.

**Stanchion interpretation**: the min-cut identifies the set of belts that, if
removed, would reduce throughput.  Adding capacity to any cut arc directly
raises max-flow.

---

## 3. Dinic's Algorithm

Complexity: $O(V^2 E)$ general; $O(E \sqrt{V})$ for unit-capacity graphs.

### 3.1 Level Graph

Run BFS from $s$ in $G_f$ to assign **level labels** $\ell(v)$ = shortest-path
distance from $s$.  Keep only arcs $(u,v)$ where $\ell(v) = \ell(u) + 1$.
This sub-graph is the level graph $L_f$.

### 3.2 Blocking Flow

A **blocking flow** in $L_f$ saturates every $s$-$t$ path.
Found in $O(VE)$ via DFS with the **current-arc optimisation**: each node
maintains a pointer to its current out-arc and advances it (never backtracks)
when the arc is unusable.

### 3.3 Algorithm

```
while BFS finds a path s -> t in G_f:
    compute blocking flow B in L_f
    augment f by B
    rebuild G_f
```

Each phase increases the length of the shortest augmenting path by at least
one, so there are at most $V - 1$ phases.

---

## 4. Push-Relabel Algorithm

Complexity: $O(V^2 E)$ generic; $O(V^2 \sqrt{E})$ with FIFO selection
and gap heuristic.

### 4.1 Valid Labelling

A height function $h : V \to \mathbb{Z}_{\ge 0}$ is **valid** if

$$h(s) = n, \quad h(t) = 0, \quad h(u) \le h(v) + 1 \text{ for all } (u,v) \in E_f.$$

### 4.2 Excess and Preflow

A **preflow** allows excess at non-source/sink nodes:

$$e(v) = \sum_{(u,v)} f(u,v) - \sum_{(v,w)} f(v,w) \ge 0, \quad v \ne s,t.$$

**Push**: if $e(u) > 0$, $(u,v) \in E_f$, and $h(u) = h(v)+1$, push
$\delta = \min(e(u), r(u,v))$ units from $u$ to $v$.

**Relabel**: if $e(u) > 0$ and no admissible arc from $u$ exists,
set $h(u) = 1 + \min_{(u,v) \in E_f} h(v)$.

### 4.3 Gap Heuristic

When no node has height $k$ (for some $0 < k < h(s)$), every node with height
$> k$ is disconnected from the sink.  Relabel them all to $n+1$, cutting off
expensive computation.

---

## 5. Minimum Cost Flow

### 5.1 Problem

Minimise $\sum_{e \in E} w(e) \cdot f(e)$ subject to the capacity and
conservation constraints, with a target flow value $F^*$.

### 5.2 Reduced Costs and Optimality

Given node potentials $\pi : V \to \mathbb{R}$, the **reduced cost** of arc
$(u,v)$ is

$$\bar{w}(u,v) = w(u,v) + \pi(u) - \pi(v).$$

A flow is **optimal** if and only if in $G_f$ there is no negative-cost
augmenting cycle (negative cycle criterion), equivalently:

- For every arc at lower bound in $G_f$: $\bar{w}(u,v) \ge 0$.
- For every arc at upper bound in $G_f$: $\bar{w}(u,v) \le 0$.

### 5.3 Successive Shortest Paths (SSP)

Augment along the shortest (cheapest) $s$-$t$ path in $G_f$ until demand is
met.

**Johnson's potential trick**: after each augmentation, update potentials by
the Dijkstra distances $d(v)$:

$$\pi'(v) = \pi(v) + d(v).$$

This makes all reduced costs non-negative, allowing Dijkstra (not
Bellman–Ford) for subsequent phases.

Complexity: $O(F \cdot (E + V \log V))$ where $F$ is the integer demand.

### 5.4 Network Simplex

Maintain a **spanning tree basis** $T$ (tree arcs) and node potentials $\pi$
satisfying $\bar{w}(u,v) = 0$ for every tree arc.

**Pivot step**:
1. **Entering arc**: find a non-tree arc with $\bar{w}(u,v) < 0$ (forward)
   or $\bar{w}(u,v) > 0$ (backward/upper-bound).
2. **Fundamental cycle**: $T$ plus the entering arc forms a unique cycle.
3. **Augment** by the minimum residual capacity on the cycle.
4. **Leaving arc**: the arc whose capacity hits 0 (or flow hits 0 for back arcs).
5. **Update** the tree and recompute potentials via BFS.

Network simplex is the best practical MCF algorithm: $O(nm \log n)$ empirically.

---

## 6. Dynamic Network Flows

### 6.1 Time-Expanded Graph

Given transit times $\tau : E \to \mathbb{Z}_{\ge 0}$ and a horizon $T$,
the **time-expanded graph** $G_T$ has:

- **Nodes**: $(v, t)$ for each $v \in V$, $t \in \{0, \ldots, T-1\}$.
- **Transit arcs**: for each $(u,v) \in E$ with $\tau(u,v) = d$, add arc
  $((u,t), (v, t+d))$ for each $t$ with $t + d < T$.
- **Holding arcs**: $((v,t), (v,t+1))$ with capacity $\infty$ and cost $0$
  (waiting at a post).

Running any static max-flow algorithm on $G_T$ gives the **maximum dynamic flow**
over horizon $T$.

### 6.2 Earliest Arrival Flow

The strongest dynamic optimality: the flow arriving at $t$ by every time $\tau$
is maximised simultaneously for all $\tau \le T$.
Such flows exist for single-commodity networks and are computed in
pseudo-polynomial time.

---

## 7. Queueing Theory

### 7.1 M/M/1 Queue

Arrivals: Poisson process with rate $\lambda$.
Service: exponential with rate $\mu$.
Utilisation: $\rho = \lambda / \mu$.  Stable iff $\rho < 1$.

Steady-state queue-length distribution:
$$P(n) = (1 - \rho)\,\rho^n, \quad n = 0,1,2,\ldots$$

Mean queue length (Little's Law):
$$L = \frac{\rho}{1 - \rho}, \qquad W = \frac{L}{\lambda} = \frac{1}{\mu - \lambda}.$$

### 7.2 Little's Law

For any stable queueing system (regardless of arrival or service distributions):
$$L = \lambda W$$
where $L$ is the mean number in system, $\lambda$ is the throughput, and
$W$ is the mean sojourn time.

### 7.3 Jackson Networks

An open Jackson network of $M$ M/M/1 nodes:

- External arrivals at node $i$: Poisson with rate $\gamma_i$.
- Routing: after service at $i$, route to $j$ with probability $r_{ij}$;
  exit with probability $1 - \sum_j r_{ij}$.

**Traffic equations** (solve for the total arrival rate vector $\lambda$):
$$\lambda_i = \gamma_i + \sum_j \lambda_j \, r_{ji}, \quad i = 1,\ldots,M.$$

In matrix form: $(I - R^\top)\lambda = \gamma$.  This is the flow balance
system — a linear network flow problem.

**Jackson's theorem**: the steady-state joint distribution has the product form
$$P(n_1,\ldots,n_M) = \prod_{i=1}^{M} (1-\rho_i)\,\rho_i^{n_i}$$
where $\rho_i = \lambda_i / \mu_i < 1$ for stability.
Each queue behaves as an independent M/M/1 even though they share customers.

---

## 8. Stanchion Placement Optimization

### 8.1 When to Add a Belt (Connect Two Posts)

Run max-flow and identify the min-cut $(S, T)$.
An arc $(u, v)$ with $u \in S$ and $v \in T$ is **saturated** and limits
throughput.

**Decision**: add a parallel belt $(u, v)$ with capacity $\Delta c$.
Impact: max-flow increases by exactly $\min(\Delta c,$ remaining demand$)$.

**Budget allocation**: with budget $B$ to distribute among cut arcs, the
optimal fractional allocation is proportional to marginal throughput gain.
In the integer case, allocate to the arc with the smallest flow-to-capacity
ratio (closest to saturation).

### 8.2 When to Remove a Belt

An arc $(u, v)$ carries **zero flow** in the max-flow solution if and only if
it lies in a path that is not part of any augmenting path.  Removing it does
not reduce max-flow.

**Decision**: remove any forward arc with $f(u,v) = 0$.
This simplifies the stanchion layout without hurting throughput.

For cost optimisation, remove arcs with high $w(u,v)$ and low $f(u,v)/c(u,v)$
(underutilised and expensive).

### 8.3 Utilisation-Based Decisions

From the Jackson analysis, node $i$ is a bottleneck if $\rho_i$ is close to 1.
Route relief: add arcs that divert flow away from the overloaded node toward
alternative paths.

### 8.4 Decision Impact Formula

Let $f^*$ be the current max-flow and $f^{**}$ the max-flow after adding or
removing an arc.  The **impact** is $\Delta = f^{**} - f^*$.

For a cut arc addition: $\Delta \ge 0$ (adding capacity can only help).
For a zero-flow removal: $\Delta = 0$ (guaranteed safe pruning).

---

## 9. Capacity-Scaling Max-Flow

*Reference: Williamson (2019) Ch. 2.6; Gabow (1985).*

Maintain a scaling parameter $\delta$ that halves each phase, starting from
$\delta = 2^{\lfloor \log_2 U \rfloor}$ where $U$ is the maximum arc capacity.

**$\delta$-residual subgraph** $G_f(\delta)$: keep only arcs whose residual
capacity is at least $\delta$.

**Algorithm:**
```
delta <- 2^floor(log2 U)
while delta >= 1:
    while there exists an s-t path in G_f(delta):
        augment along path
    delta <- delta / 2
```

**Key lemma (Lemma 2.25, Williamson):** At the start of a $\delta$-phase the
residual max-flow value is at most $2m\delta$.  Each augmentation in the
phase pushes at least $\delta$ units, so there are at most $2m$ augmentations
per phase (Lemma 2.26).

With $O(\log U)$ phases and $O(m)$ BFS per augmentation:
$$\text{Complexity} = O(m^2 \log U).$$

Advantages over plain BFS augmentation: for large-capacity networks the phase
structure keeps augmentations small in number; Dinic's $O(V^2 E)$ is usually
better for unit graphs; capacity scaling is preferred when $U \gg V$.

---

## 10. Negative-Cycle Canceling Min-Cost Flow

*Reference: Williamson (2019) Ch. 5.1-5.3; Goldberg-Tarjan (1989).*

**Theorem (Optimality):** A feasible flow `f` is minimum cost if and only if
the residual graph `G_f` contains no negative-cost cycle.

### 10.1 Algorithm

1. Find a maximum flow (using Dinic, ignoring costs).
2. While `G_f` contains a negative-cost cycle `C`:
   - Push `bottleneck(C) = min_{e in C} r(e)` units around `C`.
3. Return `f`.

Each cycle cancellation strictly reduces total cost.  Termination follows
from the finite number of distinct integer flow values when capacities are
integer.

### 10.2 Cycle Detection: Bellman-Ford

Multi-source Bellman-Ford: initialise all `dist[v] = 0` (equivalent to a
virtual super-source connected to all nodes at zero cost).  Run `n-1`
relaxation rounds on residual arcs with positive capacity.  If any arc can
still be relaxed after `n` rounds, a negative cycle exists; trace parent
pointers to recover it.

Complexity per detection: `O(nm)`.  Total (pseudo-polynomial in costs):
`O(nm * n * C_max)`.

### 10.3 Minimum Mean Cycle Canceling (Goldberg-Tarjan 1989)

Instead of canceling any negative cycle, cancel the **minimum mean cost**
cycle:
$$\mu^* = \min_{\text{cycle } C} \frac{\sum_{e \in C} w(e)}{|C|}.$$

**Karp's algorithm** computes `mu*` in `O(nm)`:

Let `D[k][v]` = minimum-cost walk of exactly `k` arcs from any node to `v`
(initialise `D[0][v] = 0` for all `v`).  Then:
$$\mu^* = \min_v \max_{0 \le k < n} \frac{D[n][v] - D[k][v]}{n - k}.$$

Canceling the minimum mean cycle gives a strongly polynomial algorithm with
`O(nm \log(nC))` iterations.  The implementation in `src/flow/min_cost/cycle_canceling.rs`
uses plain Bellman-Ford (pseudo-polynomial), which is simpler and adequate
for small stanchion networks.

---

## 11. Max-Weight Scheduling

*Reference: Tassiulas-Ephremides (1992); MIT 6.266 Lecture 4 (Shah 2008).*

### 11.1 Problem

A queueing network has `n` nodes and a set `S` of **feasible schedules**
(which queues may be served simultaneously).  At each slot `t`, pick a
schedule `sigma(t) in S` to maximise weighted queue drain.

### 11.2 Max-Weight Rule

$$\sigma^*(t) = \arg\max_{\sigma \in S} \sum_{i=1}^n Q_i(t) \cdot \sigma_i$$

where `Q_i(t)` is the queue length at node `i` at time `t`.

**Throughput optimality (Tassiulas-Ephremides):** If any scheduling algorithm
can stabilise the network for arrival rate `lambda`, then the max-weight
algorithm also stabilises it.

**Lyapunov proof:** Define `L(Q) = sum_i Q_i^2`.  The MW rule minimises the
Lyapunov drift `E[L(Q(t+1)) - L(Q(t)) | Q(t)]` at each step, keeping the
chain positive-recurrent.

The drift bound gives (for `epsilon`-interior capacity):
$$E\!\left[\sum_i Q_i^2(\infty)\right] \le \frac{n^2}{2\epsilon}.$$

### 11.3 Stanchion Interpretation

For a single-server stanchion network, `S = {e_i}` (serve exactly one node
per slot).  The MW rule reduces to **longest-queue-first (LQF)**: serve the
node with the highest `w_i * Q_i` score.

The `MaxWeightScheduler` in `src/scheduling/mod.rs` implements this and
supports per-node weights for priority (e.g., VIP lanes).

---

## 12. Complexity Summary

| Algorithm               | Complexity                  | Best used when                          |
|-------------------------|-----------------------------|-----------------------------------------|
| Ford-Fulkerson          | $O(E \cdot F)$, pseudo-poly | Small integer-capacity graphs           |
| Edmonds-Karp            | $O(VE^2)$                   | Polynomial baseline reference           |
| Dinic                   | $O(V^2 E)$                  | General-purpose; unit graphs are faster |
| Push-relabel (FIFO+gap) | $O(V^2 \sqrt{E})$           | Dense graphs                            |
| Capacity scaling        | $O(m^2 \log U)$             | Large-capacity networks (U >> V)        |
| SSP (Dijkstra+potentials)| $O(F(E + V\log V))$        | Low-demand min-cost flow                |
| Network simplex         | $O(nm \log n)$ empirical    | Practical min-cost flow (best in class) |
| Cycle canceling (neg)   | $O(nmn C)$ pseudo-poly      | Alternative MCF; simple to verify       |
| Time-expanded flow      | Depends on static solver    | Routing with travel-time constraints    |
| Jackson steady-state    | $O(M^3)$ (linear solve)     | Stochastic steady-state analysis        |
| Max-weight scheduling   | $O(n)$ per step             | Throughput-optimal queue scheduling     |

---

## 10. References

- L. Ford and D. Fulkerson, *Flows in Networks*, Princeton, 1962.
- E. Dinic, "Algorithm for solution of a problem of maximum flow in a network with power estimation," *Doklady*, 1970.
- A. Goldberg and R. Tarjan, "A new approach to the maximum-flow problem," *JACM* 35(4), 1988.
- J. Orlin, "A faster strongly polynomial minimum cost flow algorithm," *Operations Research*, 1997.
- MIT 6.854 Advanced Algorithms lecture notes (Karger, 2008): https://ocw.mit.edu/courses/6-854j-advanced-algorithms-fall-2008/pages/lecture-notes/
- MIT network algorithms scribe notes (Shah et al.)
