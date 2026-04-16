use crate::geo::types::Coordinate;
use crate::optimizer::types::Way;
use anyhow::Result;
use std::collections::HashMap;

/// Result of parsing GeoJSON: ways plus a spatial registry mapping node IDs to coordinates.
pub struct ParseResult {
    pub ways: Vec<Way>,
    pub spatial_registry: HashMap<String, Coordinate>,
}

/// Parse GeoJSON FeatureCollection into Way structs and a spatial registry.
pub fn parse_ways_from_geojson(geojson: &serde_json::Value) -> Result<ParseResult> {
    let features = geojson
        .get("features")
        .and_then(|f| f.as_array())
        .ok_or_else(|| anyhow::anyhow!("Missing features array"))?;

    let mut ways = Vec::new();
    let mut spatial_registry: HashMap<String, Coordinate> = HashMap::new();

    for (i, feature) in features.iter().enumerate() {
        let geometry = feature.get("geometry")
            .ok_or_else(|| anyhow::anyhow!("Feature {} has no geometry", i))?;

        let coords = geometry.get("coordinates")
            .and_then(|c| c.as_array())
            .ok_or_else(|| anyhow::anyhow!("Feature {} has no coordinates", i))?;

        // Build node IDs from coordinates and populate spatial registry
        let nodes: Vec<String> = coords.iter()
            .filter_map(|c| {
                let arr = c.as_array()?;
                let lon = arr.get(0)?.as_f64()?;
                let lat = arr.get(1)?.as_f64()?;
                let key = format!("{:.6}_{:.6}", lon, lat);
                spatial_registry.entry(key.clone())
                    .or_insert_with(|| Coordinate::new(lat, lon));
                Some(key)
            })
            .collect();

        // Extract tags from properties
        let properties = feature.get("properties")
            .and_then(|p| p.as_object());
        let mut tags = HashMap::new();
        if let Some(props) = properties {
            for (k, v) in props {
                if let Some(s) = v.as_str() {
                    tags.insert(k.clone(), s.to_string());
                }
            }
        }

        ways.push(Way {
            id: feature.get("id")
                .and_then(|id| id.as_str())
                .unwrap_or(&format!("way_{}", i))
                .to_string(),
            nodes,
            tags,
        });
    }

    Ok(ParseResult { ways, spatial_registry })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_geojson() {
        let geojson = serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[-73.5, 45.5], [-73.6, 45.6]]
                    },
                    "properties": {
                        "name": "Test Street"
                    }
                }
            ]
        });

        let result = parse_ways_from_geojson(&geojson).unwrap();
        assert_eq!(result.ways.len(), 1);
        assert_eq!(result.ways[0].nodes.len(), 2);
        assert_eq!(result.ways[0].tags.get("name"), Some(&"Test Street".to_string()));
        // Verify spatial registry was populated
        assert_eq!(result.spatial_registry.len(), 2);
        let key1 = "-73.500000_45.500000".to_string();
        assert!(result.spatial_registry.contains_key(&key1));
        let coord = &result.spatial_registry[&key1];
        assert!((coord.lat - 45.5).abs() < 1e-10);
        assert!((coord.lon - (-73.5)).abs() < 1e-10);
    }

    #[test]
    fn test_parse_deduplicates_nodes() {
        let geojson = serde_json::json!({
            "type": "FeatureCollection",
            "features": [
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[-73.5, 45.5], [-73.6, 45.6]]
                    }
                },
                {
                    "type": "Feature",
                    "geometry": {
                        "type": "LineString",
                        "coordinates": [[-73.6, 45.6], [-73.7, 45.7]]
                    }
                }
            ]
        });

        let result = parse_ways_from_geojson(&geojson).unwrap();
        assert_eq!(result.ways.len(), 2);
        // 3 unique coordinates across 2 ways (middle node is shared)
        assert_eq!(result.spatial_registry.len(), 3);
    }

    #[test]
    fn test_parse_empty_features() {
        let geojson = serde_json::json!({
            "type": "FeatureCollection",
            "features": []
        });

        let result = parse_ways_from_geojson(&geojson).unwrap();
        assert_eq!(result.ways.len(), 0);
        assert!(result.spatial_registry.is_empty());
    }
}
