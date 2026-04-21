//! Extract Overture Maps road data command
//!
//! This command extracts road data from Overture Maps for a given
//! bounding box or polygon.

use crate::config::Config;
use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Bounding box: MIN_LON,MIN_LAT,MAX_LON,MAX_LAT
    #[arg(long)]
    bbox: Option<String>,

    /// Polygon file for Overture extraction
    #[arg(long)]
    polygon: Option<PathBuf>,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

/// Extract Overture Maps road data
pub async fn run(args: Args) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    config.init_logging();

    tracing::info!("Extracting Overture Maps data");

    // TODO: Implement Overture extraction
    // This will involve:
    // 1. Connecting to Overture Maps WebSocket API
    // 2. Sending bounding box or polygon query
    // 3. Receiving and parsing road data
    // 4. Converting to GeoJSON format

    tracing::warn!("Overture extraction not yet implemented");
    Err(anyhow::anyhow!("Overture extraction not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_overture_args() {
        let args = Args {
            bbox: Some("-73.59,45.49,-73.55,45.52".to_string()),
            polygon: None,
            output: None,
        };
        assert_eq!(args.bbox, Some("-73.59,45.49,-73.55,45.52".to_string()));
    }
}
