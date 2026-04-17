//! OSM PBF parsing for offline route optimization
//!
//! This module provides functionality to parse .osm.pbf files and extract
//! road networks within a bounding box for the CPP optimizer.

use crate::geo::{Coordinate, Way};
use anyhow::{Context, Result};
use osmpbfreader::{OsmPbfReader, OsmObj, NodeId};
use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

/// Road types we consider for optimization (highway tags)
const HIGHWAY_TAGS: &[&str] = &[
    "motorway", "trunk", "primary", "secondary", "tertiary",
    "unclassified", "residential", "service", "motorway_link",
    "trunk_link", "primary_link", "secondary_link", "tertiary_link",
    "living_street", "pedestrian", "track", "road",
];

/// Parse a .osm.pbf file and extract ways within a bounding box
///
/// # Arguments
/// * `file_path` - Path to the .osm.pbf file
/// * `bbox` - Bounding box as (west, south, east, north)
///
/// # Returns
/// Vector of Way objects representing road segments within the bbox
pub fn parse_pbf_in_bbox<P: AsRef<Path>>(file_path: P, bbox: (f64, f64, f64, f64)) -> Result<Vec<Way>> {
    let (west, south, east, north) = bbox;

    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open OSM PBF file: {:?}", file_path.as_ref()))?;

    let mut reader = OsmPbfReader::new(file);

    // First pass: collect all nodes within the bounding box
    let mut nodes: HashMap<NodeId, Coordinate> = HashMap::new();

    for obj in reader.iter() {
        match obj? {
            OsmObj::Node(node) => {
                let lat = node.lat();
                let lon = node.lon();

                if lat >= south && lat <= north && lon >= west && lon <= east {
                    nodes.insert(node.id, Coordinate::new(lat, lon));
                }
            }
            _ => {}
        }
    }

    // Second pass: collect ways that have nodes within the bbox
    // Re-open the file since iterator consumes it
    let file = File::open(&file_path)?;
    let mut reader = OsmPbfReader::new(file);

    let mut ways = Vec::new();

    for obj in reader.iter() {
        if let OsmObj::Way(osm_way) = obj? {
            // Filter by highway tag
            if !is_highway(&osm_way.tags) {
                continue;
            }

            // Check if way has nodes within our bbox
            let mut has_bbox_nodes = false;
            let mut way_coords = Vec::new();
            let mut node_ids = Vec::new();

            for node_id in &osm_way.nodes {
                if let Some(coord) = nodes.get(node_id) {
                    has_bbox_nodes = true;
                    way_coords.push(*coord);
                    node_ids.push(node_id.0.to_string());
                }
            }

            if has_bbox_nodes && way_coords.len() >= 2 {
                let tags: HashMap<String, String> = osm_way.tags
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();

                let way = Way::new(osm_way.id.0.to_string(), node_ids, tags)
                    .with_geometry(way_coords);
                ways.push(way);
            }
        }
    }

    Ok(ways)
}

/// Parse a .osm.pbf file and extract all ways (without bbox filtering)
///
/// This is useful when the bbox filtering should be done at a different level
/// or when processing entire smaller files.
pub fn parse_pbf_ways<P: AsRef<Path>>(file_path: P) -> Result<(Vec<Way>, HashMap<String, Coordinate>)> {
    let file = File::open(&file_path)
        .with_context(|| format!("Failed to open OSM PBF file: {:?}", file_path.as_ref()))?;

    let mut reader = OsmPbfReader::new(file);

    // First pass: collect all nodes
    let mut nodes: HashMap<String, Coordinate> = HashMap::new();

    for obj in reader.iter() {
        if let OsmObj::Node(node) = obj? {
            let lat = node.lat();
            let lon = node.lon();
            nodes.insert(node.id.0.to_string(), Coordinate::new(lat, lon));
        }
    }

    // Second pass: collect ways
    let file = File::open(&file_path)?;
    let mut reader = OsmPbfReader::new(file);

    let mut ways = Vec::new();

    for obj in reader.iter() {
        if let OsmObj::Way(osm_way) = obj? {
            // Filter by highway tag
            if !is_highway(&osm_way.tags) {
                continue;
            }

            let mut way_coords = Vec::new();
            let mut node_ids = Vec::new();

            for node_id in &osm_way.nodes {
                if let Some(coord) = nodes.get(&node_id.0.to_string()) {
                    way_coords.push(*coord);
                    node_ids.push(node_id.0.to_string());
                }
            }

            if way_coords.len() >= 2 {
                let tags: HashMap<String, String> = osm_way.tags
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();

                let way = Way::new(osm_way.id.0.to_string(), node_ids, tags)
                    .with_geometry(way_coords);
                ways.push(way);
            }
        }
    }

    Ok((ways, nodes))
}

/// Check if the tags indicate this is a highway (road)
fn is_highway(tags: &osmpbfreader::Tags) -> bool {
    tags.get("highway")
        .map(|value| {
            let value = value.to_lowercase();
            HIGHWAY_TAGS.contains(&value.as_str()) && value != "construction"
        })
        .unwrap_or(false)
}
