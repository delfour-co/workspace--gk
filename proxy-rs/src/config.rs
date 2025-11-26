//! Configuration for proxy-rs

use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::error::{ProxyError, Result};

/// Main proxy configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ProxyConfig {
    /// Server configuration
    pub server: ServerConfig,
    /// TLS configuration (optional)
    pub tls: Option<TlsConfig>,
    /// Route configurations
    pub routes: Vec<RouteConfig>,
}

/// Server configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    /// Listen address for HTTPS (e.g., "0.0.0.0:443")
    pub listen_addr: String,
    /// HTTP port for redirect (e.g., 80)
    #[serde(default = "default_http_port")]
    pub http_redirect_port: u16,
    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
}

/// TLS configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TlsConfig {
    /// Path to TLS certificate
    pub cert_path: Option<String>,
    /// Path to TLS private key
    pub key_path: Option<String>,
    /// ACME email for Let's Encrypt
    pub acme_email: Option<String>,
    /// ACME directory URL
    #[serde(default = "default_acme_directory")]
    pub acme_directory: String,
    /// Domains for certificate
    pub domains: Vec<String>,
}

/// Route configuration
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RouteConfig {
    /// Host to match (e.g., "mail.example.com")
    pub host: String,
    /// Path prefix to match (e.g., "/api")
    #[serde(default = "default_path_prefix")]
    pub path_prefix: String,
    /// Backend URL (e.g., "http://localhost:8080")
    pub backend: String,
    /// Strip path prefix before forwarding
    #[serde(default)]
    pub strip_prefix: bool,
    /// Health check path (e.g., "/health")
    pub health_check: Option<String>,
    /// Request timeout override
    pub timeout_seconds: Option<u64>,
}

fn default_http_port() -> u16 {
    80
}

fn default_timeout() -> u64 {
    30
}

fn default_acme_directory() -> String {
    "https://acme-v02.api.letsencrypt.org/directory".to_string()
}

fn default_path_prefix() -> String {
    "/".to_string()
}

impl ProxyConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ProxyError::Config(format!("Failed to read config file: {}", e)))?;

        toml::from_str(&content)
            .map_err(|e| ProxyError::Config(format!("Failed to parse config: {}", e)))
    }

    /// Create a default development configuration
    pub fn development() -> Self {
        Self {
            server: ServerConfig {
                listen_addr: "0.0.0.0:8443".to_string(),
                http_redirect_port: 8080,
                timeout_seconds: 30,
            },
            tls: None,
            routes: vec![
                RouteConfig {
                    host: "localhost".to_string(),
                    path_prefix: "/api".to_string(),
                    backend: "http://127.0.0.1:8080".to_string(),
                    strip_prefix: false,
                    health_check: Some("/api/health".to_string()),
                    timeout_seconds: None,
                },
                RouteConfig {
                    host: "localhost".to_string(),
                    path_prefix: "/".to_string(),
                    backend: "http://127.0.0.1:3000".to_string(),
                    strip_prefix: false,
                    health_check: None,
                    timeout_seconds: None,
                },
            ],
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<()> {
        if self.routes.is_empty() {
            return Err(ProxyError::Config("No routes configured".to_string()));
        }

        for route in &self.routes {
            // Validate backend URL
            url::Url::parse(&route.backend).map_err(|e| {
                ProxyError::Config(format!("Invalid backend URL '{}': {}", route.backend, e))
            })?;
        }

        Ok(())
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::development()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = ProxyConfig::default();
        assert!(!config.routes.is_empty());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_parse_config() {
        let toml = r#"
[server]
listen_addr = "0.0.0.0:443"

[[routes]]
host = "example.com"
path_prefix = "/api"
backend = "http://localhost:8080"
"#;
        let config: ProxyConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.server.listen_addr, "0.0.0.0:443");
        assert_eq!(config.routes.len(), 1);
    }
}
