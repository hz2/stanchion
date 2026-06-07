use rand::{Rng, SeedableRng, rngs::SmallRng};

use crate::graph::NodeId;

/// Generates service times for a given node (seconds to serve one person).
pub trait ServiceProcess {
    fn service_time(&mut self, node: NodeId) -> f64;
}

/// Exponential service times at each node with per-node rate `mu[i]`.
///
/// Service at node `i` is sampled as `-ln(U) / mu[i]` where `U` is
/// uniform on (0, 1).
pub struct ExponentialService {
    mu:  Vec<f64>,
    rng: SmallRng,
}

impl ExponentialService {
    pub fn new(mu: Vec<f64>, seed: u64) -> Self {
        Self { mu, rng: SmallRng::seed_from_u64(seed) }
    }

    /// All nodes share the same service rate `mu`.
    pub fn uniform(mu: f64, n_nodes: usize, seed: u64) -> Self {
        Self::new(vec![mu; n_nodes], seed)
    }
}

impl ServiceProcess for ExponentialService {
    fn service_time(&mut self, node: NodeId) -> f64 {
        let mu: f64 = self.mu.get(node.0 as usize).copied().unwrap_or(1.0);
        let u: f64  = self.rng.random();
        -u.ln() / mu
    }
}

/// Fixed (deterministic) service time per node.
pub struct FixedService {
    times: Vec<f64>,
}

impl FixedService {
    pub fn new(times: Vec<f64>) -> Self {
        Self { times }
    }
}

impl ServiceProcess for FixedService {
    fn service_time(&mut self, node: NodeId) -> f64 {
        self.times.get(node.0 as usize).copied().unwrap_or(1.0)
    }
}
