//! Optimize command: Route optimization with Lean 4 FFI integration
//!
//! This command provides route optimization with support for:
//! - Graph caching for instant loading
//! - Lean 4 verified optimization (via FFI)
//! - GPX export
//! - Structured JSON telemetry

use crate::config::Config;
use crate::optimizer::ffi::Lean4Bridge;
use crate::optimizer::RouteOptimizer;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use std::path::PathBuf;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Input GeoJSON file OR compiled .rmp cache file
    input: PathBuf,

    /// Output file (default: stdout as JSON)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Export result as GPX instead of GeoJSON
    #[arg(long)]
    gpx: bool,

    /// Clean/repair GeoJSON before optimizing
    #[arg(long)]
    clean: bool,

    /// Use compiled graph cache (faster for repeated optimizations)
    #[arg(long)]
    cache: Option<PathBuf>,

    /// Optimizer host (overrides env)
    #[arg(long)]
    host: Option<String>,

    /// Left turn penalty (overrides config)
    #[arg(long)]
    turn_left: Option<f64>,

    /// Right turn penalty (overrides config)
    #[arg(long)]
    turn_right: Option<f64>,

    /// U-turn penalty (overrides config)
    #[arg(long)]
    turn_u: Option<f64>,

    /// Starting depot location (LAT,LON)
    #[arg(long)]
    depot: Option<String>,

    /// Use Lean 4 verified optimization (requires --feature lean4)
    #[arg(long)]
    verified: bool,
}

/// Optimize route from GeoJSON or compiled cache
pub async fn run(args: Args) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    config.init_logging();

    tracing::info!("Starting optimization: {}", args.input.display());

    // Load configuration overrides from CLI
    let turn_left = args.turn_left.unwrap_or(config.turn_left_penalty);
    let turn_right = args.turn_right.unwrap_or(config.turn_right_penalty);
    let turn_u = args.turn_u.unwrap_or(config.turn_u_penalty);

    // Check for verified mode
    let verified = args.verified || config.lean4_verified;

    #[cfg(feature = "lean4")]
    if verified {
        tracing::info!("Using Lean 4 verified optimization");
        return optimize_verified(&args, &config, turn_left, turn_right, turn_u).await;
    }

    #[cfg(feature = "lean4")]
    tracing::info!("Lean 4 feature available but not requested, using Rust implementation");

    #[cfg(not(feature = "lean4"))]
    if verified {
        tracing::warn!("Lean 4 verification requested but not compiled with --feature lean4");
        tracing::warn!("Falling back to Rust implementation");
    }

    // Standard Rust implementation
    optimize_rust(&args, &config, turn_left, turn_right, turn_u).await
}

/// Rust implementation (port of offline-optimizer-v2 and Python backend)
async fn optimize_rust(
    args: &Args,
    config: &Config,
    _turn_left: f64,
    _turn_right: f64,
    _turn_u: f64,
) -> Result<()> {
    tracing::debug!("Using Rust optimizer implementation");

    // Load input (GeoJSON or compiled .rmp)
    let mut optimizer = if args.input.extension().map_or(false, |e| e == "rmp") {
        // Load from cache (instant)
        tracing::info!("Loading compiled graph cache");
        load_cached_graph(&args.input)?
    } else {
        // Parse GeoJSON (slower)
        tracing::info!("Parsing GeoJSON");
        let geojson_str = std::fs::read_to_string(&args.input)?;
        let feature_collection: geojson::FeatureCollection = geojson_str.parse()?;
        let mut optimizer = RouteOptimizer::new();
        optimizer.build_graph_from_features(&feature_collection.features)?;
        optimizer
    };

    // Apply turn penalties
    optimizer.set_turn_penalties(_turn_left, _turn_right, _turn_u);

    // Parse depot if specified
    if let Some(depot) = &args.depot {
        let coords: Vec<&str> = depot.split(',').collect();
        if coords.len() == 2 {
            optimizer.set_depot(
                coords[0].parse::<f64>()?,
                coords[1].parse::<f64>()?,
            );
        }
    }

    // Run optimization
    tracing::info!("Computing Eulerian circuit...");
    let result = optimizer.optimize()?;

    // Emit structured telemetry
    emit_telemetry(&result, config.json_logs);

    // Output
    write_output(&result, &args.output, args.gpx)?;

    Ok(())
}

