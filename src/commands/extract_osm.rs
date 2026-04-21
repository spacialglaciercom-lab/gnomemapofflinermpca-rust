//! Extract OSM data command
//!
//! This command downloads and converts OSM data to GeoJSON
//! for a given bounding box.

use crate::config::Config;
use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Bounding box: MIN_LON,MIN_LAT,MAX_LON,MAX_LAT
    #[arg(long)]
    bbox: String,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Highway class filter
    #[arg(long)]
    highway: Option<String>,
}

/// Extract OSM data to GeoJSON
pub async fn run(args: Args) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    config.init_logging();

    tracing::info!("Extracting OSM data for bbox: {}", args.bbox);

    // TODO: Implement OSM extraction
    // This will involve:
    // 1. Querying Overpass API with bounding box
    // 2. Parsing OSM XML response
    // 3. Converting to GeoJSON format
    // 4. Applying highway class filters

    tracing::warn!("OSM extraction not yet implemented");
    Err(anyhow::anyhow!("OSM extraction not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_osm_args() {
        let args = Args {
            bbox: "-73.59,45.49,-73.55,45.52".to_string(),
            output: None,
            highway: Some("primary,secondary".to_string()),
        };
        assert_eq!(args.bbox, "-73.59,45.49,-73.55,45.52");
        assert_eq!(args.highway, Some("primary,secondary".to_string()));
    }
}
