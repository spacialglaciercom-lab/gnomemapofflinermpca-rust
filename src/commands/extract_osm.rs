use crate::client::RmpClient;
use anyhow::{Context, Result};
use clap::Args;
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ExtractOsmArgs {
    /// Bounding box: WEST,SOUTH,EAST,NORTH
    #[arg(long)]
    bbox: String,

    /// Output file (default: stdout)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Include specific highway types (default: all driveable)
    #[arg(long)]
    highway: Option<Vec<String>>,

    /// Suppress progress output
    #[arg(short, long)]
    quiet: bool,
}

pub async fn run(args: ExtractOsmArgs, client: &RmpClient) -> Result<()> {
    let url = format!("{}/api/extract/osm", client.config.extract_url());

    let parts: Vec<f64> = args.bbox.split(',')
        .map(|s| s.parse::<f64>())
        .collect::<std::result::Result<_, _>>()
        .context("Invalid bbox format")?;
    anyhow::ensure!(parts.len() == 4, "bbox requires exactly 4 values");

    let mut payload = serde_json::json!({ "bbox": parts });
    if let Some(highway) = &args.highway {
        payload["highway"] = serde_json::json!(highway);
    }

    if !args.quiet {
        eprintln!("Extracting OSM road data...");
    }

    let result = client.post_json(&url, &payload).await?;

    let output_text = serde_json::to_string_pretty(&result)?;
    match args.output {
        Some(path) => std::fs::write(&path, &output_text)
            .context("Failed to write output file")?,
        None => println!("{}", output_text),
    }

    Ok(())
}
