//! Geographic primitives and spatial operations
//!
//! This module contains pure geographic logic without graph theory dependencies.
//! It handles coordinate systems, distances, and spatial calculations.

pub mod parsing;
pub mod spatial;
pub mod types;

pub use parsing::{parse_geojson, NodeDeduplicator};
pub use spatial::{haversine, bearing, calculate_bearing};
pub use types::{Coordinate, GeoNode, Way, WayGeometry};
