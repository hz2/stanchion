use crate::error::StanchionError;

/// External arrival rates γ_i (people arriving from outside per unit time).
#[derive(Debug, Clone)]
pub struct ExternalArrivals(pub Vec<f64>);

/// Service rates μ_i (people served per unit time at node i).
#[derive(Debug, Clone)]
pub struct ServiceRates(pub Vec<f64>);

/// Routing matrix R where R[i][j] = probability that a person leaving node i
/// goes next to node j.  Each row must sum to ≤ 1 (remainder exits the network).
#[derive(Debug, Clone)]
pub struct RoutingMatrix(pub Vec<Vec<f64>>);

/// A Jackson open queueing network of M/M/1 queues.
///
/// Traffic equations (solve for `lambda`):
/// ```text
/// lambda = gamma + lambda * R
/// (I - R^T) * lambda = gamma
/// ```
///
/// Jackson's theorem: in steady state the joint distribution of queue lengths
/// factors as a product of independent M/M/1 distributions.
/// Node `i` has utilisation `rho_i = lambda_i / mu_i` and mean queue length
/// `L_i = rho_i / (1 - rho_i)` when `rho_i < 1`.
pub struct JacksonNetwork {
    pub arrivals: ExternalArrivals,
    pub service:  ServiceRates,
    pub routing:  RoutingMatrix,
}

/// Steady-state metrics for each node.
#[derive(Debug, Clone)]
pub struct SteadyStateResult {
    /// Total arrival rate λᵢ (external + internal).
    pub throughput:     Vec<f64>,
    /// Utilisation ρᵢ = λᵢ / μᵢ.
    pub utilization:    Vec<f64>,
    /// Mean queue length Lᵢ = ρᵢ / (1 − ρᵢ).
    pub mean_queue_len: Vec<f64>,
    /// Mean sojourn time Wᵢ = Lᵢ / λᵢ  (Little's Law).
    pub mean_sojourn:   Vec<f64>,
    /// True if all ρᵢ < 1.
    pub stable:         bool,
}

impl JacksonNetwork {
    /// Solve the traffic equations (I − Rᵀ) λ = γ via Gauss–Jordan elimination.
    pub fn steady_state(&self) -> Result<SteadyStateResult, StanchionError> {
        let n   = self.arrivals.0.len();
        let mu  = &self.service.0;
        let r   = &self.routing.0;
        let gam = &self.arrivals.0;

        // Validate routing matrix.
        for row in r {
            let s: f64 = row.iter().sum();
            if !(0.0..=(1.0 + 1e-9)).contains(&s) {
                return Err(StanchionError::InvalidRoutingMatrix);
            }
        }

        // Build (I − Rᵀ): coefficient matrix for the system (I − Rᵀ)λ = γ.
        let mut a = vec![vec![0.0f64; n]; n];
        for i in 0..n {
            for j in 0..n {
                a[i][j] = if i == j { 1.0 } else { 0.0 } - r[j][i]; // (I − Rᵀ)[i,j]
            }
        }

        let lambda = gauss_jordan(a, gam.clone())
            .ok_or(StanchionError::InvalidRoutingMatrix)?;

        let mut utilization    = vec![0.0; n];
        let mut mean_queue_len = vec![0.0; n];
        let mut mean_sojourn   = vec![0.0; n];
        let mut stable         = true;

        for i in 0..n {
            let rho = lambda[i] / mu[i];
            utilization[i] = rho;

            if rho >= 1.0 {
                stable = false;
                mean_queue_len[i] = f64::INFINITY;
                mean_sojourn[i]   = f64::INFINITY;
            } else {
                mean_queue_len[i] = rho / (1.0 - rho);
                mean_sojourn[i]   = if lambda[i] > 0.0 {
                    mean_queue_len[i] / lambda[i]
                } else {
                    0.0
                };
            }
        }

        Ok(SteadyStateResult {
            throughput: lambda,
            utilization,
            mean_queue_len,
            mean_sojourn,
            stable,
        })
    }
}

/// Gauss–Jordan elimination with partial pivoting.  Solves Ax = b in-place.
fn gauss_jordan(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();

    for col in 0..n {
        // Partial pivot.
        let pivot_row = (col..n)
            .max_by(|&i, &j| a[i][col].abs().partial_cmp(&a[j][col].abs()).unwrap())?;
        a.swap(col, pivot_row);
        b.swap(col, pivot_row);

        let piv = a[col][col];
        if piv.abs() < 1e-12 {
            return None;
        }

        // Eliminate column in every other row.
        // Clone pivot row to allow independent mutation of other rows.
        let pivot: Vec<f64> = a[col].clone();
        for row in 0..n {
            if row == col {
                continue;
            }
            let factor = a[row][col] / piv;
            for k in col..n {
                a[row][k] -= factor * pivot[k];
            }
            b[row] -= factor * b[col];
        }
    }

    Some((0..n).map(|i| b[i] / a[i][i]).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_mm1_queue() {
        // One-node network: λ = 3, μ = 4, no routing, so ρ = 0.75.
        let net = JacksonNetwork {
            arrivals: ExternalArrivals(vec![3.0]),
            service:  ServiceRates(vec![4.0]),
            routing:  RoutingMatrix(vec![vec![0.0]]),
        };
        let r = net.steady_state().unwrap();
        assert!((r.utilization[0] - 0.75).abs() < 1e-9);
        // L = ρ/(1-ρ) = 3.0
        assert!((r.mean_queue_len[0] - 3.0).abs() < 1e-9);
    }

    #[test]
    fn two_stage_tandem_queue() {
        // Node 0: external arrival 2, routes 100% to node 1.
        // Node 1: external arrival 0, exits.
        // λ₀ = 2, λ₁ = 2.  μ₀ = μ₁ = 4.  ρ₀ = ρ₁ = 0.5.
        let net = JacksonNetwork {
            arrivals: ExternalArrivals(vec![2.0, 0.0]),
            service:  ServiceRates(vec![4.0, 4.0]),
            routing:  RoutingMatrix(vec![vec![0.0, 1.0], vec![0.0, 0.0]]),
        };
        let r = net.steady_state().unwrap();
        assert!((r.throughput[0] - 2.0).abs() < 1e-9);
        assert!((r.throughput[1] - 2.0).abs() < 1e-9);
        assert!((r.utilization[0] - 0.5).abs() < 1e-9);
        assert!(r.stable);
    }

    #[test]
    fn unstable_when_rho_ge_1() {
        let net = JacksonNetwork {
            arrivals: ExternalArrivals(vec![5.0]),
            service:  ServiceRates(vec![3.0]),
            routing:  RoutingMatrix(vec![vec![0.0]]),
        };
        let r = net.steady_state().unwrap();
        assert!(!r.stable);
    }
}
