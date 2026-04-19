use crate::client::RmpClient;
use anyhow::{Context, Result};
use clap::Args;
use geojson::{Feature, FeatureCollection, Value};
use std::collections::HashSet;
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

/// Clean a FeatureCollection by removing degenerate and duplicate segments.
///
/// - `dedupe`: drop LineStrings whose coordinate sequence (forward or reversed) was already seen
/// - `remove_self_loops`: drop LineStrings where every consecutive pair of vertices is identical
/// - `snap_precision`: round each coordinate to the nearest multiple of this value (decimal degrees)
/// - `min_segment_length`: drop LineStrings shorter than this many meters (uses equirectangular approx)
pub fn clean_feature_collection(
    fc: FeatureCollection,
    dedupe: bool,
    remove_self_loops: bool,
    snap_precision: Option<f64>,
    min_segment_length: Option<f64>,
) -> FeatureCollection {
    let FeatureCollection { features, bbox, foreign_members } = fc;
    let mut seen: HashSet<String> = HashSet::new();

    let cleaned: Vec<Feature> = features
        .into_iter()
        .filter_map(|mut feature| {
            let geometry = feature.geometry.as_mut()?;

            if let Value::LineString(ref mut coords) = geometry.value {
                // Snap vertices to grid before any other checks
                if let Some(precision) = snap_precision {
                    for coord in coords.iter_mut() {
                        if coord.len() >= 2 {
                            coord[0] = (coord[0] / precision).round() * precision;
                            coord[1] = (coord[1] / precision).round() * precision;
                        }
                    }
                }

                // Drop self-loops: segments where every vertex pair is the same point
                if remove_self_loops {
                    if coords.len() < 2 || coords.windows(2).all(|w| w[0] == w[1]) {
                        return None;
                    }
                }

                // Drop segments below the minimum length threshold
                if let Some(min_len) = min_segment_length {
                    let total_deg: f64 = coords.windows(2).map(|w| {
                        if w[0].len() >= 2 && w[1].len() >= 2 {
                            let dx = w[1][0] - w[0][0];
                            let dy = w[1][1] - w[0][1];
                            (dx * dx + dy * dy).sqrt()
                        } else {
                            0.0
                        }
                    }).sum();
                    // 1 degree ≈ 111,320 m (equirectangular, good enough for deduplication)
                    if total_deg * 111_320.0 < min_len {
                        return None;
                    }
                }

                // Deduplicate: canonicalize direction so A→B and B→A hash the same
                if dedupe {
                    let forward = format!("{:?}", coords);
                    let mut rev = coords.clone();
                    rev.reverse();
                    let reverse = format!("{:?}", rev);
                    let key = std::cmp::min(forward, reverse);
                    if !seen.insert(key) {
                        return None;
                    }
                }
            }

            Some(feature)
        })
        .collect();

    FeatureCollection { features: cleaned, bbox, foreign_members }
}

pub async fn run(args: CleanArgs, _client: &RmpClient) -> Result<()> {
    let raw = std::fs::read_to_string(&args.input)
        .context("Failed to read input file")?;
    let fc: FeatureCollection = raw.parse()
        .context("Input is not a valid GeoJSON FeatureCollection")?;

    if !args.quiet {
        eprintln!("Cleaning {} features...", fc.features.len());
    }

    let cleaned = clean_feature_collection(
        fc,
        args.dedupe,
        args.remove_self_intersections,
        args.snap_precision,
        args.min_segment_length,
    );

    if !args.quiet {
        eprintln!("Retained {} features after cleaning.", cleaned.features.len());
    }

    let output_text = serde_json::to_string_pretty(&cleaned)?;
    match args.output {
        Some(path) => std::fs::write(&path, &output_text)
            .context("Failed to write output file")?,
        None => println!("{}", output_text),
    }

    Ok(())
}
