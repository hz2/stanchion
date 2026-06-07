/// Max-weight scheduling for stanchion queueing networks.
///
/// # Background (MIT 6.266 Lecture 4, Shah 2008)
///
/// Given n queues with lengths Q = (Q_1, ..., Q_n) and a set S of feasible
/// service schedules (each sigma in S is a 0/1 vector indicating which queues
/// are served simultaneously), the **max-weight (MW) algorithm** selects:
///
/// ```text
/// sigma*(t) = argmax_{sigma in S} sum_i Q_i(t) * sigma_i
/// ```
///
/// **Theorem (Tassiulas-Ephremides 1992):** MW is throughput-optimal -- if any
/// algorithm can stabilise the network (keep queues bounded), MW can too.
///
/// **Lyapunov bound:** Under MW with arrival rate lambda and epsilon-interior
/// capacity region, the mean total queue length satisfies:
///
/// ```text
/// E[sum_i Q_i^2] <= n^2 / (2 * epsilon)
/// ```
///
/// # Usage in Stanchion
///
/// For a stanchion network modeled as a DiGraph, a "schedule" at time t
/// selects which nodes are actively serving.  The single-server schedule
/// set is S = {e_i} (serve exactly one queue at a time).  MW degenerates
/// to: serve the node with the largest queue.
///
/// The `MaxWeightScheduler` provides `schedule_step()` for discrete-time
/// simulations, and `next_to_serve()` for integration with the event-driven
/// `SimulationEngine`.
use crate::graph::NodeId;

/// A snapshot of queue state for scheduling decisions.
#[derive(Debug, Clone)]
pub struct QueueState {
    /// queue_len[i] = number waiting (not in service) at node i.
    pub queue_len: Vec<usize>,
    /// in_service[i] = true if node i currently has a customer being served.
    pub in_service: Vec<bool>,
}

impl QueueState {
    pub fn new(n: usize) -> Self {
        Self {
            queue_len:  vec![0; n],
            in_service: vec![false; n],
        }
    }

    /// Total customers at node i (waiting + in service).
    #[inline]
    pub fn total_at(&self, i: usize) -> usize {
        self.queue_len[i] + if self.in_service[i] { 1 } else { 0 }
    }
}

/// Schedules service decisions according to the max-weight rule.
///
/// For single-server networks, this reduces to longest-queue-first (LQF):
/// serve the node with the highest `Q_i * weight_i` score.
pub struct MaxWeightScheduler {
    /// Per-node weights (default 1.0 for equal priority).
    pub weights: Vec<f64>,
}

impl MaxWeightScheduler {
    pub fn new(n: usize) -> Self {
        Self { weights: vec![1.0; n] }
    }

    pub fn with_weights(weights: Vec<f64>) -> Self {
        Self { weights }
    }

    /// Return the MW score for node i: Q_i * weight_i (queued customers only).
    #[inline]
    pub fn score(&self, state: &QueueState, i: usize) -> f64 {
        state.queue_len[i] as f64 * self.weights[i]
    }

    /// Select the single best node to serve next (single-server LQF/MW rule).
    /// Returns None if all queues are empty.
    pub fn next_to_serve(&self, state: &QueueState) -> Option<NodeId> {
        state
            .queue_len
            .iter()
            .enumerate()
            .filter(|&(_, &q)| q > 0)
            .max_by(|&(i, _), &(j, _)| {
                self.score(state, i)
                    .partial_cmp(&self.score(state, j))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| NodeId(i as u32))
    }

    /// Multi-server schedule: for each node independently decide whether to serve.
    /// `server_count[i]` = number of parallel servers at node i.
    /// Returns a vector of booleans: serve[i] = true if node i should start service.
    pub fn schedule_step(&self, state: &QueueState, server_count: &[usize]) -> Vec<bool> {
        let n = state.queue_len.len();
        (0..n)
            .map(|i| {
                let servers = *server_count.get(i).unwrap_or(&1);
                state.queue_len[i] > 0
                    && state.in_service.iter().enumerate()
                        .filter(|&(j, &b)| j == i && b)
                        .count()
                        < servers
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn longest_queue_selected() {
        let mut state = QueueState::new(3);
        state.queue_len[0] = 2;
        state.queue_len[1] = 5;
        state.queue_len[2] = 1;
        let sched = MaxWeightScheduler::new(3);
        assert_eq!(sched.next_to_serve(&state), Some(NodeId(1)));
    }

    #[test]
    fn weights_break_ties() {
        let mut state = QueueState::new(2);
        state.queue_len[0] = 4;
        state.queue_len[1] = 4;
        let sched = MaxWeightScheduler::with_weights(vec![1.0, 2.0]);
        // node 1 wins because 4*2.0 > 4*1.0
        assert_eq!(sched.next_to_serve(&state), Some(NodeId(1)));
    }

    #[test]
    fn empty_queues_return_none() {
        let state = QueueState::new(4);
        let sched = MaxWeightScheduler::new(4);
        assert_eq!(sched.next_to_serve(&state), None);
    }
}
