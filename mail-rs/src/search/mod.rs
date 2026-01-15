//! Full-text search module
//!
//! Provides email content indexing and search capabilities using Tantivy.

pub mod indexer;
pub mod manager;
pub mod types;

pub use indexer::EmailIndexer;
pub use manager::{SearchConfig, SearchManager};
pub use types::*;
