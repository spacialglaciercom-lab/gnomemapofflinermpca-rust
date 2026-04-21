//! Clean command: Clean/repair GeoJSON files
//!
//! This command removes self-loops, duplicates, and short segments
//! from GeoJSON files.

use crate::config::Config;
use anyhow::Result;
use clap::Args as ClapArgs;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Input GeoJSON file
    input: PathBuf,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Minimum segment length in meters (default: 1.0)
    #[arg(long, default_value = "1.0")]
    min_length: f64,

    /// Print statistics
    #[arg(long)]
    stats: bool,
}

/// Clean/repair GeoJSON
pub async fn run(args: Args) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    config.init_logging();

    tracing::info!("Cleaning GeoJSON: {}", args.input.display());

    // TODO: Implement GeoJSON cleaning
    // This will involve:
    // 1. Removing self-loops (segments that start and end at same point)
    // 2. Removing duplicate segments
    // 3. Removing segments shorter than min_length
    // 4. Merging colinear segments

    tracing::warn!("GeoJSON cleaning not yet implemented");
    Err(anyhow::anyhow!("GeoJSON cleaning not yet implemented"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clean_args() {
        let args = Args {
            input: PathBuf::from("test.geojson"),
            output: None,
            min_length: 2.0,
            stats: true,
        };
        assert_eq!(args.input, PathBuf::from("test.geojson"));
        assert_eq!(args.min_length, 2.0);
        assert!(args.stats);
    }
}
