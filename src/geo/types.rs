//! Core geographic types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A geographic coordinate (latitude, longitude)
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lon: f64,
}

impl Coordinate {
    pub fn new(lat: f64, lon: f64) -> Self {
        Self { lat, lon }
    }

    /// Format to ~1m precision for node identity
    pub fn to_key(&self) -> String {
        format!("{:.6}_{:.6}", self.lon, self.lat)
    }
}

/// A node in the geographic network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoNode {
    pub id: String,
    pub coord: Coordinate,
    pub z: Option<f64>,
}

impl GeoNode {
    pub fn new(id: String, coord: Coordinate) -> Self {
        Self { id, coord, z: None }
    }

    pub fn with_elevation(mut self, z: f64) -> Self {
        self.z = Some(z);
        self
    }
}

/// Way geometry with coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WayGeometry {
    pub coordinates: Vec<Coordinate>,
}

/// A way (road segment) from OSM/GeoJSON data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Way {
    pub id: String,
    pub geometry: WayGeometry,
    pub node_ids: Vec<String>,
    pub tags: HashMap<String, String>,
}

impl Way {
    pub fn new(id: String, node_ids: Vec<String>, tags: HashMap<String, String>) -> Self {
        Self {
            id,
            geometry: WayGeometry { coordinates: Vec::new() },
            node_ids,
            tags,
        }
    }

    pub fn with_geometry(mut self, coordinates: Vec<Coordinate>) -> Self {
        self.geometry = WayGeometry { coordinates };
        self
    }

    pub fn is_oneway(&self) -> bool {
        self.tags
            .get("oneway")
            .map(|v| v == "yes" || v == "true" || v == "1")
            .unwrap_or(false)
    }
}
