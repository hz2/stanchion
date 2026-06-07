use std::{cmp::Ordering, collections::BinaryHeap};

use crate::graph::NodeId;

/// Simulation time (non-negative real).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimTime(pub f64);

impl SimTime {
    pub const ZERO: SimTime = SimTime(0.0);

    pub fn advance(self, dt: f64) -> SimTime {
        SimTime(self.0 + dt)
    }
}

// Comparison via total order (NaN excluded by construction).
impl Eq for SimTime {}
impl PartialOrd for SimTime {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for SimTime {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.partial_cmp(&other.0).unwrap_or(Ordering::Equal)
    }
}

/// Events that drive the simulation forward.
#[derive(Debug, Clone)]
pub enum Event {
    /// A person arrives at the given stanchion node.
    Arrival { node: NodeId, person_id: u64 },
    /// A person finishes service and moves to the next node (or exits).
    Departure { node: NodeId, person_id: u64, next: Option<NodeId> },
}

// Wrap events with time for the priority queue.
struct TimedEvent {
    time:  SimTime,
    event: Event,
}

// BinaryHeap is a max-heap; we want min-time first, so reverse the comparison.
impl Ord for TimedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        other.time.cmp(&self.time)
    }
}
impl PartialOrd for TimedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl PartialEq for TimedEvent {
    fn eq(&self, other: &Self) -> bool {
        self.time == other.time
    }
}
impl Eq for TimedEvent {}

/// Min-heap of timed events.
#[derive(Default)]
pub struct EventQueue(BinaryHeap<TimedEvent>);

impl EventQueue {
    pub fn new() -> Self {
        Self(BinaryHeap::new())
    }

    pub fn push(&mut self, time: SimTime, event: Event) {
        self.0.push(TimedEvent { time, event });
    }

    pub fn pop(&mut self) -> Option<(SimTime, Event)> {
        self.0.pop().map(|te| (te.time, te.event))
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
