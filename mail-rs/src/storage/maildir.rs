use crate::error::{MailError, Result};
use std::path::PathBuf;
use tokio::fs;
use tracing::info;

pub struct MaildirStorage {
    base_path: PathBuf,
}

impl MaildirStorage {
    pub fn new(base_path: String) -> Self {
        Self {
            base_path: PathBuf::from(base_path),
        }
    }

    pub async fn store(&self, recipient: &str, data: &[u8]) -> Result<String> {
        // Create mailbox directory structure if it doesn't exist
        let mailbox_path = self.base_path.join(recipient);
        self.ensure_maildir_structure(&mailbox_path).await?;

        // Generate unique filename
        let filename = self.generate_filename();
        let tmp_path = mailbox_path.join("tmp").join(&filename);
        let new_path = mailbox_path.join("new").join(&filename);

        // Write to tmp directory first
        fs::write(&tmp_path, data).await?;

        // Move to new directory (atomic operation)
        fs::rename(&tmp_path, &new_path).await?;

        info!(
            "Stored email for {} as {}",
            recipient,
            new_path.display()
        );

        Ok(filename)
    }

    async fn ensure_maildir_structure(&self, mailbox_path: &PathBuf) -> Result<()> {
        for subdir in &["tmp", "new", "cur"] {
            let dir = mailbox_path.join(subdir);
            if !dir.exists() {
                fs::create_dir_all(&dir).await.map_err(|e| {
                    MailError::Storage(format!("Failed to create directory {:?}: {}", dir, e))
                })?;
            }
        }
        Ok(())
    }

    fn generate_filename(&self) -> String {
        // Maildir filename format: timestamp.pid.hostname
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let pid = std::process::id();
        let hostname = gethostname::gethostname()
            .to_string_lossy()
            .to_string();

        format!("{}.{}.{}", timestamp, pid, hostname)
    }
}
