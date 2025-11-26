//! REST API module for mail-rs
//!
//! Provides HTTP API endpoints for email operations

pub mod admin;
pub mod auth;
pub mod handlers;
pub mod metrics;
pub mod server;

pub use metrics::Metrics;
pub use server::ApiServer;
