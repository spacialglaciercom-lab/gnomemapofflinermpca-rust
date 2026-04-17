pub mod abstractions;
pub mod graph;
pub mod hierholzer;
pub mod matching;
pub mod types;

use crate::geo::spatial::coord_distance;
use crate::geo::types::{Coordinate, Way as GeoWay};
use crate::optimizer::abstractions::SpatialProvider;
use crate::optimizer::types::{Node, OptimizationResult, RoutePoint, Way};
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex, EdgeIndex};
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
    augmented_edges: std::collections::HashSet<(NodeIndex, NodeIndex)>,
}

impl RouteOptimizer {
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index: HashMap::new(),
            spatial_registry: HashMap::new(),
            augmented_edges: std::collections::HashSet::new(),
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

                    self.add_edge(prev, idx, distance, false);
                    if !is_oneway {
                        self.add_edge(idx, prev, distance, false);
                    }
                }
                prev_idx = Some(idx);
            }
        }
        Ok(())
    }

    /// Build directed graph from geo::types::Way (from PBF parser).
    pub fn build_graph_from_geo_ways(&mut self, ways: &[GeoWay]) -> Result<()> {
        for way in ways {
            let is_oneway = way.is_oneway();

            let mut prev_idx: Option<NodeIndex> = None;
            for node_id in &way.node_ids {
                let idx = *self.node_index.entry(node_id.clone())
                    .or_insert_with(|| self.graph.add_node(Node::new(node_id.clone())));

                if let Some(prev) = prev_idx {
                    let prev_id = &self.graph[prev].id;
                    let distance = self.haversine_between(prev_id, node_id);

                    self.add_edge(prev, idx, distance, false);
                    if !is_oneway {
                        self.add_edge(idx, prev, distance, false);
                    }
                }
                prev_idx = Some(idx);
            }
        }
        Ok(())
    }

    /// Add an edge to the graph, optionally marking it as augmented (added for balancing).
    fn add_edge(&mut self, from: NodeIndex, to: NodeIndex, weight: f64, is_augmented: bool) -> EdgeIndex {
        let edge = self.graph.add_edge(from, to, weight);
        if is_augmented {
            self.augmented_edges.insert((from, to));
        }
        edge
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
        let mut supply: Vec<(NodeIndex, i64)> = Vec::new();
        let mut demand: Vec<(NodeIndex, i64)> = Vec::new();

        for idx in self.graph.node_indices() {
            let in_deg = self.graph.edges_directed(idx, petgraph::Direction::Incoming).count();
            let out_deg = self.graph.edges_directed(idx, petgraph::Direction::Outgoing).count();
            let diff = out_deg as i64 - in_deg as i64;
            if diff > 0 {
                supply.push((idx, diff));
            } else if diff < 0 {
                demand.push((idx, -diff));
            }
        }

        // Greedy matching: pair supply with demand nodes and add shortest paths
        // TODO: Replace with proper minimum-weight matching algorithm
        while let Some((supply_idx, supply_count)) = supply.first_mut() {
            if *supply_count == 0 {
                supply.remove(0);
                continue;
            }

            if let Some((demand_idx, demand_count)) = demand.first_mut() {
                if *demand_count == 0 {
                    demand.remove(0);
                    continue;
                }

                // Find shortest path and add as augmented edge
                let result = petgraph::algo::dijkstra(
                    &self.graph,
                    *supply_idx,
                    Some(*demand_idx),
                    |e| *e.weight(),
                );

                if let Some(dist) = result.get(demand_idx) {
                    // Add the shortest path as an augmented edge
                    self.add_edge(*supply_idx, *demand_idx, *dist, true);
                }

                *supply_count -= 1;
                *demand_count -= 1;
            } else {
                break;
            }
        }

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

        // 2. Build directed graph
        self.build_graph(&parse_result.ways)?;

        // 3. Make Eulerian
        self.make_eulerian()?;

        // 4. Find circuit (start from first node)
        let start = parse_result.ways.first()
            .and_then(|w| w.nodes.first())
            .ok_or_else(|| anyhow::anyhow!("No nodes in input"))?;
        let circuit = self.find_circuit(start)?;

        // 5. Convert circuit to route points
        let route: Vec<RoutePoint> = circuit.iter()
            .filter_map(|&idx| {
                let node = &self.graph[idx];
                let coord = self.spatial_registry.get(&node.id)?;
                Some(RoutePoint::with_node_id(coord.lat, coord.lon, &node.id))
            })
            .collect();

        // 6. Sum edge weights along circuit
        let total_distance: f64 = circuit.windows(2)
            .filter_map(|w| {
                let from = w[0];
                let to = w[1];
                self.graph.edges_connecting(from, to).next().map(|e| *e.weight())
            })
            .sum();

        Ok(OptimizationResult::new(route, total_distance / 1000.0))
    }

    /// Set the spatial registry (node ID -> coordinate mapping).
    /// This must be called before build_graph if not using optimize() from GeoJSON.
    pub fn set_spatial_registry(&mut self, registry: HashMap<String, Coordinate>) {
        self.spatial_registry = registry;
    }

    /// Populate spatial registry from geo::types::Way (which has geometry).
    pub fn populate_spatial_registry_from_geo_ways(&mut self, ways: &[GeoWay]) {
        for way in ways {
            for (node_id, coord) in way.node_ids.iter().zip(way.geometry.coordinates.iter()) {
                self.spatial_registry.insert(node_id.clone(), *coord);
            }
        }
    }

    /// Full optimization pipeline from pre-parsed ways.
    /// This is the preferred entry point for offline PBF processing.
    pub fn optimize_with_geo_ways(&mut self, ways: &[GeoWay]) -> Result<OptimizationResult> {
        // 1. Populate spatial registry from way geometries
        self.populate_spatial_registry_from_geo_ways(ways);

        // 2. Build directed graph (uses spatial_registry for edge weights)
        self.build_graph_from_geo_ways(ways)?;

        // Track original graph size for stats BEFORE balancing
        let original_edge_count = self.graph.edge_count();
        let original_node_count = self.graph.node_count();

        // 3. Make Eulerian (adds augmented edges)
        self.make_eulerian()?;

        // 4. Find circuit (start from first node)
        let start = ways.first()
            .and_then(|w| w.node_ids.first())
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

        // 6. Calculate total and deadhead distances
        let mut total_distance = 0.0;
        let mut deadhead_distance = 0.0;

        for window in circuit.windows(2) {
            let from = window[0];
            let to = window[1];

            // In an Eulerian circuit, there might be multiple edges between two nodes.
            // We need to check if this specific traversal corresponds to an augmented edge.
            if let Some(edge) = self.graph.edges_connecting(from, to).next() {
                let dist = *edge.weight();
                total_distance += dist;

                if self.augmented_edges.contains(&(from, to)) {
                    deadhead_distance += dist;
                }
            }
        }

        // 7. Build result with stats
        let total_km = total_distance / 1000.0;
        let deadhead_km = deadhead_distance / 1000.0;

        let result = OptimizationResult::new(route, total_km)
            .with_deadhead(deadhead_km)
            .with_graph_size(original_edge_count, original_node_count);

        Ok(result)
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
