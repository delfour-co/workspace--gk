//! mail-rs: Secure SMTP/IMAP mail server
//!
//! A high-performance, security-focused mail server written in Rust.
//!
//! # Features
//!
//! - **SMTP Server**: Receive emails via SMTP protocol (RFC 5321)
//! - **Security**: Comprehensive input validation and resource limits
//! - **Performance**: Async/await with Tokio for high concurrency
//! - **Storage**: Maildir format for reliability
//!
//! # Security Features
//!
//! - Input validation (email addresses, commands, data)
//! - Resource limits (message size, recipients, line length)
//! - Timeout protection against slowloris attacks
//! - Error tracking and automatic disconnection
//!
//! # Example
//!
//! ```no_run
//! use mail_rs::config::Config;
//! use mail_rs::smtp::SmtpServer;
//! use mail_rs::storage::MaildirStorage;
//! use std::sync::Arc;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = Config::default();
//!     let storage = Arc::new(MaildirStorage::new(
//!         config.storage.maildir_path.clone()
//!     ));
//!
//!     let server = SmtpServer::new(config, storage);
//!     server.run().await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! # Modules
//!
//! - [`config`]: Configuration management
//! - [`error`]: Error types and handling
//! - [`smtp`]: SMTP protocol implementation
//! - [`storage`]: Email storage backends
//! - [`security`]: TLS and authentication
//! - [`utils`]: Utility functions (validation, etc.)

pub mod api;
pub mod authentication;
pub mod config;
pub mod error;
pub mod imap;
pub mod mime;
pub mod security;
pub mod smtp;
pub mod storage;
pub mod utils;

// Re-export commonly used types
pub use config::Config;
pub use error::{MailError, Result};
