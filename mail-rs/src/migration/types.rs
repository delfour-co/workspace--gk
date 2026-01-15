//! Migration types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Import/Export job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationJob {
    /// Unique ID
    pub id: String,
    /// Owner email
    pub owner_email: String,
    /// Job type
    pub job_type: MigrationJobType,
    /// Job status
    pub status: MigrationStatus,
    /// Total messages to process
    pub total_messages: u64,
    /// Messages processed so far
    pub processed_messages: u64,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Path to file (upload or download)
    pub file_path: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Completion timestamp
    pub completed_at: Option<DateTime<Utc>>,
}

/// Type of migration job
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationJobType {
    /// Import from mbox format
    ImportMbox,
    /// Import from EML files
    ImportEml,
    /// Export to mbox format
    ExportMbox,
    /// Export to EML archive
    ExportEml,
}

impl std::fmt::Display for MigrationJobType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationJobType::ImportMbox => write!(f, "import_mbox"),
            MigrationJobType::ImportEml => write!(f, "import_eml"),
            MigrationJobType::ExportMbox => write!(f, "export_mbox"),
            MigrationJobType::ExportEml => write!(f, "export_eml"),
        }
    }
}

/// Migration job status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MigrationStatus {
    /// Job is queued
    Pending,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
}

impl std::fmt::Display for MigrationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MigrationStatus::Pending => write!(f, "pending"),
            MigrationStatus::Running => write!(f, "running"),
            MigrationStatus::Completed => write!(f, "completed"),
            MigrationStatus::Failed => write!(f, "failed"),
            MigrationStatus::Cancelled => write!(f, "cancelled"),
        }
    }
}

/// Import request
#[derive(Debug, Clone, Deserialize)]
pub struct ImportRequest {
    /// Target folder (None = INBOX)
    pub target_folder: Option<String>,
    /// Whether to mark as read
    pub mark_as_read: bool,
}

/// Export request
#[derive(Debug, Clone, Deserialize)]
pub struct ExportRequest {
    /// Folders to export (None = all)
    pub folders: Option<Vec<String>>,
    /// Date range start
    pub from_date: Option<DateTime<Utc>>,
    /// Date range end
    pub to_date: Option<DateTime<Utc>>,
}
