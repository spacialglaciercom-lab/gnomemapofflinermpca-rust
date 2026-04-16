use crate::client::RmpClient;
use anyhow::{Context, Result};
use clap::Args;
use tokio_stream::StreamExt;

#[derive(Debug, Args)]
pub struct LogsArgs {
    /// Jail or service name to fetch logs from
    service: String,

    /// Number of lines to tail (default: 50)
    #[arg(short = 'n', long, default_value = "50")]
    lines: usize,

    /// Follow (stream) new log lines
    #[arg(short, long)]
    follow: bool,

    /// Filter log lines containing this string
    #[arg(long)]
    grep: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

pub async fn run(args: LogsArgs, client: &RmpClient) -> Result<()> {
    let url = format!(
        "{}/api/logs/{}?lines={}&follow={}",
        client.config.backend_url(),
        args.service,
        args.lines,
        args.follow,
    );

    if args.follow {
        // Streaming mode: read response as byte stream
        let resp = client.raw().get(&url).send().await
            .context("Failed to connect for log streaming")?;

        let mut stream = resp.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk
                .context("Stream error")
                .map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&bytes);
            for line in text.lines() {
                if let Some(ref pattern) = args.grep {
                    if !line.contains(pattern.as_str()) { continue; }
                }
                println!("{}", line);
            }
        }
    } else {
        let result = client.get(&url).await?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else if let Some(lines) = result.get("lines").and_then(|l| l.as_array()) {
            for line in lines {
                let text = line.as_str().unwrap_or("");
                if let Some(ref pattern) = args.grep {
                    if !text.contains(pattern.as_str()) { continue; }
                }
                println!("{}", text);
            }
        }
    }

    Ok(())
}
