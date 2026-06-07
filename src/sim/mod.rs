pub mod arrivals;
pub mod event;
pub mod service;

pub use arrivals::{ArrivalProcess, DeterministicArrival, PoissonArrival};
pub use event::{Event, EventQueue, SimTime};
pub use service::{ExponentialService, FixedService, ServiceProcess};

use std::collections::VecDeque;

use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::{
    error::StanchionError,
    graph::{DiGraph, NodeId},
};

/// Parameters controlling a simulation run.
#[derive(Debug, Clone)]
pub struct SimConfig {
    /// Simulated time to run.
    pub max_time:   f64,
    /// Optional hard cap on events processed (safety valve for tests).
    pub max_events: Option<u64>,
    /// RNG seed for routing decisions.
    pub seed:       u64,
}

/// Statistics collected from a completed simulation run.
#[derive(Debug, Clone)]
pub struct SimStats {
    pub total_arrivals:    u64,
    pub total_departures:  u64,
    /// Time-average queue length (queue + in-service) per node.
    pub mean_queue_len:    Vec<f64>,
    /// Departures per unit time.
    pub throughput:        f64,
}

/// Event-driven simulator for a stanchion network.
///
/// `A` drives inter-arrival times; `S` drives per-node service times.
pub struct SimulationEngine<A: ArrivalProcess, S: ServiceProcess> {
    pub arrival: A,
    pub service: S,
}

impl<A: ArrivalProcess, S: ServiceProcess> SimulationEngine<A, S> {
    pub fn new(arrival: A, service: S) -> Self {
        Self { arrival, service }
    }

    pub fn run(
        &mut self,
        graph:  &DiGraph,
        config: &SimConfig,
    ) -> Result<SimStats, StanchionError> {
        let source = graph.source().ok_or(StanchionError::NoSource)?;
        let n      = graph.node_count();

        let mut events      = EventQueue::new();
        let mut queues      = vec![VecDeque::<u64>::new(); n];
        let mut busy        = vec![false; n];
        let mut rng         = SmallRng::seed_from_u64(config.seed);

        let mut next_id          = 0u64;
        let mut total_arrivals   = 0u64;
        let mut total_departures = 0u64;
        let mut area             = vec![0.0f64; n]; // time-integral of queue length
        let mut last_t           = 0.0f64;
        let mut event_count      = 0u64;

        let t0 = SimTime(self.arrival.next_interarrival());
        events.push(t0, Event::Arrival { node: source, person_id: next_id });
        next_id += 1;

        while let Some((now, ev)) = events.pop() {
            if now.0 >= config.max_time {
                break;
            }
            if config.max_events.is_some_and(|m| event_count >= m) {
                break;
            }
            event_count += 1;

            // Accumulate time-weighted queue lengths.
            let dt = now.0 - last_t;
            for v in 0..n {
                let in_sys = queues[v].len() as f64 + if busy[v] { 1.0 } else { 0.0 };
                area[v] += dt * in_sys;
            }
            last_t = now.0;

            match ev {
                Event::Arrival { node, person_id } => {
                    total_arrivals += 1;
                    let vi = node.0 as usize;
                    if !busy[vi] {
                        busy[vi] = true;
                        let svc  = self.service.service_time(node);
                        let dest = next_node(graph, node, &mut rng);
                        events.push(now.advance(svc), Event::Departure { node, person_id, next: dest });
                    } else {
                        queues[vi].push_back(person_id);
                    }
                    // Schedule next arrival at source.
                    let next_t = now.advance(self.arrival.next_interarrival());
                    events.push(next_t, Event::Arrival { node: source, person_id: next_id });
                    next_id += 1;
                }

                Event::Departure { node, person_id: _, next } => {
                    let vi = node.0 as usize;
                    busy[vi] = false;
                    // Start next person from queue, if any.
                    if let Some(pid) = queues[vi].pop_front() {
                        busy[vi] = true;
                        let svc  = self.service.service_time(node);
                        let dest = next_node(graph, node, &mut rng);
                        events.push(now.advance(svc), Event::Departure { node, person_id: pid, next: dest });
                    }
                    // Route departed person to next node.
                    if let Some(dest) = next {
                        let di = dest.0 as usize;
                        if !busy[di] {
                            busy[di] = true;
                            let svc       = self.service.service_time(dest);
                            let next_dest = next_node(graph, dest, &mut rng);
                            events.push(now.advance(svc), Event::Departure { node: dest, person_id: 0, next: next_dest });
                        } else {
                            queues[di].push_back(0);
                        }
                    } else {
                        total_departures += 1;
                    }
                }
            }
        }

        let t = config.max_time;
        let mean_queue_len = area.iter().map(|&a| a / t).collect();

        Ok(SimStats {
            total_arrivals,
            total_departures,
            mean_queue_len,
            throughput: total_departures as f64 / t,
        })
    }
}

/// Pick a random forward neighbour of `node`; returns `None` if `node` is a dead-end.
fn next_node(graph: &DiGraph, node: NodeId, rng: &mut SmallRng) -> Option<NodeId> {
    let fwd: Vec<NodeId> = graph
        .neighbors(node)
        .filter(|(_, idx)| idx % 2 == 0)
        .map(|(v, _)| v)
        .collect();

    match fwd.len() {
        0 => None,
        1 => Some(fwd[0]),
        n => Some(fwd[rng.random_range(0..n)]),
    }
}
