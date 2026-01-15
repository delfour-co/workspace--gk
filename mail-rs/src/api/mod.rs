//! REST API module for mail-rs
//!
//! Provides HTTP API endpoints for email operations

pub mod admin;
pub mod auth;
pub mod auto_reply;
pub mod caldav;
pub mod greylisting;
pub mod handlers;
pub mod import_export;
pub mod metrics;
pub mod mfa;
pub mod monitoring;
pub mod quotas;
pub mod search;
pub mod security_stats;
pub mod server;
pub mod sieve;
pub mod spam;
pub mod templates;
pub mod web;

pub use metrics::Metrics;
pub use server::ApiServer;
