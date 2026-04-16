//! GeoJSON parsing utilities

use crate::geo::types::{Coordinate, GeoNode};
use anyhow::Result;
use std::collections::HashMap;

/// Deduplicates nodes by coordinate identity.
pub struct NodeDeduplicator {
    seen: HashMap<String, GeoNode>,
}

impl NodeDeduplicator {
    pub fn new() -> Self {
        Self {
            seen: HashMap::new(),
        }
    }

    pub fn insert(&mut self, coord: Coordinate) -> String {
        let key = coord.to_key();
        self.seen.entry(key.clone())
            .or_insert_with(|| GeoNode::new(key.clone(), coord));
        key
    }
}

impl Default for NodeDeduplicator {
    fn default() -> Self {
        Self::new()
    }
}

/// Parse a GeoJSON value into a list of GeoNodes (placeholder).
pub fn parse_geojson(_geojson: &serde_json::Value) -> Result<Vec<GeoNode>> {
    Ok(vec![])
}
