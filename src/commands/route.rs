//! `route` subcommand — offline point-to-point routing via a local .osm.pbf file
//!
//! Reads --from/--to coordinates, builds a road graph from the specified PBF
//! around that corridor, runs A* between the two nearest network nodes, and
//! writes a JSON response with path + turn instructions to stdout.
//! Progress events are written to stderr as JSON lines (same format as `serve`).

use anyhow::Result;
use clap::Args;
use serde::Serialize;
use std::io::Write;
use std::path::PathBuf;

use crate::config::Config;
use crate::geo::spatial::{bearing, coord_distance};
use crate::geo::types::Coordinate;
use crate::optimizer::RouteOptimizer;

#[derive(Debug, Args)]
pub struct RouteArgs {
    /// Origin as "LAT,LON"
    #[arg(long)]
    from: String,

    /// Destination as "LAT,LON"
    #[arg(long)]
    to: String,

    /// Path to .osm.pbf file
    #[arg(long)]
    map: PathBuf,

    /// Vehicle profile: car, truck, delivery
    #[arg(long, default_value = "car")]
    profile: String,

    /// Pretty-print JSON output
    #[arg(long)]
    pretty: bool,
}

// ── Response types ────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PathPoint {
    latitude: f64,
    longitude: f64,
}

#[derive(Serialize)]
struct Instruction {
    /// Numeric TurnPoint.Type value (matches route.js TurnPoint.Type enum)
    #[serde(rename = "type")]
    turn_type: u8,
    text: String,
    /// Distance in metres from the previous instruction to this one
    distance: f64,
    /// Index into the `path` array where this instruction applies
    coordinate_index: usize,
}

#[derive(Serialize)]
struct RouteResponse {
    success: bool,
    path: Vec<PathPoint>,
    instructions: Vec<Instruction>,
    distance_m: f64,
    duration_s: f64,
}

#[derive(Serialize)]
struct RouteError {
    success: bool,
    error: String,
}

