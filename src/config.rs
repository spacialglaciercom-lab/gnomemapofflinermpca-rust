//! Layered configuration system using figment
//!
//! Configuration priority (highest to lowest):
//! 1. CLI flags (via Clap derive)
//! 2. Environment variables (RMPCA_*)
//! 3. RouteMaster.toml (user config file)
//! 4. Hardcoded defaults

use clap::{Parser, ValueEnum};
use figment::{Figment, providers::{Env, Serialized, Format, Toml}};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Layered configuration: defaults → config file → env vars → CLI flags
///
/// Configuration priority (highest to lowest):
/// 1. CLI flags (via Clap derive)
/// 2. Environment variables (RMPCA_*)
/// 3. RouteMaster.toml (user config file)
/// 4. Hardcoded defaults
#[derive(Debug, Clone, Parser, Serialize, Deserialize)]
pub struct Config {
    // Network configuration
    /// Extract jail address (default: 10.10.0.2)
    #[arg(long, env = "RMPCA_EXTRACT_HOST", default_value = "10.10.0.2")]
    pub extract_host: String,

    /// Backend jail address (default: 10.10.0.3)
    #[arg(long, env = "RMPCA_BACKEND_HOST", default_value = "10.10.0.3")]
    pub backend_host: String,

    /// Optimizer nginx address (default: 10.10.0.7)
    #[arg(long, env = "RMPCA_OPTIMIZER_HOST", default_value = "10.10.0.7")]
    pub optimizer_host: String,

    /// Optimizer port (default: 8000)
    #[arg(long, default_value = "8000")]
    pub optimizer_port: u16,

    /// Request timeout in seconds (default: 120)
    #[arg(long, default_value = "120")]
    pub timeout_secs: u64,

    // Optimization profiles (configurable via RouteMaster.toml)
    /// Left turn penalty (configurable: 0-10)
    #[arg(long, default_value = "0")]
    pub turn_left_penalty: f64,

    /// Right turn penalty (configurable: 0-10)
    #[arg(long, default_value = "0")]
    pub turn_right_penalty: f64,

    /// U-turn penalty (configurable: 0-10)
    #[arg(long, default_value = "0")]
    pub turn_u_penalty: f64,

    // Caching configuration
    /// Cache directory for compiled graphs
    #[arg(long, env = "RMPCA_CACHE_DIR", default_value = "~/.cache/rmpca")]
    pub cache_dir: String,

    // Telemetry configuration
    /// Output structured JSON logs
    #[arg(long, env = "RMPCA_JSON_LOGS")]
    pub json_logs: bool,

    // Lean 4 verification mode
    /// Use Lean 4 verified optimization (if compiled with lean4 feature)
    #[arg(long, env = "RMPCA_LEAN4_VERIFIED")]
    pub lean4_verified: bool,
}

impl Config {
    /// Load configuration from all layers (defaults → config file → env vars → CLI flags)
    pub fn load() -> Result<Self, figment::Error> {
        // Start with defaults and environment variables
        let base: Figment = Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::prefixed("RMPCA_").split("_"));

        // Add config file if it exists
        let config_path = Self::config_path();
        let with_file = if config_path.exists() {
            base.merge(Toml::file(config_path))
        } else {
            base
        };

        // Extract final config
        with_file.extract()
    }

    /// Get path to user config file
    pub fn config_path() -> PathBuf {
        let mut path = Self::home_dir();
        path.push("RouteMaster.toml");
        path
    }

    /// Get home directory
    fn home_dir() -> PathBuf {
        #[cfg(feature = "dirs")]
        {
            dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
        }

        #[cfg(not(feature = "dirs"))]
        {
            PathBuf::from(".")
        }
    }

    /// Expand cache directory with home directory
    pub fn cache_dir_expanded(&self) -> PathBuf {
        let path = PathBuf::from(&self.cache_dir);
        if path.starts_with("~") {
            let mut home = Self::home_dir();
            if let Ok(rest) = path.strip_prefix("~") {
                home.push(rest);
            }
            home
        } else {
            path
        }
    }

    /// Initialize logging with JSON or pretty output
    pub fn init_logging(&self) {
        if self.json_logs {
            tracing_subscriber::fmt()
                .json()
                .init();
        } else {
            tracing_subscriber::fmt()
                .pretty()
                .init();
        }
    }

    // Helper methods for URL construction
    pub fn optimizer_url(&self) -> String {
        format!("http://{}:{}", self.optimizer_host, self.optimizer_port)
    }

    pub fn backend_url(&self) -> String {
        format!("http://{}:3000", self.backend_host)
    }

    pub fn extract_url(&self) -> String {
        format!("http://{}:4000", self.extract_host)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            extract_host: "10.10.0.2".to_string(),
            backend_host: "10.10.0.3".to_string(),
            optimizer_host: "10.10.0.7".to_string(),
            optimizer_port: 8000,
            timeout_secs: 120,
            turn_left_penalty: 0.0,
            turn_right_penalty: 0.0,
            turn_u_penalty: 0.0,
            cache_dir: "~/.cache/rmpca".to_string(),
            json_logs: false,
            lean4_verified: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.extract_host, "10.10.0.2");
        assert_eq!(config.backend_host, "10.10.0.3");
        assert_eq!(config.optimizer_host, "10.10.0.7");
        assert_eq!(config.optimizer_port, 8000);
    }

    #[test]
    fn test_url_construction() {
        let config = Config::default();
        assert_eq!(config.optimizer_url(), "http://10.10.0.7:8000");
        assert_eq!(config.backend_url(), "http://10.10.0.3:3000");
        assert_eq!(config.extract_url(), "http://10.10.0.2:4000");
    }
}
