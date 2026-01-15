//! Migration module for mailbox import/export
//!
//! Supports mbox and EML formats for mailbox migration.

pub mod eml;
pub mod manager;
pub mod mbox;
pub mod types;

pub use manager::MigrationManager;
pub use types::*;
