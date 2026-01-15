//! CalDAV/CardDAV module
//!
//! Provides calendar and contacts synchronization via WebDAV extensions.

pub mod calendar;
pub mod contacts;
pub mod manager;
pub mod types;

pub use manager::CalDavManager;
pub use types::*;
