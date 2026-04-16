use crate::client::RmpClient;
use anyhow::Result;
use clap::Args;
use serde::Serialize;

#[derive(Debug, Args)]
pub struct StatusArgs {
    /// Ping HTTP health endpoints
    #[arg(long)]
    health: bool,

    /// Filter to a specific jail or service name
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
    address: String,
    health: String,
}

/// Jail definitions derived from Config defaults.
fn jail_definitions(client: &RmpClient) -> Vec<(&'static str, &'static str, String, &'static str)> {
    vec![
        ("rmpca-extract",   "extract",   format!("{}:4000", client.config.rmpca_extract_host),    "/"),
        ("rmpca-backend",   "backend",   format!("{}:3000", client.config.rmpca_backend_host),    "/health"),
        ("rmpca-optimizer", "optimizer", format!("{}:{}", client.config.rmpca_optimizer_host, client.config.rmpca_optimizer_port), "/health"),
    ]
}

pub async fn run(args: StatusArgs, client: &RmpClient) -> Result<()> {
    let jails = jail_definitions(client);
    let mut results = Vec::new();

    for (jail, service, address, health_path) in &jails {
        if let Some(ref filter) = args.jail {
            if *jail != filter.as_str() && *service != filter.as_str() {
                continue;
            }
        }

        let health_status = if args.health {
            let url = format!("http://{}{}", address, health_path);
            match client.health_check(&url).await {
                Ok(code) if code < 400 => format!("ok ({})", code),
                Ok(code) => format!("fail ({})", code),
                Err(_) => "unreachable".to_string(),
            }
        } else {
            "–".to_string()
        };

        results.push(JailStatus {
            jail: jail.to_string(),
            service: service.to_string(),
            address: address.clone(),
            health: health_status,
        });
    }

    if args.json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else if args.quiet {
        for r in &results {
            println!("{}\t{}", r.jail, r.health);
        }
    } else {
        println!("{:<24} {:<12} {:<20} {}", "JAIL", "SERVICE", "ADDRESS", "HEALTH");
        println!("{}", "─".repeat(70));
        for r in &results {
            println!("{:<24} {:<12} {:<20} {}", r.jail, r.service, r.address, r.health);
        }
    }

    Ok(())
}
