use crate::client::RmpClient;
use anyhow::{Context, Result};
use clap::Args;
use geojson::{FeatureCollection, Geometry, Value as GeoValue};
use std::path::PathBuf;

#[derive(Debug, Args)]
pub struct ValidateArgs {
    /// Input GeoJSON file
    input: PathBuf,

    /// Output issues as JSON
    #[arg(long)]
    json: bool,

    /// Strict mode: warn on non-LineString geometries
    #[arg(long)]
    strict: bool,
}

pub async fn run(args: ValidateArgs, _client: &RmpClient) -> Result<()> {
    let raw = std::fs::read_to_string(&args.input)
        .context("Failed to read input file")?;
    let fc: FeatureCollection = raw.parse()
        .context("Input is not a valid GeoJSON FeatureCollection")?;

    let mut issues: Vec<String> = Vec::new();

    for (i, feature) in fc.features.iter().enumerate() {
        // Check for missing geometry
        if feature.geometry.is_none() {
            issues.push(format!("Feature {}: missing geometry", i));
            continue;
        }

        let geom = feature.geometry.as_ref().unwrap();
        match &geom.value {
            GeoValue::LineString(coords) => {
                if coords.len() < 2 {
                    issues.push(format!("Feature {}: LineString with < 2 points", i));
                }
            }
            other => {
                if args.strict {
                    issues.push(format!("Feature {}: unexpected geometry type {:?}", i, other));
                }
            }
        }
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&serde_json::json!({
            "valid": issues.is_empty(),
            "feature_count": fc.features.len(),
            "issues": issues,
        }))?);
    } else {
        if issues.is_empty() {
            eprintln!("Valid: {} features, no issues", fc.features.len());
        } else {
            eprintln!("{} issues found:", issues.len());
            for issue in &issues {
                eprintln!("  - {}", issue);
            }
        }
    }

    Ok(())
}
