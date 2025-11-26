//! proxy-rs: HTTP Reverse Proxy with automatic TLS
//!
//! A high-performance reverse proxy for routing HTTP traffic
//! to backend services with automatic Let's Encrypt certificates.
//!
//! # Features
//!
//! - HTTP/HTTPS reverse proxy
//! - Automatic TLS with Let's Encrypt (ACME)
//! - Path-based and host-based routing
//! - Health checks for backends
//! - Request/response logging
//!
//! # Example Configuration
//!
//! ```toml
//! [server]
//! listen_addr = "0.0.0.0:443"
//! http_redirect_port = 80
//!
//! [tls]
//! acme_email = "admin@example.com"
//! acme_directory = "https://acme-v02.api.letsencrypt.org/directory"
//!
//! [[routes]]
//! host = "mail.example.com"
//! path_prefix = "/api"
//! backend = "http://localhost:8080"
//!
//! [[routes]]
//! host = "mail.example.com"
//! path_prefix = "/"
//! backend = "http://localhost:3000"
//! ```

pub mod acme;
pub mod config;
pub mod error;
pub mod health;
pub mod proxy;
pub mod router;
pub mod tls;

pub use config::ProxyConfig;
pub use error::{ProxyError, Result};
pub use proxy::ProxyServer;
