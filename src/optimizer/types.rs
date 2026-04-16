//! Core types for the route optimization engine
//!
//! This module defines the fundamental data structures used throughout
//! the optimization system, representing nodes, ways (road segments),
//! and optimization results.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use crate::geo::types::Coordinate;
use crate::geo::spatial::coord_distance;

/// Represents a node in the road network
///
/// Nodes are typically intersections or endpoints in the road network.
/// They are identified by unique IDs (often OSM node IDs).
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Hash)]
pub struct Node {
    /// Unique identifier for this node
    pub id: String,
}

impl Node {
    /// Create a new node with the given ID
    pub fn new(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
        }
    }
}

/// Represents a way (road segment) in OSM data
///
/// Ways are collections of ordered nodes that form road segments.
/// They can be one-way or two-way streets.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Way {
    /// Unique identifier for this way
    pub id: String,

    /// Ordered list of node IDs that form this way
    pub nodes: Vec<String>,

    /// OSM tags describing this road segment
    pub tags: HashMap<String, String>,
}

impl Way {
    /// Create a new way with the given ID and nodes
    pub fn new(id: impl Into<String>, nodes: Vec<String>) -> Self {
        Self {
            id: id.into(),
            nodes,
            tags: HashMap::new(),
        }
    }

    /// Add a tag to this way
    pub fn with_tag(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.tags.insert(key.into(), value.into());
        self
    }

    /// Check if this way is one-way based on OSM tags
    pub fn is_oneway(&self) -> bool {
        self.tags
            .get("oneway")
            .map(|v| v == "yes" || v == "true" || v == "1")
            .unwrap_or(false)
    }

    /// Get the road type (highway tag)
    pub fn highway_type(&self) -> Option<&String> {
        self.tags.get("highway")
    }

    /// Get the maximum speed limit (maxspeed tag)
    pub fn max_speed(&self) -> Option<f64> {
        self.tags
            .get("maxspeed")
            .and_then(|v| v.parse::<f64>().ok())
    }
}

/// Result of route optimization
///
/// Contains the optimized route as a sequence of points,
/// along with metadata about the route.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    /// Ordered sequence of points forming the optimized route
    pub route: Vec<RoutePoint>,

    /// Total distance of the optimized route in kilometers
    pub total_distance: f64,

    /// Optional message or status information
    pub message: String,

    /// Optional route statistics
    pub stats: Option<RouteStats>,
}

impl OptimizationResult {
    /// Create a new optimization result
    pub fn new(route: Vec<RoutePoint>, total_distance: f64) -> Self {
        Self {
            route,
            total_distance,
            message: "Optimization complete".to_string(),
            stats: None,
        }
    }

    /// Calculate statistics for this result
    pub fn calculate_stats(&mut self) {
        let stats = RouteStats {
            total_points: self.route.len(),
            total_distance_km: self.total_distance,
            average_segment_length: if self.route.len() > 1 {
                let total = self.route.windows(2)
                    .map(|w| w[0].distance_to(&w[1]))
                    .sum::<f64>();
                total / (self.route.len() - 1) as f64
            } else {
                0.0
            },
        };
        self.stats = Some(stats);
    }
}

/// A single point in an optimized route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutePoint {
    /// Latitude coordinate in decimal degrees
    pub latitude: f64,

    /// Longitude coordinate in decimal degrees
    pub longitude: f64,

    /// Optional reference to the original node ID
    pub node_id: Option<String>,
}

impl RoutePoint {
    /// Create a new route point
    pub fn new(lat: f64, lon: f64) -> Self {
        Self {
            latitude: lat,
            longitude: lon,
            node_id: None,
        }
    }

    /// Create a route point with a node ID reference
    pub fn with_node_id(lat: f64, lon: f64, node_id: impl Into<String>) -> Self {
        Self {
            latitude: lat,
            longitude: lon,
            node_id: Some(node_id.into()),
        }
    }

    /// Calculate the Haversine distance to another route point in meters
    pub fn distance_to(&self, other: &RoutePoint) -> f64 {
        let c1 = Coordinate::new(self.latitude, self.longitude);
        let c2 = Coordinate::new(other.latitude, other.longitude);
        coord_distance(&c1, &c2)
    }
}

/// Statistics about an optimized route
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteStats {
    /// Total number of points in the route
    pub total_points: usize,

    /// Total distance in kilometers
    pub total_distance_km: f64,

    /// Average segment length in meters
    pub average_segment_length: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_creation() {
        let node = Node::new("node1");
        assert_eq!(node.id, "node1");
    }

    #[test]
    fn test_way_creation() {
        let way = Way::new("way1", vec!["node1".to_string(), "node2".to_string()]);
        assert_eq!(way.id, "way1");
        assert_eq!(way.nodes.len(), 2);
        assert!(way.tags.is_empty());
    }

    #[test]
    fn test_way_with_tags() {
        let way = Way::new("way1", vec!["node1".to_string()])
            .with_tag("highway", "primary")
            .with_tag("maxspeed", "60");

        assert_eq!(way.highway_type(), Some(&"primary".to_string()));
        assert_eq!(way.max_speed(), Some(60.0));
        assert!(!way.is_oneway());

        let oneway_way = way.with_tag("oneway", "yes");
        assert!(oneway_way.is_oneway());
    }

    #[test]
    fn test_route_point_creation() {
        let point = RoutePoint::new(45.5, -73.6);
        assert_eq!(point.latitude, 45.5);
        assert_eq!(point.longitude, -73.6);
        assert!(point.node_id.is_none());

        let with_id = RoutePoint::with_node_id(45.5, -73.6, "node1");
        assert_eq!(with_id.node_id, Some("node1".to_string()));
    }

    #[test]
    fn test_optimization_result() {
        let route = vec![
            RoutePoint::new(45.5, -73.6),
            RoutePoint::new(45.51, -73.61),
        ];
        let mut result = OptimizationResult::new(route, 12.5);

        assert_eq!(result.total_distance, 12.5);
        assert_eq!(result.route.len(), 2);
        assert!(result.stats.is_none());

        result.calculate_stats();
        assert!(result.stats.is_some());
        let stats = result.stats.as_ref().unwrap();
        assert_eq!(stats.total_points, 2);
        assert_eq!(stats.total_distance_km, 12.5);
    }
}
