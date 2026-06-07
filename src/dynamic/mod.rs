pub mod time_expanded;

pub use time_expanded::{TimeHorizon, build_time_expanded};

use crate::{
    error::StanchionError,
    graph::{DiGraph, EdgeId},
};

use crate::flow::{FlowResult, MaxFlowSolver};

/// Solve dynamic max-flow by time-expanding the graph and delegating to a static solver.
pub struct DynamicFlowSolver<S: MaxFlowSolver> {
    solver: S,
}

impl<S: MaxFlowSolver> DynamicFlowSolver<S> {
    pub fn new(solver: S) -> Self {
        Self { solver }
    }

    /// `transit_fn` maps each forward `EdgeId` to its integer transit time (in steps).
    pub fn solve(
        &self,
        base:       &DiGraph,
        horizon:    TimeHorizon,
        transit_fn: impl Fn(EdgeId) -> u32,
    ) -> Result<FlowResult, StanchionError> {
        let (expanded, exp_src, exp_snk) = build_time_expanded(base, horizon, transit_fn)?;
        self.solver.max_flow(&expanded, exp_src, exp_snk)
    }
}
