//! REST API module for mail-rs
//!
//! Provides HTTP API endpoints for email operations

pub mod admin;
pub mod auth;
pub mod auto_reply;
pub mod greylisting;
pub mod handlers;
pub mod metrics;
pub mod monitoring;
pub mod quotas;
pub mod security_stats;
pub mod server;
pub mod templates;
pub mod web;

pub use metrics::Metrics;
pub use server::ApiServer;
