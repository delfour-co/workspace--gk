//! Error types for proxy-rs

use thiserror::Error;

/// Result type alias for proxy operations
pub type Result<T> = std::result::Result<T, ProxyError>;

/// Proxy error types
#[derive(Error, Debug)]
pub enum ProxyError {
    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Backend connection error
    #[error("Backend connection error: {0}")]
    BackendConnection(String),

    /// TLS error
    #[error("TLS error: {0}")]
    Tls(String),

    /// Route not found
    #[error("No route found for {host}{path}")]
    RouteNotFound { host: String, path: String },

    /// Backend unhealthy
    #[error("Backend unhealthy: {0}")]
    BackendUnhealthy(String),
}
