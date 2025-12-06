use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Greylist entry tracking sender attempts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GreylistEntry {
    /// Sender email address
    pub sender: String,
    /// Recipient email address
    pub recipient: String,
    /// Client IP address
    pub client_ip: String,
    /// First time this triple was seen
    pub first_seen: DateTime<Utc>,
    /// Last time this triple was seen
    pub last_seen: DateTime<Utc>,
    /// Number of delivery attempts
    pub attempts: u32,
    /// Current status
    pub status: GreylistStatus,
}

impl GreylistEntry {
    /// Create new greylist entry
    pub fn new(sender: String, recipient: String, client_ip: String) -> Self {
        let now = Utc::now();
        GreylistEntry {
            sender,
            recipient,
            client_ip,
            first_seen: now,
            last_seen: now,
            attempts: 1,
            status: GreylistStatus::Greylisted,
        }
    }

    /// Generate unique key for this entry
    pub fn key(&self) -> String {
        format!("{}:{}:{}", self.sender, self.recipient, self.client_ip)
    }

    /// Check if entry should be auto-whitelisted
    pub fn should_whitelist(&self, delay_secs: i64) -> bool {
        let elapsed = Utc::now()
            .signed_duration_since(self.first_seen)
            .num_seconds();
        elapsed >= delay_secs && self.attempts >= 2
    }
}

/// Greylist status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GreylistStatus {
    Greylisted,  // Temporarily delayed
    Whitelisted, // Permanently allowed
    Blacklisted, // Permanently blocked
}

/// Whitelist/Blacklist entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListEntry {
    /// Pattern to match (email or domain)
    pub pattern: String,
    /// When this entry was added
    pub added_at: DateTime<Utc>,
    /// Optional reason/note
    pub reason: Option<String>,
}

impl ListEntry {
    pub fn new(pattern: String) -> Self {
        ListEntry {
            pattern,
            added_at: Utc::now(),
            reason: None,
        }
    }

    pub fn with_reason(pattern: String, reason: String) -> Self {
        ListEntry {
            pattern,
            added_at: Utc::now(),
            reason: Some(reason),
        }
    }

    /// Check if this entry matches an email address
    pub fn matches(&self, email: &str) -> bool {
        if self.pattern == email {
            return true;
        }

        // Domain match (e.g., "@example.com" matches "user@example.com")
        if self.pattern.starts_with('@') {
            if let Some(domain) = email.split('@').nth(1) {
                return format!("@{}", domain) == self.pattern;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greylist_entry_new() {
        let entry = GreylistEntry::new(
            "sender@example.com".to_string(),
            "recipient@test.com".to_string(),
            "192.0.2.1".to_string(),
        );

        assert_eq!(entry.sender, "sender@example.com");
        assert_eq!(entry.recipient, "recipient@test.com");
        assert_eq!(entry.client_ip, "192.0.2.1");
        assert_eq!(entry.attempts, 1);
        assert_eq!(entry.status, GreylistStatus::Greylisted);
    }

    #[test]
    fn test_greylist_entry_key() {
        let entry = GreylistEntry::new(
            "sender@example.com".to_string(),
            "recipient@test.com".to_string(),
            "192.0.2.1".to_string(),
        );

        assert_eq!(
            entry.key(),
            "sender@example.com:recipient@test.com:192.0.2.1"
        );
    }

    #[test]
    fn test_should_whitelist() {
        let mut entry = GreylistEntry::new(
            "sender@example.com".to_string(),
            "recipient@test.com".to_string(),
            "192.0.2.1".to_string(),
        );

        // Just created, should not whitelist yet
        assert!(!entry.should_whitelist(300)); // 5 minutes

        // Simulate time passing and retry
        entry.first_seen = Utc::now() - chrono::Duration::seconds(400);
        entry.attempts = 2;

        assert!(entry.should_whitelist(300));
    }

    #[test]
    fn test_greylist_status_equality() {
        assert_eq!(GreylistStatus::Greylisted, GreylistStatus::Greylisted);
        assert_eq!(GreylistStatus::Whitelisted, GreylistStatus::Whitelisted);
        assert_eq!(GreylistStatus::Blacklisted, GreylistStatus::Blacklisted);
        assert_ne!(GreylistStatus::Greylisted, GreylistStatus::Whitelisted);
    }

    #[test]
    fn test_list_entry_new() {
        let entry = ListEntry::new("user@example.com".to_string());
        assert_eq!(entry.pattern, "user@example.com");
        assert!(entry.reason.is_none());
    }

    #[test]
    fn test_list_entry_with_reason() {
        let entry =
            ListEntry::with_reason("spam@example.com".to_string(), "Known spammer".to_string());
        assert_eq!(entry.pattern, "spam@example.com");
        assert_eq!(entry.reason, Some("Known spammer".to_string()));
    }

    #[test]
    fn test_list_entry_matches_exact() {
        let entry = ListEntry::new("user@example.com".to_string());
        assert!(entry.matches("user@example.com"));
        assert!(!entry.matches("other@example.com"));
    }

    #[test]
    fn test_list_entry_matches_domain() {
        let entry = ListEntry::new("@example.com".to_string());
        assert!(entry.matches("anyone@example.com"));
        assert!(entry.matches("user@example.com"));
        assert!(!entry.matches("user@other.com"));
    }

    #[test]
    fn test_list_entry_matches_no_at_symbol() {
        let entry = ListEntry::new("user".to_string());
        // No @ symbol, so won't match email addresses or plain "user"
        assert!(!entry.matches("user@example.com"));
        // But will match exact string "user" (though not a typical use case)
        assert!(entry.matches("user"));
    }
}
