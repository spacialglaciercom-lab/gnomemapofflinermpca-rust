use crate::config::Config;
use anyhow::{Context, Result};
use reqwest::Client;
use serde_json::Value;
use std::time::Duration;

/// Shared HTTP client for all jail communication.
/// Built once in main, passed by reference to commands.
pub struct RmpClient {
    inner: Client,
    pub config: Config,
}

impl RmpClient {
    pub fn new(config: &Config) -> Result<Self> {
        let inner = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs()))
            .build()
            .context("Failed to build HTTP client")?;

        Ok(Self {
            inner,
            config: config.clone(),
        })
    }

    pub async fn get(&self, url: &str) -> Result<Value> {
        let resp = self.inner.get(url).send().await
            .context("HTTP GET failed")?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("HTTP {} from {}", status, url);
        }
        resp.json().await.context("Failed to parse response JSON")
    }

    pub async fn post_json(&self, url: &str, body: &Value) -> Result<Value> {
        let resp = self.inner.post(url).json(body).send().await
            .context("HTTP POST failed")?;
        let status = resp.status();
        if !status.is_success() {
            let error_body = resp.text().await.unwrap_or_default();
            anyhow::bail!("HTTP {} from {}: {}", status, url, error_body);
        }
        resp.json().await.context("Failed to parse response JSON")
    }

    /// Health check — returns Ok(status_code) even on non-2xx
    pub async fn health_check(&self, url: &str) -> Result<u16> {
        match self.inner.get(url).send().await {
            Ok(resp) => Ok(resp.status().as_u16()),
            Err(e) => Err(e.into()),
        }
    }

    pub fn raw(&self) -> &Client {
        &self.inner
    }
}
