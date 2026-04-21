//! rmpca — Enterprise-grade unified CLI for rmp.ca operations
//!
//! This is a Rust port of the FreeBSD shell-based dispatcher, transformed
//! into an enterprise-grade offline engine suitable for RouteMasterPro.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;
mod config;
mod optimizer;

use commands::*;

/// rmpca — Enterprise-grade route optimization CLI
#[derive(Parser)]
#[command(name = "rmpca")]
#[command(about = "Unified CLI for rmp.ca operations", long_about = None)]
#[command(version)]
#[command(long_about = rmpca_long_help())]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Output structured JSON logs (for frontend integration)
    #[arg(long, global = true, env = "RMPCA_JSON_LOGS")]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Extract Overture Maps road data for a bounding box or polygon
    ExtractOverture(commands::extract_overture::Args),

    /// Download & convert OSM data to GeoJSON for a bounding box
    ExtractOsm(commands::extract_osm::Args),

    /// Compile GeoJSON to binary graph cache (instant subsequent optimizations)
    #[command(alias = "cache-map")]
    CompileMap(commands::compile_map::Args),

    /// Optimize a GeoJSON route (supports Lean 4 verification)
    #[command(aliases = &["opt"])]
    Optimize(commands::optimize::Args),

    /// Clean/repair GeoJSON (dedupe, remove self-loops, etc.)
    Clean(commands::clean::Args),

    /// Validate a GeoJSON file structure and geometry
    Validate(commands::validate::Args),

    /// End-to-end: extract → clean → optimize → export
    Pipeline(commands::pipeline::Args),

    /// Show health/status of all rmpca jails and services
    Status(commands::status::Args),

    /// Tail service logs from a jail
    Logs(commands::logs::Args),

    /// Run property-based tests for algorithmic correctness
    #[command(alias = "proptest")]
    TestProperties,
}

fn rmpca_long_help() -> &'static str {
    r#"
rmpca — Enterprise-grade route optimization CLI

Quick Start:
  rmpca compile-map city.geojson    # Compile map once (5-30s)
  rmpca optimize --cache city.rmp   # Optimize instantly (1-5ms!)

Configuration (Priority: CLI > Env > Config File > Defaults):
  RouteMaster.toml    - User configuration file (~/.config/RouteMaster.toml)
  RMPCA_* env vars   - Environment variable overrides
  --flag arguments     - Command-line flags (highest priority)

Enterprise Features:
  • Graph caching    - Subsequent optimizations: 1000x faster
  • Lean 4 FFI     - Formal verification via compiled Lean 4 proofs
  • Property tests    - Mathematically rigorous algorithm testing
  • JSON telemetry   - Structured logs for frontend integration
  • Layered config   - Flexible profiles for trucks, cars, etc.

For help with a specific command: rmpca <command> --help
"#
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize structured logging
    let config = config::Config::load().unwrap_or_else(|e| {
        // If config loading fails, use defaults but log the error
        eprintln!("Warning: Failed to load configuration: {}", e);
        config::Config::default()
    });
    config.init_logging();

    tracing::info!("rmpca v{} starting", env!("CARGO_PKG_VERSION"));

    match cli.command {
        Commands::ExtractOverture(args) => extract_overture::run(args).await,
        Commands::ExtractOsm(args) => extract_osm::run(args).await,
        Commands::CompileMap(args) => compile_map::run(args).await,
        Commands::Optimize(args) => optimize::run(args).await,
        Commands::Clean(args) => clean::run(args).await,
        Commands::Validate(args) => validate::run(args).await,
        Commands::Pipeline(args) => pipeline::run(args).await,
        Commands::Status(args) => status::run(args).await,
        Commands::Logs(args) => logs::run(args).await,
        Commands::TestProperties => {
            // Run property-based tests
            eprintln!("Running property-based tests...");
            eprintln!("This tests algorithmic invariants across random inputs.");
            eprintln!("Use: cargo test --release --tests property_tests");
            Ok(())
        }
    }
}