#[derive(Serialize)]
struct ProgressEvent {
    event: &'static str,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    percent: Option<u8>,
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn write_error(msg: &str) -> ! {
    let err = RouteError { success: false, error: msg.to_string() };
    println!("{}", serde_json::to_string(&err).unwrap());
    std::process::exit(1);
}

fn write_progress(msg: &str, pct: Option<u8>) {
    let evt = ProgressEvent { event: "progress", message: msg.to_string(), percent: pct };
    eprintln!("{}", serde_json::to_string(&evt).unwrap());
}

fn parse_latlon(s: &str, flag: &str) -> Result<Coordinate> {
    let parts: Vec<f64> = s.split(',')
        .map(|p| p.trim().parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .map_err(|_| anyhow::anyhow!("{} '{}' is not valid LAT,LON", flag, s))?;
    anyhow::ensure!(parts.len() == 2, "{} requires exactly two values (LAT,LON)", flag);
    Ok(Coordinate::new(parts[0], parts[1]))
}

/// Average road speed in m/s for a given profile
fn profile_speed_ms(profile: &str) -> f64 {
    match profile {
        "truck"    => 60.0 / 3.6,
        "delivery" => 50.0 / 3.6,
        _          => 80.0 / 3.6,  // car
    }
}

/// Normalise a bearing difference to -180..+180 (negative = left, positive = right)
fn normalise_angle(a: f64) -> f64 {
    let mut v = a % 360.0;
    if v > 180.0  { v -= 360.0; }
    if v < -180.0 { v += 360.0; }
    v
}

/// Map a normalised turn angle to a TurnPoint.Type numeric value (route.js)
fn angle_to_turn_type(angle: f64) -> u8 {
    let abs = angle.abs();
    if abs < 20.0        { 5  }  // CONTINUE
    else if abs < 45.0   { if angle < 0.0 { 3 } else { 6 } }  // SLIGHT_LEFT / SLIGHT_RIGHT
    else if abs < 120.0  { if angle < 0.0 { 2 } else { 7 } }  // LEFT / RIGHT
    else if abs < 160.0  { if angle < 0.0 { 1 } else { 8 } }  // SHARP_LEFT / SHARP_RIGHT
    else                 { 14 }  // UTURN
}

fn turn_type_text(t: u8) -> &'static str {
    match t {
        1  => "Turn sharp left",
        2  => "Turn left",
        3  => "Turn slight left",
        5  => "Continue straight",
        6  => "Turn slight right",
        7  => "Turn right",
        8  => "Turn sharp right",
        14 => "Make a U-turn",
        _  => "Continue",
    }
}

/// Build turn instructions from the A* path.
///
/// Emits an instruction whenever the bearing changes by ≥15°.
/// Accumulates straight-ahead distance between turns.
fn build_instructions(path: &[(Coordinate, String)]) -> Vec<Instruction> {
    if path.is_empty() {
        return vec![];
    }

    let mut instrs = vec![Instruction {
        turn_type: 0,  // START
        text: "Depart".to_string(),
        distance: 0.0,
        coordinate_index: 0,
    }];

    if path.len() < 2 {
        instrs.push(Instruction {
            turn_type: 10,  // END
            text: "Arrive at destination".to_string(),
            distance: 0.0,
            coordinate_index: 0,
        });
        return instrs;
    }

    let mut accumulated = 0.0_f64;

    for i in 1..path.len() {
        let prev = &path[i - 1].0;
        let curr = &path[i].0;
        accumulated += coord_distance(prev, curr);

        if i == path.len() - 1 {
            // Last node — always emit END
            instrs.push(Instruction {
                turn_type: 10,
                text: "Arrive at destination".to_string(),
                distance: accumulated,
                coordinate_index: i,
            });
            break;
        }

        let next = &path[i + 1].0;

        // Skip degenerate segments (snapped duplicate coordinates)
        if coord_distance(prev, curr) < 0.5 {
            continue;
        }

        let in_bearing  = bearing(prev.lat, prev.lon, curr.lat, curr.lon);
        let out_bearing = bearing(curr.lat, curr.lon, next.lat, next.lon);
        let turn = normalise_angle(out_bearing - in_bearing);

        if turn.abs() >= 15.0 {
            let t = angle_to_turn_type(turn);
            instrs.push(Instruction {
                turn_type: t,
                text: turn_type_text(t).to_string(),
                distance: accumulated,
                coordinate_index: i,
            });
            accumulated = 0.0;
        }
    }

    instrs
}

// ── Entry point ───────────────────────────────────────────────────────────────

pub fn run(args: RouteArgs, _config: &Config) -> Result<()> {
    let from = parse_latlon(&args.from, "--from")
        .unwrap_or_else(|e| write_error(&e.to_string()));
    let to   = parse_latlon(&args.to,   "--to")
        .unwrap_or_else(|e| write_error(&e.to_string()));

    if !args.map.exists() {
        write_error(&format!("OSM PBF file not found: {}", args.map.display()));
    }

    write_progress("Parsing road network...", Some(10));

    // Derive a bounding box that contains both endpoints with 0.05° (~5.5 km) padding
    let pad = 0.05_f64;
    let bbox = (
        from.lon.min(to.lon) - pad,
        from.lat.min(to.lat) - pad,
        from.lon.max(to.lon) + pad,
        from.lat.max(to.lat) + pad,
    );

    let ways = crate::osm::parse_pbf_in_bbox(&args.map, bbox)
        .unwrap_or_else(|e| write_error(&format!("Failed to parse PBF: {}", e)));

    if ways.is_empty() {
        write_error("No roads found near the specified points. \
                     Check that the PBF file covers this area.");
    }

    write_progress(&format!("Loaded {} road segments. Building graph...", ways.len()), Some(40));

    let mut optimizer = RouteOptimizer::new();
    optimizer.populate_spatial_registry_from_geo_ways(&ways);
    optimizer.build_graph_from_geo_ways(&ways)
        .unwrap_or_else(|e| write_error(&e.to_string()));

    write_progress("Finding shortest path...", Some(70));

    let result = optimizer.route_between(&from, &to)
        .unwrap_or_else(|e| write_error(&e.to_string()));

    write_progress("Building turn instructions...", Some(90));

    let path: Vec<PathPoint> = result.path.iter()
        .map(|(c, _)| PathPoint { latitude: c.lat, longitude: c.lon })
        .collect();

    let instructions = build_instructions(&result.path);
    let duration_s   = result.distance_m / profile_speed_ms(&args.profile);

    let response = RouteResponse {
        success: true,
        path,
        instructions,
        distance_m: result.distance_m,
        duration_s,
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    if args.pretty {
        serde_json::to_writer_pretty(&mut out, &response)?;
    } else {
        serde_json::to_writer(&mut out, &response)?;
    }
    writeln!(out)?;

    write_progress("Done", Some(100));

    Ok(())
}
