use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

/// Backup status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BackupStatus {
    /// Backup completed successfully
    Success,
    /// Backup in progress
    InProgress,
    /// Backup failed
    Failed,
    /// Backup not started
    NotStarted,
}

impl std::fmt::Display for BackupStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupStatus::Success => write!(f, "Success"),
            BackupStatus::InProgress => write!(f, "In Progress"),
            BackupStatus::Failed => write!(f, "Failed"),
            BackupStatus::NotStarted => write!(f, "Not Started"),
        }
    }
}

/// Backup metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupMetadata {
    /// Backup filename
    pub filename: String,
    /// When backup was created
    pub created_at: DateTime<Utc>,
    /// Backup size in bytes
    pub size_bytes: u64,
    /// Backup status
    pub status: BackupStatus,
    /// Optional error message
    pub error: Option<String>,
}

impl BackupMetadata {
    pub fn new(filename: String, size_bytes: u64) -> Self {
        BackupMetadata {
            filename,
            created_at: Utc::now(),
            size_bytes,
            status: BackupStatus::Success,
            error: None,
        }
    }

    pub fn failed(filename: String, error: String) -> Self {
        BackupMetadata {
            filename,
            created_at: Utc::now(),
            size_bytes: 0,
            status: BackupStatus::Failed,
            error: Some(error),
        }
    }
}

/// Backup configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupConfig {
    /// Directory to store backups
    pub backup_dir: PathBuf,
    /// Directory to backup (maildir)
    pub maildir_path: PathBuf,
    /// Maximum number of backups to keep
    pub max_backups: usize,
    /// Enable compression
    pub compress: bool,
}

impl Default for BackupConfig {
    fn default() -> Self {
        BackupConfig {
            backup_dir: PathBuf::from("/var/backups/mail-rs"),
            maildir_path: PathBuf::from("/var/mail"),
            max_backups: 7, // Keep 7 days of backups
            compress: true,
        }
    }
}

/// Backup manager
pub struct BackupManager {
    config: BackupConfig,
}

impl BackupManager {
    /// Create new backup manager
    pub fn new(config: BackupConfig) -> Self {
        BackupManager { config }
    }

    /// Create new backup manager with default config
    pub fn with_defaults() -> Self {
        BackupManager {
            config: BackupConfig::default(),
        }
    }

    /// Ensure backup directory exists
    async fn ensure_backup_dir(&self) -> Result<()> {
        if !self.config.backup_dir.exists() {
            fs::create_dir_all(&self.config.backup_dir).await?;
        }
        Ok(())
    }

    /// Generate backup filename
    fn generate_backup_filename(&self) -> String {
        let timestamp = Utc::now().format("%Y%m%d_%H%M%S");
        if self.config.compress {
            format!("mail-backup-{}.tar.gz", timestamp)
        } else {
            format!("mail-backup-{}.tar", timestamp)
        }
    }

