use serde::{Deserialize, Serialize};

/// User quota configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserQuota {
    /// Email address
    pub email: String,
    /// Maximum storage in bytes
    pub storage_limit: u64,
    /// Current storage used in bytes
    pub storage_used: u64,
    /// Maximum messages per day
    pub message_limit_daily: u32,
    /// Messages sent today
    pub message_count_today: u32,
    /// Maximum size per message in bytes
    pub max_message_size: u64,
}

impl Default for UserQuota {
    fn default() -> Self {
        UserQuota {
            email: String::new(),
            storage_limit: 1024 * 1024 * 1024, // 1GB default
            storage_used: 0,
            message_limit_daily: 100,
            message_count_today: 0,
            max_message_size: 25 * 1024 * 1024, // 25MB default
        }
    }
}

impl UserQuota {
    /// Create new quota for user
    pub fn new(email: String) -> Self {
        UserQuota {
            email,
            ..Default::default()
        }
    }

    /// Check if storage quota is exceeded
    pub fn is_storage_exceeded(&self) -> bool {
        self.storage_used >= self.storage_limit
    }

    /// Check if message limit is exceeded
    pub fn is_message_limit_exceeded(&self) -> bool {
        self.message_count_today >= self.message_limit_daily
    }

    /// Check if message size exceeds limit
    pub fn is_message_size_exceeded(&self, message_size: u64) -> bool {
        message_size > self.max_message_size
    }

    /// Get storage usage percentage
    pub fn storage_usage_percent(&self) -> f64 {
        if self.storage_limit == 0 {
            return 0.0;
        }
        (self.storage_used as f64 / self.storage_limit as f64) * 100.0
    }

    /// Get remaining storage in bytes
    pub fn storage_remaining(&self) -> u64 {
        self.storage_limit.saturating_sub(self.storage_used)
    }

    /// Get remaining daily messages
    pub fn messages_remaining_today(&self) -> u32 {
        self.message_limit_daily.saturating_sub(self.message_count_today)
    }
}

/// Quota check status
#[derive(Debug, Clone, PartialEq)]
pub enum QuotaStatus {
    Ok,
    StorageExceeded,
    MessageLimitExceeded,
    MessageSizeExceeded,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_quota_default() {
        let quota = UserQuota::default();
        assert_eq!(quota.storage_limit, 1024 * 1024 * 1024);
        assert_eq!(quota.storage_used, 0);
        assert_eq!(quota.message_limit_daily, 100);
        assert_eq!(quota.message_count_today, 0);
    }

    #[test]
    fn test_user_quota_new() {
        let quota = UserQuota::new("test@example.com".to_string());
        assert_eq!(quota.email, "test@example.com");
        assert_eq!(quota.storage_limit, 1024 * 1024 * 1024);
    }

    #[test]
    fn test_is_storage_exceeded() {
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.storage_limit = 1000;
        quota.storage_used = 500;
        assert!(!quota.is_storage_exceeded());

        quota.storage_used = 1000;
        assert!(quota.is_storage_exceeded());

        quota.storage_used = 1001;
        assert!(quota.is_storage_exceeded());
    }

    #[test]
    fn test_is_message_limit_exceeded() {
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.message_limit_daily = 10;
        quota.message_count_today = 5;
        assert!(!quota.is_message_limit_exceeded());

        quota.message_count_today = 10;
        assert!(quota.is_message_limit_exceeded());

        quota.message_count_today = 11;
        assert!(quota.is_message_limit_exceeded());
    }

    #[test]
    fn test_is_message_size_exceeded() {
        let quota = UserQuota::new("test@example.com".to_string());
        assert!(!quota.is_message_size_exceeded(1024)); // 1KB
        assert!(!quota.is_message_size_exceeded(25 * 1024 * 1024)); // 25MB (at limit)
        assert!(quota.is_message_size_exceeded(26 * 1024 * 1024)); // 26MB
    }

    #[test]
    fn test_storage_usage_percent() {
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.storage_limit = 1000;
        quota.storage_used = 250;
        assert_eq!(quota.storage_usage_percent(), 25.0);

        quota.storage_used = 500;
        assert_eq!(quota.storage_usage_percent(), 50.0);

        quota.storage_used = 1000;
        assert_eq!(quota.storage_usage_percent(), 100.0);
    }

    #[test]
    fn test_storage_remaining() {
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.storage_limit = 1000;
        quota.storage_used = 400;
        assert_eq!(quota.storage_remaining(), 600);

        quota.storage_used = 1000;
        assert_eq!(quota.storage_remaining(), 0);

        quota.storage_used = 1100;
        assert_eq!(quota.storage_remaining(), 0); // Saturating
    }

    #[test]
    fn test_messages_remaining_today() {
        let mut quota = UserQuota::new("test@example.com".to_string());
        quota.message_limit_daily = 100;
        quota.message_count_today = 30;
        assert_eq!(quota.messages_remaining_today(), 70);

        quota.message_count_today = 100;
        assert_eq!(quota.messages_remaining_today(), 0);

        quota.message_count_today = 110;
        assert_eq!(quota.messages_remaining_today(), 0); // Saturating
    }

    #[test]
    fn test_quota_status_equality() {
        assert_eq!(QuotaStatus::Ok, QuotaStatus::Ok);
        assert_eq!(QuotaStatus::StorageExceeded, QuotaStatus::StorageExceeded);
        assert_ne!(QuotaStatus::Ok, QuotaStatus::StorageExceeded);
    }
}
