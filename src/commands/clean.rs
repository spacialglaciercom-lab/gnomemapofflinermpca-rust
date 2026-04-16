use crate::client::RmpClient;
use anyhow::{Context, Result};
use clap::Args;
use geojson::FeatureCollection;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct CleanArgs {
    /// Input GeoJSON file
    input: PathBuf,

    /// Output file (if not specified, prints to stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Remove duplicate geometries
    #[arg(long, default_value = "true")]
    dedupe: bool,

    /// Remove self-intersecting segments
    #[arg(long, default_value = "true")]
    remove_self_intersections: bool,

    /// Snap vertices to grid (precision in decimal degrees)
    #[arg(long)]
    snap_precision: Option<f64>,

    /// Minimum segment length in meters
    #[arg(long)]
    min_segment_length: Option<f64>,

    /// Suppress progress output
    #[arg(short, long)]
    quiet: bool,
}

pub async fn run(args: CleanArgs, _client: &RmpClient) -> Result<()> {
    let raw = std::fs::read_to_string(&args.input)
        .context("Failed to read input file")?;
    let fc: FeatureCollection = raw.parse()
        .context("Input is not a valid GeoJSON FeatureCollection")?;

    if !args.quiet {
        eprintln!("Cleaning {} features...", fc.features.len());
    }

    // TODO: Implement cleaning logic
    // 1. Deduplicate segments by coordinate comparison
    // 2. Remove self-loops (edges where start == end node)
    // 3. Snap nearby nodes within tolerance
    // 4. Remove zero-length segments
    // 5. Merge collinear segments on same street

    let output_text = serde_json::to_string_pretty(&fc)?;
    match args.output {
        Some(path) => std::fs::write(&path, &output_text)
            .context("Failed to write output file")?,
        None => println!("{}", output_text),
    }

    Ok(())
}
