pub mod abstractions;
pub mod graph;
pub mod hierholzer;
pub mod matching;
pub mod types;

use crate::geo::spatial::coord_distance;
use crate::geo::types::Coordinate;
use crate::optimizer::abstractions::SpatialProvider;
use crate::optimizer::types::{Node, OptimizationResult, RoutePoint, Way};
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use std::collections::HashMap;

/// Route optimizer using directed Chinese Postman approach.
///
/// Algorithm:
/// 1. Parse GeoJSON → directed graph (handles one-way streets)
/// 2. Balance in/out degrees to make graph Eulerian
/// 3. Find Eulerian circuit via iterative Hierholzer's
/// 4. Post-process to eliminate unnecessary U-turns
pub struct RouteOptimizer {
    graph: DiGraph<Node, f64>,
    node_index: HashMap<String, NodeIndex>,
    spatial_registry: HashMap<String, Coordinate>,
}

impl RouteOptimizer {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            spatial_registry: HashMap::new(),
        }
    }

    /// Build directed graph from parsed ways and spatial registry.
    ///
    /// Edge weights are Haversine distances derived from the spatial registry.
    pub fn build_graph(&mut self, ways: &[Way]) -> Result<()> {
        for way in ways {
            let is_oneway = way.tags.get("oneway")
                .map_or(false, |v| v == "yes" || v == "1" || v == "true");

            let mut prev_idx: Option<NodeIndex> = None;
            for node_id in &way.nodes {
                let idx = *self.node_index.entry(node_id.clone())
                    .or_insert_with(|| self.graph.add_node(Node::new(node_id.clone())));

                if let Some(prev) = prev_idx {
                    let prev_id = &self.graph[prev].id;
                    let distance = self.haversine_between(prev_id, node_id);

                    self.graph.add_edge(prev, idx, distance);
                    if !is_oneway {
                        self.graph.add_edge(idx, prev, distance);
                    }
                }
                prev_idx = Some(idx);
            }
        }
        Ok(())
    }

    /// Calculate Haversine distance between two node IDs using the spatial registry.
    fn haversine_between(&self, id_a: &str, id_b: &str) -> f64 {
        match (self.spatial_registry.get(id_a), self.spatial_registry.get(id_b)) {
            (Some(c1), Some(c2)) => coord_distance(c1, c2),
            _ => 0.0,
        }
    }

    /// Balance vertex degrees to make graph Eulerian.
    ///
    /// For directed graphs: in-degree must equal out-degree at every vertex.
    /// Find imbalanced vertices, compute shortest paths between them,
    /// then add minimum-cost augmenting edges.
    pub fn make_eulerian(&mut self) -> Result<()> {
        let mut supply: Vec<NodeIndex> = Vec::new();
        let mut demand: Vec<NodeIndex> = Vec::new();

        for idx in self.graph.node_indices() {
            let in_deg = self.graph.edges_directed(idx, petgraph::Direction::Incoming).count();
            let out_deg = self.graph.edges_directed(idx, petgraph::Direction::Outgoing).count();
            match in_deg.cmp(&out_deg) {
                std::cmp::Ordering::Less => demand.push(idx),
                std::cmp::Ordering::Greater => supply.push(idx),
                std::cmp::Ordering::Equal => {}
            }
        }

        // TODO: Build cost matrix using SpatialProvider, call matching::min_weight_matching(),
        // and augment graph with matched edges.
        Ok(())
    }

    /// Find Eulerian circuit using iterative Hierholzer's (directed version).
    pub fn find_circuit(&self, start: &str) -> Result<Vec<NodeIndex>> {
        let start_idx = self.node_index.get(start)
            .ok_or_else(|| anyhow::anyhow!("Start node '{}' not in graph", start))?;
        hierholzer::directed_eulerian_circuit(&self.graph, *start_idx)
    }

    /// Full optimization pipeline from GeoJSON input.
    pub fn optimize(&mut self, geojson: &serde_json::Value) -> Result<OptimizationResult> {
        // 1. Parse GeoJSON into ways + spatial registry
        let parse_result = graph::parse_ways_from_geojson(geojson)?;
        self.spatial_registry = parse_result.spatial_registry;

        // 2. Build directed graph (uses spatial_registry for edge weights)
        self.build_graph(&parse_result.ways)?;

        // 3. Make Eulerian
        self.make_eulerian()?;

        // 4. Find circuit (start from first node)
        let start = parse_result.ways.first()
            .and_then(|w| w.nodes.first())
            .ok_or_else(|| anyhow::anyhow!("No nodes in input"))?;
        let circuit = self.find_circuit(start)?;

        // 5. Convert circuit to route points using spatial registry
        let route: Vec<RoutePoint> = circuit.iter()
            .filter_map(|&idx| {
                let node = &self.graph[idx];
                let coord = self.spatial_registry.get(&node.id)?;
                Some(RoutePoint::with_node_id(coord.lat, coord.lon, &node.id))
            })
            .collect();

        // Sum edge weights along circuit
        let total_distance: f64 = circuit.windows(2)
            .filter_map(|w| {
                let from = w[0];
                let to = w[1];
                self.graph.edges_connecting(from, to).next().map(|e| *e.weight())
            })
            .sum();

        Ok(OptimizationResult::new(route, total_distance / 1000.0))
    }
}

impl SpatialProvider for RouteOptimizer {
    type Node = Node;

    fn get_coordinate(&self, node: &Self::Node) -> Option<Coordinate> {
        self.spatial_registry.get(&node.id).cloned()
    }

    fn distance(&self, from: &Self::Node, to: &Self::Node) -> Option<f64> {
        let c1 = self.get_coordinate(from)?;
        let c2 = self.get_coordinate(to)?;
        Some(coord_distance(&c1, &c2))
    }
}

impl Default for RouteOptimizer {
    fn default() -> Self {
        Self::new()
    }
}
