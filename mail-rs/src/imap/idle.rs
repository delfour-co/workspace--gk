//! IMAP IDLE command implementation with filesystem watching
//!
//! RFC 2177 - IMAP4 IDLE command
//! Allows clients to receive real-time notifications of mailbox changes

use crate::error::MailError;
use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;
use tracing::debug;

/// Watches a maildir for changes and sends notifications
pub struct IdleWatcher {
    /// Filesystem watcher
    _watcher: RecommendedWatcher,
    /// Channel to receive filesystem events
    rx: Receiver<Result<Event, notify::Error>>,
    /// Path being watched
    watch_path: PathBuf,
}

impl IdleWatcher {
    /// Create a new IDLE watcher for a maildir
    ///
    /// # Arguments
    /// * `maildir_path` - Path to the maildir to watch (e.g., /data/maildir/user@example.com)
    pub fn new(maildir_path: &Path) -> Result<Self, MailError> {
        let (tx, rx): (Sender<Result<Event, notify::Error>>, Receiver<Result<Event, notify::Error>>) = channel();

        // Create watcher
        let mut watcher = RecommendedWatcher::new(
            move |result| {
                let _ = tx.send(result);
            },
            Config::default()
                .with_poll_interval(Duration::from_secs(1))
                .with_compare_contents(false),
        )
        .map_err(|e| MailError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;

        // Watch both new/ and cur/ directories for changes
        let new_path = maildir_path.join("new");
        let cur_path = maildir_path.join("cur");

        // Watch new/ directory (for incoming messages)
        if new_path.exists() {
            watcher
                .watch(&new_path, RecursiveMode::NonRecursive)
                .map_err(|e| MailError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            debug!("Watching new/ directory: {:?}", new_path);
        }

        // Watch cur/ directory (for flag changes)
        if cur_path.exists() {
            watcher
                .watch(&cur_path, RecursiveMode::NonRecursive)
                .map_err(|e| MailError::Io(std::io::Error::new(std::io::ErrorKind::Other, e.to_string())))?;
            debug!("Watching cur/ directory: {:?}", cur_path);
        }

        Ok(Self {
            _watcher: watcher,
            rx,
            watch_path: maildir_path.to_path_buf(),
        })
    }

    /// Wait for filesystem changes with a timeout
    ///
    /// Returns true if changes were detected, false if timeout occurred
    pub async fn wait_for_changes(&self, timeout_duration: Duration) -> Result<bool, MailError> {
        let start = std::time::Instant::now();
        let poll_interval = Duration::from_millis(100);

        // Poll for changes until timeout
        while start.elapsed() < timeout_duration {
            // Check for any pending events (non-blocking)
            if self.check_changes_nonblocking() {
                return Ok(true);
            }

            // Sleep briefly before next check
            tokio::time::sleep(poll_interval).await;
        }

        // Timeout - no changes detected
        Ok(false)
    }

    /// Check for changes without blocking (non-blocking check)
    pub fn check_changes_nonblocking(&self) -> bool {
        self.rx.try_iter().next().is_some()
    }

    /// Get the path being watched
    pub fn watch_path(&self) -> &Path {
        &self.watch_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_idle_watcher_creation() {
        let temp_dir = TempDir::new().unwrap();
        let maildir = temp_dir.path().join("test@example.com");
        fs::create_dir_all(maildir.join("new")).unwrap();
        fs::create_dir_all(maildir.join("cur")).unwrap();

        let watcher = IdleWatcher::new(&maildir);
        assert!(watcher.is_ok());
    }

    #[tokio::test]
    async fn test_idle_watcher_detects_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let maildir = temp_dir.path().join("test@example.com");
        fs::create_dir_all(maildir.join("new")).unwrap();
        fs::create_dir_all(maildir.join("cur")).unwrap();

        let watcher = IdleWatcher::new(&maildir).unwrap();

        // Give watcher time to initialize
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Create a new file in new/ directory
        fs::write(maildir.join("new/test.eml"), b"Test email").unwrap();

        // Wait for change detection (with timeout)
        let result = watcher.wait_for_changes(Duration::from_secs(2)).await;
        assert!(result.is_ok());

        // Should detect the change
        assert!(result.unwrap() || watcher.check_changes_nonblocking());
    }

    #[tokio::test]
    async fn test_idle_watcher_timeout() {
        let temp_dir = TempDir::new().unwrap();
        let maildir = temp_dir.path().join("test@example.com");
        fs::create_dir_all(maildir.join("new")).unwrap();
        fs::create_dir_all(maildir.join("cur")).unwrap();

        let watcher = IdleWatcher::new(&maildir).unwrap();

        // Wait without creating any files (should timeout)
        let result = watcher.wait_for_changes(Duration::from_millis(500)).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false); // No changes detected
    }
}
