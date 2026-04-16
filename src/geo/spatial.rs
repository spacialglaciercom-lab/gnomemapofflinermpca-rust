//! Spatial calculations for geographic data

use crate::geo::types::Coordinate;
use std::f64::consts::PI;

/// Haversine distance between two coordinates in meters
pub fn haversine(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    const R: f64 = 6_371_000.0; // Earth radius in meters
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();

    let a = (dlat / 2.0).sin().powi(2)
        + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().asin();
    R * c
}

/// Haversine distance between two coordinates
pub fn coord_distance(c1: &Coordinate, c2: &Coordinate) -> f64 {
    haversine(c1.lat, c1.lon, c2.lat, c2.lon)
}

/// Calculate bearing from point1 to point2 in degrees (0-360)
pub fn bearing(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let x = dlon.sin() * lat2_rad.cos();
    let y = lat1_rad.cos() * lat2_rad.sin() - lat1_rad.sin() * lat2_rad.cos() * dlon.cos();

    (x.atan2(y).to_degrees() + 360.0) % 360.0
}

/// Calculate bearing between two coordinates
pub fn calculate_bearing(c1: &Coordinate, c2: &Coordinate) -> f64 {
    bearing(c1.lat, c1.lon, c2.lat, c2.lon)
}

/// Check if two coordinates are within tolerance distance (meters)
pub fn within_tolerance(c1: &Coordinate, c2: &Coordinate, tolerance_meters: f64) -> bool {
    coord_distance(c1, c2) <= tolerance_meters
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_haversine_known_distance() {
        // Montreal to Quebec City ≈ 233 km
        let d = haversine(45.5017, -73.5673, 46.8139, -71.2080);
        assert!((d - 233_000.0).abs() < 5_000.0, "Expected ~233km, got {:.0}m", d);
    }

    #[test]
    fn test_haversine_same_point() {
        let d = haversine(45.5, -73.5, 45.5, -73.5);
        assert!(d < 0.01);
    }

    #[test]
    fn test_bearing() {
        // Bearing from (0,0) to (1,0) should be approximately 0 degrees (north)
        let b = bearing(0.0, 0.0, 1.0, 0.0);
        assert!((b - 0.0).abs() < 1.0);
    }

    #[test]
    fn test_within_tolerance() {
        let c1 = Coordinate::new(45.5, -73.5);
        let c2 = Coordinate::new(45.50001, -73.50001);
        assert!(within_tolerance(&c1, &c2, 10.0)); // Should be within 10m
        assert!(!within_tolerance(&c1, &c2, 0.01)); // Not within 1cm
    }
}
