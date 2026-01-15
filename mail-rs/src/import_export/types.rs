//! Import/Export types
//!
//! Data structures for import/export operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Export format
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ExportFormat {
    /// MBOX format - single file with all messages
    Mbox,
    /// EML format - individual files per message
    Eml,
    /// ZIP archive containing EML files
    EmlZip,
}

impl Default for ExportFormat {
    fn default() -> Self {
        Self::Mbox
    }
}

/// Import format
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum ImportFormat {
    /// MBOX format
    Mbox,
    /// Single EML file
    Eml,
    /// ZIP archive containing EML files
    EmlZip,
}

/// Export request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportRequest {
    /// User email
    pub email: String,
    /// Folders to export (None = all folders)
    pub folders: Option<Vec<String>>,
    /// Export format
    pub format: ExportFormat,
    /// Include subfolders
    pub include_subfolders: bool,
    /// Date range start (None = no limit)
    pub date_from: Option<DateTime<Utc>>,
    /// Date range end (None = no limit)
    pub date_to: Option<DateTime<Utc>>,
}

/// Import request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportRequest {
    /// User email
    pub email: String,
    /// Target folder (None = auto-detect or INBOX)
    pub target_folder: Option<String>,
    /// Import format
    pub format: ImportFormat,
    /// File path or data
    pub source_path: String,
    /// Skip duplicates
    pub skip_duplicates: bool,
    /// Preserve original dates
    pub preserve_dates: bool,
}

/// Operation status
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum OperationStatus {
    /// Operation is pending
    Pending,
    /// Operation is running
    Running,
    /// Operation completed successfully
    Completed,
    /// Operation failed
    Failed,
    /// Operation was cancelled
    Cancelled,
}

/// Export job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    /// Job ID
    pub id: String,
    /// User email
    pub email: String,
    /// Format
    pub format: ExportFormat,
    /// Status
    pub status: OperationStatus,
    /// Progress (0-100)
    pub progress: u8,
    /// Total messages to export
    pub total_messages: u64,
    /// Messages exported so far
    pub exported_messages: u64,
    /// Output file path (when completed)
    pub output_path: Option<String>,
    /// File size in bytes
    pub file_size: Option<u64>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
}

/// Import job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportJob {
    /// Job ID
    pub id: String,
    /// User email
    pub email: String,
    /// Format
    pub format: ImportFormat,
    /// Target folder
    pub target_folder: String,
    /// Status
    pub status: OperationStatus,
    /// Progress (0-100)
    pub progress: u8,
    /// Total messages to import
    pub total_messages: u64,
    /// Messages imported so far
    pub imported_messages: u64,
    /// Messages skipped (duplicates)
    pub skipped_messages: u64,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Created at
    pub created_at: DateTime<Utc>,
    /// Completed at
    pub completed_at: Option<DateTime<Utc>>,
}

/// Import/Export statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportExportStats {
    /// Total exports performed
    pub total_exports: u64,
    /// Total imports performed
    pub total_imports: u64,
    /// Active export jobs
    pub active_exports: u32,
    /// Active import jobs
    pub active_imports: u32,
    /// Total bytes exported
    pub bytes_exported: u64,
    /// Total bytes imported
    pub bytes_imported: u64,
    /// Total messages exported
    pub messages_exported: u64,
    /// Total messages imported
    pub messages_imported: u64,
}

impl Default for ImportExportStats {
    fn default() -> Self {
        Self {
            total_exports: 0,
            total_imports: 0,
            active_exports: 0,
            active_imports: 0,
            bytes_exported: 0,
            bytes_imported: 0,
            messages_exported: 0,
            messages_imported: 0,
        }
    }
}
