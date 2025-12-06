use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::types::{QuotaStatus, UserQuota};

/// Quota manager for enforcing user limits
pub struct QuotaManager {
    quotas: Arc<RwLock<HashMap<String, UserQuota>>>,
    default_quota: UserQuota,
}

impl QuotaManager {
    /// Create new quota manager
    pub fn new() -> Self {
        QuotaManager {
            quotas: Arc::new(RwLock::new(HashMap::new())),
            default_quota: UserQuota::default(),
        }
    }

    /// Create quota manager with custom defaults
    pub fn with_defaults(default_quota: UserQuota) -> Self {
        QuotaManager {
            quotas: Arc::new(RwLock::new(HashMap::new())),
            default_quota,
        }
    }

    /// Get quota for user (creates default if not exists)
    pub async fn get_quota(&self, email: &str) -> UserQuota {
        let quotas = self.quotas.read().await;

        if let Some(quota) = quotas.get(email) {
            quota.clone()
        } else {
            drop(quotas);
            // Create default quota for user
            let mut quota = self.default_quota.clone();
            quota.email = email.to_string();

            let mut quotas = self.quotas.write().await;
            quotas.insert(email.to_string(), quota.clone());
            quota
        }
    }

    /// Set quota for user
    pub async fn set_quota(&self, quota: UserQuota) -> Result<()> {
        let mut quotas = self.quotas.write().await;
        quotas.insert(quota.email.clone(), quota);
        Ok(())
    }

    /// Check if user can store a message of given size
    pub async fn check_storage(&self, email: &str, message_size: u64) -> QuotaStatus {
        let quota = self.get_quota(email).await;

        if quota.is_message_size_exceeded(message_size) {
            return QuotaStatus::MessageSizeExceeded;
        }

        if quota.storage_used + message_size > quota.storage_limit {
            return QuotaStatus::StorageExceeded;
        }

        QuotaStatus::Ok
    }

    /// Check if user can send another message today
    pub async fn check_message_limit(&self, email: &str) -> QuotaStatus {
        let quota = self.get_quota(email).await;

        if quota.is_message_limit_exceeded() {
            return QuotaStatus::MessageLimitExceeded;
        }

        QuotaStatus::Ok
    }

    /// Update storage usage for user
    pub async fn update_storage(&self, email: &str, size_delta: i64) -> Result<()> {
        let mut quotas = self.quotas.write().await;

        if let Some(quota) = quotas.get_mut(email) {
            if size_delta >= 0 {
                quota.storage_used = quota.storage_used.saturating_add(size_delta as u64);
            } else {
                quota.storage_used = quota.storage_used.saturating_sub((-size_delta) as u64);
            }
        } else {
            // Create new quota
            let mut quota = self.default_quota.clone();
            quota.email = email.to_string();
            if size_delta >= 0 {
                quota.storage_used = size_delta as u64;
            }
            quotas.insert(email.to_string(), quota);
        }

        Ok(())
    }

    /// Increment message count for today
    pub async fn increment_message_count(&self, email: &str) -> Result<()> {
        let mut quotas = self.quotas.write().await;

        if let Some(quota) = quotas.get_mut(email) {
            quota.message_count_today = quota.message_count_today.saturating_add(1);
        } else {
            let mut quota = self.default_quota.clone();
            quota.email = email.to_string();
            quota.message_count_today = 1;
            quotas.insert(email.to_string(), quota);
        }

        Ok(())
    }

    /// Reset daily message counts (should be called daily)
    pub async fn reset_daily_counts(&self) -> Result<()> {
        let mut quotas = self.quotas.write().await;

        for quota in quotas.values_mut() {
            quota.message_count_today = 0;
        }

        Ok(())
    }

    /// Get all quotas (for admin view)
    pub async fn list_quotas(&self) -> Vec<UserQuota> {
        let quotas = self.quotas.read().await;
        quotas.values().cloned().collect()
    }

    /// Get quota count
    pub async fn quota_count(&self) -> usize {
        let quotas = self.quotas.read().await;
        quotas.len()
    }
}

