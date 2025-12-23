//! Auto-reply / Vacation responder types

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Auto-reply configuration for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplyConfig {
    /// User's email address
    pub email: String,
    /// Whether auto-reply is currently enabled
    pub is_active: bool,
    /// Start date/time (None = active immediately)
    pub start_date: Option<DateTime<Utc>>,
    /// End date/time (None = no end date)
    pub end_date: Option<DateTime<Utc>>,
    /// Subject line for auto-reply
    pub subject: String,
    /// HTML body of auto-reply message
    pub body_html: String,
    /// Plain text body of auto-reply message
    pub body_text: String,
    /// Interval in hours before sending another auto-reply to the same sender (default: 24h)
    pub reply_interval_hours: i32,
    /// Created timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
}

/// Record of an auto-reply sent to a specific sender
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoReplySent {
    /// ID of this record
    pub id: String,
    /// User who has auto-reply enabled
    pub user_email: String,
    /// Email address that received the auto-reply
    pub sent_to: String,
    /// When the auto-reply was sent
    pub sent_at: DateTime<Utc>,
}

/// Request to create or update auto-reply configuration
#[derive(Debug, Deserialize)]
pub struct CreateAutoReplyRequest {
    pub is_active: bool,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub reply_interval_hours: Option<i32>,
}

/// Request to update auto-reply configuration
#[derive(Debug, Deserialize)]
pub struct UpdateAutoReplyRequest {
    pub is_active: Option<bool>,
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub reply_interval_hours: Option<i32>,
}

impl AutoReplyConfig {
    /// Check if auto-reply is currently active (considering dates)
    pub fn is_currently_active(&self) -> bool {
        if !self.is_active {
            return false;
        }

        let now = Utc::now();

        // Check start date
        if let Some(start) = self.start_date {
            if now < start {
                return false;
            }
        }

        // Check end date
        if let Some(end) = self.end_date {
            if now > end {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_currently_active_simple() {
        let config = AutoReplyConfig {
            email: "test@example.com".to_string(),
            is_active: true,
            start_date: None,
            end_date: None,
            subject: "Out of office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: 24,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(config.is_currently_active());
    }

    #[test]
    fn test_is_currently_active_inactive() {
        let mut config = AutoReplyConfig {
            email: "test@example.com".to_string(),
            is_active: false,
            start_date: None,
            end_date: None,
            subject: "Out of office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: 24,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        assert!(!config.is_currently_active());

        config.is_active = true;
        assert!(config.is_currently_active());
    }

    #[test]
    fn test_is_currently_active_with_dates() {
        use chrono::Duration;

        let now = Utc::now();
        let config = AutoReplyConfig {
            email: "test@example.com".to_string(),
            is_active: true,
            start_date: Some(now - Duration::days(1)),
            end_date: Some(now + Duration::days(1)),
            subject: "Out of office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: 24,
            created_at: now,
            updated_at: now,
        };

        assert!(config.is_currently_active());
    }

    #[test]
    fn test_is_currently_active_expired() {
        use chrono::Duration;

        let now = Utc::now();
        let config = AutoReplyConfig {
            email: "test@example.com".to_string(),
            is_active: true,
            start_date: Some(now - Duration::days(10)),
            end_date: Some(now - Duration::days(1)),
            subject: "Out of office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: 24,
            created_at: now,
            updated_at: now,
        };

        assert!(!config.is_currently_active());
    }
}