    /// Create a new backup
    pub async fn create_backup(&self) -> Result<BackupMetadata> {
        self.ensure_backup_dir().await?;

        let filename = self.generate_backup_filename();
        let backup_path = self.config.backup_dir.join(&filename);

        // Build tar command
        let mut cmd = Command::new("tar");
        cmd.arg("-C")
            .arg(self.config.maildir_path.parent().unwrap_or(Path::new("/")))
            .arg("-cf")
            .arg(&backup_path);

        if self.config.compress {
            cmd.arg("-z");
        }

        cmd.arg(
            self.config
                .maildir_path
                .file_name()
                .ok_or_else(|| anyhow!("Invalid maildir path"))?,
        );

        // Execute backup
        let output = cmd.output().await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr).to_string();
            return Ok(BackupMetadata::failed(filename, error));
        }

        // Get backup file size
        let metadata = fs::metadata(&backup_path).await?;
        let size_bytes = metadata.len();

        Ok(BackupMetadata::new(filename, size_bytes))
    }

    /// List all backups
    pub async fn list_backups(&self) -> Result<Vec<BackupMetadata>> {
        let mut backups = Vec::new();

        if !self.config.backup_dir.exists() {
            return Ok(backups);
        }

        let mut entries = fs::read_dir(&self.config.backup_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy().to_string();

                    if filename_str.starts_with("mail-backup-") {
                        let metadata = fs::metadata(&path).await?;
                        let size_bytes = metadata.len();

                        let created_at = metadata
                            .created()
                            .ok()
                            .and_then(|t| DateTime::<Utc>::from(t).into())
                            .unwrap_or_else(Utc::now);

                        backups.push(BackupMetadata {
                            filename: filename_str,
                            created_at,
                            size_bytes,
                            status: BackupStatus::Success,
                            error: None,
                        });
                    }
                }
            }
        }

        // Sort by creation time, newest first
        backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        Ok(backups)
    }

    /// Restore from backup
    pub async fn restore_backup(&self, filename: &str) -> Result<()> {
        let backup_path = self.config.backup_dir.join(filename);

        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found: {}", filename));
        }

        // Build tar extract command
        let mut cmd = Command::new("tar");
        cmd.arg("-C")
            .arg(self.config.maildir_path.parent().unwrap_or(Path::new("/")))
            .arg("-xf")
            .arg(&backup_path);

        if self.config.compress && filename.ends_with(".tar.gz") {
            cmd.arg("-z");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Restore failed: {}", error));
        }

        Ok(())
    }

    /// Delete a backup
    pub async fn delete_backup(&self, filename: &str) -> Result<()> {
        let backup_path = self.config.backup_dir.join(filename);

        if !backup_path.exists() {
            return Err(anyhow!("Backup file not found: {}", filename));
        }

        fs::remove_file(backup_path).await?;
        Ok(())
    }

    /// Cleanup old backups (keep only max_backups)
    pub async fn cleanup_old_backups(&self) -> Result<usize> {
        let mut backups = self.list_backups().await?;

        if backups.len() <= self.config.max_backups {
            return Ok(0);
        }

        // Remove oldest backups
        let to_remove = backups.len() - self.config.max_backups;
        let mut removed = 0;

        for backup in backups.iter_mut().skip(self.config.max_backups) {
            if let Err(e) = self.delete_backup(&backup.filename).await {
                eprintln!("Failed to delete backup {}: {}", backup.filename, e);
            } else {
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// Get total backup size
    pub async fn get_total_backup_size(&self) -> Result<u64> {
        let backups = self.list_backups().await?;
        Ok(backups.iter().map(|b| b.size_bytes).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_backup_status_display() {
        assert_eq!(BackupStatus::Success.to_string(), "Success");
        assert_eq!(BackupStatus::InProgress.to_string(), "In Progress");
        assert_eq!(BackupStatus::Failed.to_string(), "Failed");
        assert_eq!(BackupStatus::NotStarted.to_string(), "Not Started");
    }

    #[test]
    fn test_backup_metadata_new() {
        let metadata = BackupMetadata::new("backup-123.tar.gz".to_string(), 1024);

        assert_eq!(metadata.filename, "backup-123.tar.gz");
        assert_eq!(metadata.size_bytes, 1024);
        assert_eq!(metadata.status, BackupStatus::Success);
        assert!(metadata.error.is_none());
    }

    #[test]
    fn test_backup_metadata_failed() {
        let metadata = BackupMetadata::failed("backup-123.tar.gz".to_string(), "Error".to_string());

        assert_eq!(metadata.filename, "backup-123.tar.gz");
        assert_eq!(metadata.status, BackupStatus::Failed);
        assert_eq!(metadata.error, Some("Error".to_string()));
        assert_eq!(metadata.size_bytes, 0);
    }

    #[test]
    fn test_backup_config_default() {
        let config = BackupConfig::default();

        assert_eq!(config.max_backups, 7);
        assert!(config.compress);
    }

    #[test]
    fn test_backup_manager_new() {
        let config = BackupConfig::default();
        let manager = BackupManager::new(config.clone());

        assert_eq!(manager.config.max_backups, config.max_backups);
    }

    #[test]
    fn test_backup_manager_with_defaults() {
        let manager = BackupManager::with_defaults();
        assert_eq!(manager.config.max_backups, 7);
    }

    #[test]
    fn test_generate_backup_filename() {
        let manager = BackupManager::with_defaults();
        let filename = manager.generate_backup_filename();

        assert!(filename.starts_with("mail-backup-"));
        assert!(filename.ends_with(".tar.gz"));
    }

    #[test]
    fn test_generate_backup_filename_uncompressed() {
        let mut config = BackupConfig::default();
        config.compress = false;
        let manager = BackupManager::new(config);
        let filename = manager.generate_backup_filename();

        assert!(filename.starts_with("mail-backup-"));
        assert!(filename.ends_with(".tar"));
        assert!(!filename.ends_with(".gz"));
    }

    #[tokio::test]
    async fn test_ensure_backup_dir() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");

        let mut config = BackupConfig::default();
        config.backup_dir = backup_dir.clone();

        let manager = BackupManager::new(config);

        assert!(!backup_dir.exists());
        manager.ensure_backup_dir().await.unwrap();
        assert!(backup_dir.exists());
    }

    #[tokio::test]
    async fn test_list_backups_empty() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).await.unwrap();

        let mut config = BackupConfig::default();
        config.backup_dir = backup_dir;

        let manager = BackupManager::new(config);
        let backups = manager.list_backups().await.unwrap();

        assert_eq!(backups.len(), 0);
    }

    #[tokio::test]
    async fn test_list_backups_with_files() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).await.unwrap();

        // Create fake backup files
        fs::write(backup_dir.join("mail-backup-20240101_120000.tar.gz"), b"test1")
            .await
            .unwrap();
        fs::write(backup_dir.join("mail-backup-20240102_120000.tar.gz"), b"test2")
            .await
            .unwrap();
        fs::write(backup_dir.join("other-file.txt"), b"ignore")
            .await
            .unwrap();

        let mut config = BackupConfig::default();
        config.backup_dir = backup_dir;

        let manager = BackupManager::new(config);
        let backups = manager.list_backups().await.unwrap();

        assert_eq!(backups.len(), 2);
        assert!(backups
            .iter()
            .any(|b| b.filename == "mail-backup-20240101_120000.tar.gz"));
        assert!(backups
            .iter()
            .any(|b| b.filename == "mail-backup-20240102_120000.tar.gz"));
    }

    #[tokio::test]
    async fn test_delete_backup() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).await.unwrap();

        let filename = "mail-backup-20240101_120000.tar.gz";
        let backup_path = backup_dir.join(filename);
        fs::write(&backup_path, b"test").await.unwrap();

        let mut config = BackupConfig::default();
        config.backup_dir = backup_dir;

        let manager = BackupManager::new(config);

        assert!(backup_path.exists());
        manager.delete_backup(filename).await.unwrap();
        assert!(!backup_path.exists());
    }

    #[tokio::test]
    async fn test_get_total_backup_size() {
        let temp_dir = TempDir::new().unwrap();
        let backup_dir = temp_dir.path().join("backups");
        fs::create_dir_all(&backup_dir).await.unwrap();

        fs::write(backup_dir.join("mail-backup-20240101_120000.tar.gz"), b"test1")
            .await
            .unwrap();
        fs::write(backup_dir.join("mail-backup-20240102_120000.tar.gz"), b"test22")
            .await
            .unwrap();

        let mut config = BackupConfig::default();
        config.backup_dir = backup_dir;

        let manager = BackupManager::new(config);
        let total_size = manager.get_total_backup_size().await.unwrap();

        assert_eq!(total_size, 11); // 5 + 6 bytes
    }
}