impl Default for QuotaManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_quota_manager_new() {
        let manager = QuotaManager::new();
        assert_eq!(manager.quota_count().await, 0);
    }

    #[tokio::test]
    async fn test_get_quota_creates_default() {
        let manager = QuotaManager::new();
        let quota = manager.get_quota("test@example.com").await;

        assert_eq!(quota.email, "test@example.com");
        assert_eq!(quota.storage_used, 0);
        assert_eq!(manager.quota_count().await, 1);
    }

    #[tokio::test]
    async fn test_set_quota() {
        let manager = QuotaManager::new();

        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.storage_limit = 5000;

        manager.set_quota(quota).await.unwrap();

        let retrieved = manager.get_quota("test@example.com").await;
        assert_eq!(retrieved.storage_limit, 5000);
    }

    #[tokio::test]
    async fn test_check_storage_ok() {
        let manager = QuotaManager::new();

        let status = manager.check_storage("test@example.com", 1024).await;
        assert_eq!(status, QuotaStatus::Ok);
    }

    #[tokio::test]
    async fn test_check_storage_size_exceeded() {
        let manager = QuotaManager::new();

        // Default max message size is 25MB
        let status = manager
            .check_storage("test@example.com", 30 * 1024 * 1024)
            .await;
        assert_eq!(status, QuotaStatus::MessageSizeExceeded);
    }

    #[tokio::test]
    async fn test_check_storage_exceeded() {
        let manager = QuotaManager::new();

        // Set low storage limit
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.storage_limit = 1000;
        quota.storage_used = 900;
        manager.set_quota(quota).await.unwrap();

        let status = manager.check_storage("test@example.com", 200).await;
        assert_eq!(status, QuotaStatus::StorageExceeded);
    }

    #[tokio::test]
    async fn test_check_message_limit_ok() {
        let manager = QuotaManager::new();

        let status = manager.check_message_limit("test@example.com").await;
        assert_eq!(status, QuotaStatus::Ok);
    }

    #[tokio::test]
    async fn test_check_message_limit_exceeded() {
        let manager = QuotaManager::new();

        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.message_limit_daily = 10;
        quota.message_count_today = 10;
        manager.set_quota(quota).await.unwrap();

        let status = manager.check_message_limit("test@example.com").await;
        assert_eq!(status, QuotaStatus::MessageLimitExceeded);
    }

    #[tokio::test]
    async fn test_update_storage() {
        let manager = QuotaManager::new();

        manager.update_storage("test@example.com", 500).await.unwrap();
        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.storage_used, 500);

        manager.update_storage("test@example.com", 300).await.unwrap();
        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.storage_used, 800);

        manager
            .update_storage("test@example.com", -200)
            .await
            .unwrap();
        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.storage_used, 600);
    }

    #[tokio::test]
    async fn test_increment_message_count() {
        let manager = QuotaManager::new();

        manager
            .increment_message_count("test@example.com")
            .await
            .unwrap();
        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.message_count_today, 1);

        manager
            .increment_message_count("test@example.com")
            .await
            .unwrap();
        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.message_count_today, 2);
    }

    #[tokio::test]
    async fn test_reset_daily_counts() {
        let manager = QuotaManager::new();

        manager
            .increment_message_count("user1@example.com")
            .await
            .unwrap();
        manager
            .increment_message_count("user2@example.com")
            .await
            .unwrap();

        manager.reset_daily_counts().await.unwrap();

        let quota1 = manager.get_quota("user1@example.com").await;
        let quota2 = manager.get_quota("user2@example.com").await;

        assert_eq!(quota1.message_count_today, 0);
        assert_eq!(quota2.message_count_today, 0);
    }

    #[tokio::test]
    async fn test_list_quotas() {
        let manager = QuotaManager::new();

        manager.get_quota("user1@example.com").await;
        manager.get_quota("user2@example.com").await;

        let quotas = manager.list_quotas().await;
        assert_eq!(quotas.len(), 2);
    }

    #[tokio::test]
    async fn test_with_defaults() {
        let mut default_quota = UserQuota::default();
        default_quota.storage_limit = 5000;
        default_quota.message_limit_daily = 50;

        let manager = QuotaManager::with_defaults(default_quota);

        let quota = manager.get_quota("test@example.com").await;
        assert_eq!(quota.storage_limit, 5000);
        assert_eq!(quota.message_limit_daily, 50);
    }
}
