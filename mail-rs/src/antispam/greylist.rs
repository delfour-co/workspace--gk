use anyhow::Result;
use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{GreylistEntry, GreylistStatus, ListEntry};

/// Greylist manager configuration
#[derive(Debug, Clone)]
pub struct GreylistConfig {
    /// Delay in seconds before accepting retry (default: 300 = 5 minutes)
    pub delay_seconds: i64,
    /// Auto-whitelist after N days of successful delivery (default: 7)
    pub auto_whitelist_days: i64,
    /// Cleanup entries older than N days (default: 30)
    pub cleanup_days: i64,
}

impl Default for GreylistConfig {
    fn default() -> Self {
        GreylistConfig {
            delay_seconds: 300,         // 5 minutes
            auto_whitelist_days: 7,     // 1 week
            cleanup_days: 30,           // 1 month
        }
    }
}

/// Greylist manager for anti-spam
pub struct GreylistManager {
    config: GreylistConfig,
    entries: Arc<RwLock<HashMap<String, GreylistEntry>>>,
    whitelist: Arc<RwLock<Vec<ListEntry>>>,
    blacklist: Arc<RwLock<Vec<ListEntry>>>,
}

impl GreylistManager {
    /// Create new greylist manager with default config
    pub fn new() -> Self {
        GreylistManager {
            config: GreylistConfig::default(),
            entries: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(Vec::new())),
            blacklist: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Create greylist manager with custom config
    pub fn with_config(config: GreylistConfig) -> Self {
        GreylistManager {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            whitelist: Arc::new(RwLock::new(Vec::new())),
            blacklist: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Check if email should be accepted, greylisted, or rejected
    pub async fn check(
        &self,
        sender: &str,
        recipient: &str,
        client_ip: &str,
    ) -> GreylistStatus {
        // Check blacklist first
        if self.is_blacklisted(sender).await {
            return GreylistStatus::Blacklisted;
        }

        // Check whitelist
        if self.is_whitelisted(sender).await {
            return GreylistStatus::Whitelisted;
        }

        // Check greylist entry
        let key = format!("{}:{}:{}", sender, recipient, client_ip);
        let mut entries = self.entries.write().await;

        if let Some(entry) = entries.get_mut(&key) {
            // Existing entry
            entry.last_seen = Utc::now();
            entry.attempts += 1;

            if entry.should_whitelist(self.config.delay_seconds) {
                entry.status = GreylistStatus::Whitelisted;
            }

            entry.status.clone()
        } else {
            // New sender triple - greylist it
            let entry = GreylistEntry::new(
                sender.to_string(),
                recipient.to_string(),
                client_ip.to_string(),
            );
            let status = entry.status.clone();
            entries.insert(key, entry);
            status
        }
    }

    /// Check if sender is whitelisted
    pub async fn is_whitelisted(&self, sender: &str) -> bool {
        let whitelist = self.whitelist.read().await;
        whitelist.iter().any(|entry| entry.matches(sender))
    }

    /// Check if sender is blacklisted
    pub async fn is_blacklisted(&self, sender: &str) -> bool {
        let blacklist = self.blacklist.read().await;
        blacklist.iter().any(|entry| entry.matches(sender))
    }

    /// Add sender to whitelist
    pub async fn add_to_whitelist(&self, pattern: String, reason: Option<String>) -> Result<()> {
        let mut whitelist = self.whitelist.write().await;
        let entry = if let Some(r) = reason {
            ListEntry::with_reason(pattern, r)
        } else {
            ListEntry::new(pattern)
        };
        whitelist.push(entry);
        Ok(())
    }

    /// Add sender to blacklist
    pub async fn add_to_blacklist(&self, pattern: String, reason: Option<String>) -> Result<()> {
        let mut blacklist = self.blacklist.write().await;
        let entry = if let Some(r) = reason {
            ListEntry::with_reason(pattern, r)
        } else {
            ListEntry::new(pattern)
        };
        blacklist.push(entry);
        Ok(())
    }

    /// Remove from whitelist
    pub async fn remove_from_whitelist(&self, pattern: &str) -> Result<()> {
        let mut whitelist = self.whitelist.write().await;
        whitelist.retain(|entry| entry.pattern != pattern);
        Ok(())
    }

    /// Remove from blacklist
    pub async fn remove_from_blacklist(&self, pattern: &str) -> Result<()> {
        let mut blacklist = self.blacklist.write().await;
        blacklist.retain(|entry| entry.pattern != pattern);
        Ok(())
    }

    /// Get whitelist
    pub async fn get_whitelist(&self) -> Vec<ListEntry> {
        let whitelist = self.whitelist.read().await;
        whitelist.clone()
    }

    /// Get blacklist
    pub async fn get_blacklist(&self) -> Vec<ListEntry> {
        let blacklist = self.blacklist.read().await;
        blacklist.clone()
    }

    /// Cleanup old greylist entries
    pub async fn cleanup_old_entries(&self) -> Result<usize> {
        let mut entries = self.entries.write().await;
        let cutoff = Utc::now() - chrono::Duration::days(self.config.cleanup_days);

        let initial_count = entries.len();
        entries.retain(|_, entry| entry.last_seen > cutoff);
        let removed = initial_count - entries.len();

        Ok(removed)
    }

    /// Get greylist entry count
    pub async fn entry_count(&self) -> usize {
        let entries = self.entries.read().await;
        entries.len()
    }

    /// Get all greylist entries (for admin view)
    pub async fn get_entries(&self) -> Vec<GreylistEntry> {
        let entries = self.entries.read().await;
        entries.values().cloned().collect()
    }
}

impl Default for GreylistManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_greylist_manager_new() {
        let manager = GreylistManager::new();
        assert_eq!(manager.entry_count().await, 0);
    }

    #[tokio::test]
    async fn test_check_new_sender_greylisted() {
        let manager = GreylistManager::new();

        let status = manager
            .check("sender@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        assert_eq!(status, GreylistStatus::Greylisted);
        assert_eq!(manager.entry_count().await, 1);
    }

    #[tokio::test]
    async fn test_check_retry_still_greylisted() {
        let manager = GreylistManager::new();

        manager
            .check("sender@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        // Immediate retry - still greylisted
        let status = manager
            .check("sender@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        assert_eq!(status, GreylistStatus::Greylisted);
    }

    #[tokio::test]
    async fn test_check_whitelisted_sender() {
        let manager = GreylistManager::new();

        manager
            .add_to_whitelist("sender@example.com".to_string(), None)
            .await
            .unwrap();

        let status = manager
            .check("sender@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        assert_eq!(status, GreylistStatus::Whitelisted);
    }

    #[tokio::test]
    async fn test_check_blacklisted_sender() {
        let manager = GreylistManager::new();

        manager
            .add_to_blacklist("spam@example.com".to_string(), Some("Spammer".to_string()))
            .await
            .unwrap();

        let status = manager
            .check("spam@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        assert_eq!(status, GreylistStatus::Blacklisted);
    }

    #[tokio::test]
    async fn test_whitelist_domain() {
        let manager = GreylistManager::new();

        manager
            .add_to_whitelist("@trusted.com".to_string(), None)
            .await
            .unwrap();

        assert!(manager.is_whitelisted("anyone@trusted.com").await);
        assert!(!manager.is_whitelisted("user@other.com").await);
    }

    #[tokio::test]
    async fn test_blacklist_domain() {
        let manager = GreylistManager::new();

        manager
            .add_to_blacklist("@spam.com".to_string(), None)
            .await
            .unwrap();

        assert!(manager.is_blacklisted("anyone@spam.com").await);
        assert!(!manager.is_blacklisted("user@other.com").await);
    }

    #[tokio::test]
    async fn test_remove_from_whitelist() {
        let manager = GreylistManager::new();

        manager
            .add_to_whitelist("user@example.com".to_string(), None)
            .await
            .unwrap();
        assert!(manager.is_whitelisted("user@example.com").await);

        manager
            .remove_from_whitelist("user@example.com")
            .await
            .unwrap();
        assert!(!manager.is_whitelisted("user@example.com").await);
    }

    #[tokio::test]
    async fn test_remove_from_blacklist() {
        let manager = GreylistManager::new();

        manager
            .add_to_blacklist("user@example.com".to_string(), None)
            .await
            .unwrap();
        assert!(manager.is_blacklisted("user@example.com").await);

        manager
            .remove_from_blacklist("user@example.com")
            .await
            .unwrap();
        assert!(!manager.is_blacklisted("user@example.com").await);
    }

    #[tokio::test]
    async fn test_get_whitelist() {
        let manager = GreylistManager::new();

        manager
            .add_to_whitelist("user1@example.com".to_string(), None)
            .await
            .unwrap();
        manager
            .add_to_whitelist("user2@example.com".to_string(), None)
            .await
            .unwrap();

        let whitelist = manager.get_whitelist().await;
        assert_eq!(whitelist.len(), 2);
    }

    #[tokio::test]
    async fn test_get_blacklist() {
        let manager = GreylistManager::new();

        manager
            .add_to_blacklist("spam1@example.com".to_string(), None)
            .await
            .unwrap();
        manager
            .add_to_blacklist("spam2@example.com".to_string(), None)
            .await
            .unwrap();

        let blacklist = manager.get_blacklist().await;
        assert_eq!(blacklist.len(), 2);
    }

    #[tokio::test]
    async fn test_cleanup_old_entries() {
        let mut config = GreylistConfig::default();
        config.cleanup_days = 1; // 1 day
        let manager = GreylistManager::with_config(config);

        // Add entry
        manager
            .check("sender@example.com", "recipient@test.com", "192.0.2.1")
            .await;

        assert_eq!(manager.entry_count().await, 1);

        // Cleanup (entry is recent, should not be removed)
        let removed = manager.cleanup_old_entries().await.unwrap();
        assert_eq!(removed, 0);
        assert_eq!(manager.entry_count().await, 1);
    }

    #[tokio::test]
    async fn test_get_entries() {
        let manager = GreylistManager::new();

        manager
            .check("sender1@example.com", "recipient@test.com", "192.0.2.1")
            .await;
        manager
            .check("sender2@example.com", "recipient@test.com", "192.0.2.2")
            .await;

        let entries = manager.get_entries().await;
        assert_eq!(entries.len(), 2);
    }

    #[tokio::test]
    async fn test_with_config() {
        let config = GreylistConfig {
            delay_seconds: 60,
            auto_whitelist_days: 14,
            cleanup_days: 60,
        };

        let manager = GreylistManager::with_config(config.clone());
        assert_eq!(manager.config.delay_seconds, 60);
        assert_eq!(manager.config.auto_whitelist_days, 14);
    }
}