/// Lean 4 verified implementation via FFI
#[cfg(feature = "lean4")]
async fn optimize_verified(
    args: &Args,
    config: &Config,
    _turn_left: f64,
    _turn_right: f64,
    _turn_u: f64,
) -> Result<()> {
    tracing::debug!("Using Lean 4 verified optimizer");

    // Load input (same as Rust implementation)
    let optimizer = if args.input.extension().map_or(false, |e| e == "rmp") {
        load_cached_graph(&args.input)?
    } else {
        let geojson_str = std::fs::read_to_string(&args.input)?;
        let feature_collection: geojson::FeatureCollection = geojson_str.parse()?;
        let mut optimizer = RouteOptimizer::new();
        optimizer.build_graph_from_features(&feature_collection.features)?;
        optimizer
    };

    // Apply configuration
    optimizer.set_turn_penalties(_turn_left, _turn_right, _turn_u);
    if let Some(depot) = &args.depot {
        let coords: Vec<&str> = depot.split(',').collect();
        if coords.len() == 2 {
            optimizer.set_depot(
                coords[0].parse::<f64>()?,
                coords[1].parse::<f64>()?,
            );
        }
    }

    // FFI BOUNDARY: Flatten graph for Lean 4
    tracing::info!("Preparing graph for Lean 4 verification");
    let flat_graph = optimizer.flatten_for_ffi();

    tracing::info!("Graph: {} nodes, {} edges", flat_graph.node_count, flat_graph.edge_count);

    // Call Lean 4 via FFI bridge
    let bridge = Lean4Bridge::new()?;
    let result = unsafe {
        let dummy_node = crate::optimizer::types::Node::new("", 0.0, 0.0);
        dummy_node.flatten_for_ffi(); // This is a placeholder
        bridge.optimize_lean4(
            flat_graph.nodes,
            flat_graph.node_count,
            flat_graph.edges,
            flat_graph.edge_count,
            flat_graph.start_node,
        )?
    };

    // Convert verified result back to Rust types
    let opt_result = optimizer.from_verified_result(result)?;

    // Emit structured telemetry
    emit_telemetry(&opt_result, config.json_logs);

    // Output
    write_output(&opt_result, &args.output, args.gpx)?;

    Ok(())
}

/// Load graph from compiled cache (instant)
fn load_cached_graph(path: &PathBuf) -> Result<RouteOptimizer> {
    let start = std::time::Instant::now();

    let bytes = std::fs::read(path)?;
    let optimizer = bincode::deserialize(&bytes)?;

    tracing::debug!("Loaded cache in {:?}", start.elapsed());
    Ok(optimizer)
}

/// Emit structured telemetry (JSON or human-readable)
fn emit_telemetry(result: &crate::optimizer::OptimizationResult, json: bool) {
    if json {
        let telemetry = serde_json::json!({
            "event": "optimization_complete",
            "total_distance_km": result.total_distance,
            "route_length": result.route.len(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });
        println!("{}", telemetry);
    } else {
        tracing::info!("Optimization complete");
        tracing::info!("Total distance: {:.2} km", result.total_distance);
        tracing::info!("Route points: {}", result.route.len());
    }
}

/// Write output to file or stdout
fn write_output(
    result: &crate::optimizer::OptimizationResult,
    output: &Option<PathBuf>,
    gpx: bool,
) -> Result<()> {
    let output_text = if gpx {
        convert_to_gpx(result)?
    } else {
        serde_json::to_string_pretty(result)?
    };

    match output {
        Some(path) => std::fs::write(path, output_text)?,
        None => println!("{}", output_text),
    }

    Ok(())
}

fn convert_to_gpx(result: &crate::optimizer::OptimizationResult) -> Result<String> {
    let mut gpx = String::from(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<gpx version="1.1" creator="rmpca-optimize">
  <trk><name>Optimized Route</name><trkseg>"#,
    );

    for point in &result.route {
        gpx.push_str(&format!(
            r#"    <trkpt lat="{}" lon="{}" />"#,
            point.latitude, point.longitude
        ));
    }

    gpx.push_str(r#"  </trkseg></trk>
</gpx>"#);

    Ok(gpx)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimize_args() {
        let args = Args {
            input: PathBuf::from("test.geojson"),
            output: None,
            gpx: false,
            clean: false,
            cache: None,
            host: None,
            turn_left: Some(1.0),
            turn_right: Some(0.5),
            turn_u: Some(2.0),
            depot: Some("45.5,-73.6".to_string()),
            verified: false,
        };
        assert_eq!(args.turn_left, Some(1.0));
        assert_eq!(args.depot, Some("45.5,-73.6".to_string()));
    }

    #[test]
    fn test_gpx_conversion() {
        let result = crate::optimizer::OptimizationResult::new(vec![
            crate::optimizer::RoutePoint::new(45.5, -73.6),
            crate::optimizer::RoutePoint::new(45.51, -73.61),
        ], 1.0);

        let gpx = convert_to_gpx(&result).unwrap();
        assert!(gpx.contains("<?xml version=\"1.0\""));
        assert!(gpx.contains("<gpx"));
        assert!(gpx.contains("<trk>"));
        assert!(gpx.contains("lat=\"45.5\""));
        assert!(gpx.contains("lon=\"-73.6\""));
    }
}
