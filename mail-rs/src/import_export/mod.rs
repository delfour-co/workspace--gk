//! Import/Export module
//!
//! Provides mailbox import and export functionality supporting MBOX and EML formats.

pub mod manager;
pub mod mbox;
pub mod types;

pub use manager::ImportExportManager;
pub use types::*;
