//! SMTP server and client implementation (RFC 5321)
//!
//! This module provides a complete SMTP implementation:
//! - [`server`]: SMTP server accepting incoming mail
//! - [`client`]: SMTP client for sending outgoing mail
//! - [`session`]: SMTP session state machine
//! - [`commands`]: SMTP command parsing and handling
//! - [`queue`]: Message queue for outgoing emails

pub mod client;
pub mod commands;
pub mod queue;
pub mod server;
pub mod session;

pub use client::SmtpClient;
pub use commands::SmtpCommand;
pub use queue::{QueueStatus, QueuedEmail, SmtpQueue};
pub use server::SmtpServer;
pub use session::SmtpSession;
