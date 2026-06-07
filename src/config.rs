//! JSON configuration for stanchion networks and simulations.
//!
//! Load a network and its simulation parameters from a `.json` file:
//! ```no_run
//! use stanchion::config::NetworkConfig;
//! let cfg = NetworkConfig::from_file("layout.json").unwrap();
//! let net = cfg.build().unwrap();
//! ```
//!
//! Example JSON:
//! ```json
//! {
//!   "nodes": [
//!     { "id": 0, "label": "entry",    "service_rate": 10.0 },
//!     { "id": 1, "label": "lane_a",   "service_rate": 3.0  },
//!     { "id": 2, "label": "security", "service_rate": 5.0  }
//!   ],
//!   "edges": [
//!     { "from": 0, "to": 1, "capacity": 3.0, "cost": 1.0 },
//!     { "from": 1, "to": 2, "capacity": 3.0, "cost": 1.0 }
//!   ],
//!   "source": 0,
//!   "sink": 2,
//!   "simulation": {
//!     "max_time": 1000.0,
//!     "arrival_rate": 2.0,
//!     "seed": 42,
//!     "max_events": null
//!   },
//!   "routing": [[0.0, 1.0, 0.0], [0.0, 0.0, 1.0], [0.0, 0.0, 0.0]]
//! }
//! ```

use std::{fs, path::Path};

use serde::{Deserialize, Serialize};

use crate::{
    error::StanchionError,
    graph::{Capacity, Cost, DiGraph, GraphBuilder, NodeId},
    sim::SimConfig,
};

/// Per-node configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id:           u32,
    pub label:        String,
    /// M/M/1 service rate for Jackson analysis and simulation.
    #[serde(default)]
    pub service_rate: f64,
}

/// Per-edge configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeConfig {
    pub from:     u32,
    pub to:       u32,
    pub capacity: f64,
    #[serde(default = "default_cost")]
    pub cost:     f64,
}

fn default_cost() -> f64 {
    1.0
}

/// Simulation parameters stored in the config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub max_time:     f64,
    pub arrival_rate: f64,
    #[serde(default = "default_seed")]
    pub seed:         u64,
    #[serde(default)]
    pub max_events:   Option<u64>,
}

fn default_seed() -> u64 {
    42
}

impl From<SimulationConfig> for SimConfig {
    fn from(c: SimulationConfig) -> Self {
        SimConfig {
            max_time:   c.max_time,
            max_events: c.max_events,
            seed:       c.seed,
        }
    }
}

/// Top-level network + simulation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub nodes:      Vec<NodeConfig>,
    pub edges:      Vec<EdgeConfig>,
    pub source:     u32,
    pub sink:       u32,
    #[serde(default)]
    pub simulation: Option<SimulationConfig>,
    /// Routing matrix R[i][j]: probability of routing from node i to node j.
    #[serde(default)]
    pub routing:    Option<Vec<Vec<f64>>>,
}

/// Output of `NetworkConfig::build`: the graph plus optional sim config.
pub struct BuiltNetwork {
    pub graph:      DiGraph,
    pub sim_config: Option<SimConfig>,
    /// Service rates extracted from node configs (for Jackson analysis).
    pub service_rates:     Vec<f64>,
    /// External arrival rates: non-zero only at source by default.
    pub external_arrivals: Vec<f64>,
    /// Routing matrix (identity-less; zeroed if not specified).
    pub routing:    Vec<Vec<f64>>,
    pub arrival_rate: f64,
}

impl NetworkConfig {
    /// Load from a JSON file.
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self, StanchionError> {
        let s = fs::read_to_string(path).map_err(|e| StanchionError::ConfigIo(e.to_string()))?;
        Self::from_json_str(&s)
    }

    /// Parse from a JSON string.
    pub fn from_json_str(s: &str) -> Result<Self, StanchionError> {
        serde_json::from_str(s).map_err(|e| StanchionError::ConfigParse(e.to_string()))
    }

    /// Build the `DiGraph` and supporting structures.
    pub fn build(self) -> Result<BuiltNetwork, StanchionError> {
        let n = self.nodes.len();
        let mut b = GraphBuilder::new();
        // nodes must be added in id order
        let mut sorted = self.nodes.clone();
        sorted.sort_by_key(|n| n.id);
        for nc in &sorted {
            let (nb, _) = b.add_stanchion(nc.label.as_str());
            b = nb;
        }
        b = b
            .source(NodeId(self.source))
            .sink(NodeId(self.sink));
        for ec in &self.edges {
            b = b.connect(
                NodeId(ec.from),
                NodeId(ec.to),
                Capacity(ec.capacity),
                Cost(ec.cost),
            );
        }
        let graph = b.build()?;

        let service_rates: Vec<f64> = sorted.iter().map(|nc| nc.service_rate).collect();

        let arrival_rate = self
            .simulation
            .as_ref()
            .map_or(1.0, |s| s.arrival_rate);

        let mut external_arrivals = vec![0.0; n];
        if (self.source as usize) < n {
            external_arrivals[self.source as usize] = arrival_rate;
        }

        let routing = self.routing.unwrap_or_else(|| vec![vec![0.0; n]; n]);
        let sim_config = self.simulation.map(Into::into);

        Ok(BuiltNetwork {
            graph,
            sim_config,
            service_rates,
            external_arrivals,
            routing,
            arrival_rate,
        })
    }

    /// Write config to a JSON file.
    pub fn to_file(&self, path: impl AsRef<Path>) -> Result<(), StanchionError> {
        let s = serde_json::to_string_pretty(self)
            .map_err(|e| StanchionError::ConfigParse(e.to_string()))?;
        fs::write(path, s).map_err(|e| StanchionError::ConfigIo(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: &str = r#"{
        "nodes": [
            { "id": 0, "label": "entry",    "service_rate": 10.0 },
            { "id": 1, "label": "lane_a",   "service_rate": 3.0  },
            { "id": 2, "label": "security", "service_rate": 5.0  }
        ],
        "edges": [
            { "from": 0, "to": 1, "capacity": 3.0, "cost": 1.0 },
            { "from": 1, "to": 2, "capacity": 3.0, "cost": 1.0 }
        ],
        "source": 0,
        "sink": 2,
        "simulation": {
            "max_time": 500.0,
            "arrival_rate": 2.0,
            "seed": 7
        }
    }"#;

    #[test]
    fn parse_and_build() {
        let cfg = NetworkConfig::from_json_str(EXAMPLE).unwrap();
        let net = cfg.build().unwrap();
        assert_eq!(net.graph.node_count(), 3);
        assert_eq!(net.graph.edge_count(), 2);
        assert_eq!(net.service_rates, vec![10.0, 3.0, 5.0]);
        assert_eq!(net.arrival_rate, 2.0);
    }

    #[test]
    fn default_edge_cost() {
        let json = r#"{
            "nodes": [{"id":0,"label":"s"},{"id":1,"label":"t"}],
            "edges": [{"from":0,"to":1,"capacity":1.0}],
            "source": 0,
            "sink": 1
        }"#;
        let cfg = NetworkConfig::from_json_str(json).unwrap();
        assert_eq!(cfg.edges[0].cost, 1.0);
    }
}
