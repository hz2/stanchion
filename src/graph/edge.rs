use super::node::NodeId;

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct EdgeId(pub u32);

/// People per unit time (throughput capacity of a stanchion belt).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Capacity(pub f64);

/// Transit time or discomfort cost for traversing a belt.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Cost(pub f64);

#[derive(Debug, Clone)]
pub struct Edge {
    pub from:     NodeId,
    pub to:       NodeId,
    pub capacity: Capacity,
    pub cost:     Cost,
}
