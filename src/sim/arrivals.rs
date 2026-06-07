use rand::{Rng, SeedableRng, rngs::SmallRng};

/// Generates inter-arrival times (seconds between consecutive arrivals).
pub trait ArrivalProcess {
    fn next_interarrival(&mut self) -> f64;
}

/// Poisson arrivals with rate `lambda`.
///
/// Inter-arrival times follow an Exponential(lambda) distribution,
/// sampled as `-ln(U) / lambda` where `U` is uniform on (0, 1).
pub struct PoissonArrival {
    lambda: f64,
    rng:    SmallRng,
}

impl PoissonArrival {
    pub fn new(lambda: f64, seed: u64) -> Self {
        Self { lambda, rng: SmallRng::seed_from_u64(seed) }
    }
}

impl ArrivalProcess for PoissonArrival {
    fn next_interarrival(&mut self) -> f64 {
        let u: f64 = self.rng.random();
        -u.ln() / self.lambda
    }
}

/// Deterministic arrivals: constant inter-arrival interval `1 / rate`.
pub struct DeterministicArrival {
    interval: f64,
}

impl DeterministicArrival {
    pub fn new(rate: f64) -> Self {
        Self { interval: 1.0 / rate }
    }
}

impl ArrivalProcess for DeterministicArrival {
    fn next_interarrival(&mut self) -> f64 {
        self.interval
    }
}
