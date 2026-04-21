//! Status command: Show health/status of rmpca jails and services
//!
//! This command checks the status of all rmpca jails and services,
//! optionally performing HTTP health checks.

use crate::config::Config;
use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, ClapArgs)]
pub struct Args {
    /// Ping HTTP health/readiness endpoints
    #[arg(long)]
    health: bool,

    /// Filter to a specific jail
    #[arg(long)]
    jail: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Minimal output (for scripting)
    #[arg(short, long)]
    quiet: bool,
}

#[derive(Debug, Serialize)]
struct JailStatus {
    jail: String,
    service: String,
    jail_status: String,
    service_status: String,
    address: String,
    health: String,
}

/// Show health/status of all rmpca jails and services
pub async fn run(args: Args) -> Result<()> {
    let config = Config::load().unwrap_or_default();
    config.init_logging();

    tracing::info!("Checking jail status");

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()?;

    // Define jails and services
    let jails = vec![
        ("rmpca-extract", "extract", "10.10.0.2", Some(4000), Some("/")),
        ("rmpca-backend", "backend", "10.10.0.3", Some(3000), Some("/health")),
        ("rmpca-optimizer", "optimizer", "10.10.0.5", Some(8000), Some("/health")),
        ("rmpca-nginx-opt", "nginx", "10.10.0.7", Some(80), Some("/health")),
    ];

    let mut results = Vec::new();

    for (jail, service, ip, port, health_path) in jails {
        // Apply filter
        if let Some(ref filter) = args.jail {
            if jail != filter && service != filter {
                continue;
            }
        }

        let address = match port {
            Some(p) => format!("{}:{}", ip, p),
            None => ip.to_string(),
        };

        // Health check (HTTP)
        let health_status = if args.health && port.is_some() && health_path.is_some() {
            let url = format!("http://{}{}", address, health_path.unwrap());
            match client.get(&url).send().await {
                Ok(resp) if resp.status().is_success() => {
                    format!("ok ({})", resp.status())
                }
                Ok(resp) => format!("fail ({})", resp.status()),
                Err(_) => "fail (connection error)".to_string(),
            }
        } else {
            "–".to_string()
        };

        results.push(JailStatus {
            jail: jail.to_string(),
            service: service.to_string(),
            jail_status: "up".to_string(), // TODO: Check actual jail status
            service_status: "running".to_string(), // TODO: Check actual service status
            address,
            health: health_status,
        });
    }

    // Output
    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else if args.quiet {
        for r in &results {
            println!("{} {} {}", r.jail, r.jail_status, r.service_status);
        }
    } else {
        println!("{:<24} {:<10} {:<10} {:<14} {}",
            "JAIL", "JAIL", "SERVICE", "IP:PORT", "HEALTH");
        println!("{}", "-".repeat(78));
        for r in &results {
            println!("{:<24} {:<10} {:<10} {:<14} {}",
                r.jail, r.jail_status, r.service_status, r.address, r.health);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_args() {
        let args = Args {
            health: true,
            jail: Some("rmpca-backend".to_string()),
            json: false,
            quiet: true,
        };
        assert!(args.health);
        assert_eq!(args.jail, Some("rmpca-backend".to_string()));
        assert!(args.quiet);
    }
}
