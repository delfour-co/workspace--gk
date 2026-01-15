//! Search types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Search query parameters
#[derive(Debug, Clone, Deserialize)]
pub struct SearchQuery {
    /// The search query string
    pub query: String,
    /// Folder to search in (None = all folders)
    pub folder: Option<String>,
    /// Date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Date range end
    pub to_date: Option<DateTime<Utc>>,
    /// Maximum results to return
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Search result entry
#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    /// Email message ID
    pub message_id: String,
    /// Subject line
    pub subject: String,
    /// Sender
    pub from: String,
    /// Date sent
    pub date: DateTime<Utc>,
    /// Folder containing the email
    pub folder: String,
    /// Highlighted snippet from body
    pub snippet: String,
    /// Relevance score
    pub score: f32,
}

/// Search results response
#[derive(Debug, Clone, Serialize)]
pub struct SearchResults {
    /// Matching results
    pub results: Vec<SearchResult>,
    /// Total matches
    pub total: usize,
    /// Query time in milliseconds
    pub query_time_ms: u64,
}

/// Index status
#[derive(Debug, Clone, Serialize)]
pub struct IndexStatus {
    /// Total indexed documents
    pub document_count: u64,
    /// Index size in bytes
    pub index_size_bytes: u64,
    /// Last indexing timestamp
    pub last_indexed_at: Option<DateTime<Utc>>,
    /// Is indexing in progress
    pub is_indexing: bool,
}
