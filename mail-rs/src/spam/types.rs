//! Spam types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Spam scoring result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamResult {
    /// Total spam score
    pub score: f64,
    /// Is this message spam (score > threshold)
    pub is_spam: bool,
    /// Rules that matched
    pub rules_matched: Vec<SpamRuleMatch>,
    /// Recommended action
    pub action: SpamAction,
}

/// A matched spam rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamRuleMatch {
    /// Rule name
    pub rule_name: String,
    /// Score contribution
    pub score: f64,
    /// Description
    pub description: String,
}

/// Action to take on spam
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SpamAction {
    /// Deliver normally
    Deliver,
    /// Add spam headers but deliver
    AddHeaders,
    /// Move to spam folder
    Quarantine,
    /// Reject the message
    Reject,
}

/// Spam configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamConfig {
    /// Score threshold for spam classification
    pub spam_threshold: f64,
    /// Score threshold for certain ham
    pub ham_threshold: f64,
    /// Enable quarantine
    pub quarantine_enabled: bool,
    /// Enable learning
    pub learning_enabled: bool,
    /// Quarantine folder name
    pub quarantine_folder: String,
}

impl Default for SpamConfig {
    fn default() -> Self {
        Self {
            spam_threshold: 5.0,
            ham_threshold: -0.5,
            quarantine_enabled: true,
            learning_enabled: true,
            quarantine_folder: "Spam".to_string(),
        }
    }
}

/// Spam rule definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamRule {
    /// Unique ID
    pub id: String,
    /// Rule name
    pub name: String,
    /// Description
    pub description: String,
    /// Rule type
    pub rule_type: SpamRuleType,
    /// Pattern to match
    pub pattern: String,
    /// Score if matched
    pub score: f64,
    /// Is enabled
    pub is_enabled: bool,
}

/// Types of spam rules
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpamRuleType {
    /// Header check
    Header,
    /// Body content check
    Body,
    /// DNS blacklist check
    Dns,
    /// Bayesian score
    Bayesian,
    /// Custom regex
    Regex,
}

/// Spam log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamLog {
    /// Unique ID
    pub id: String,
    /// Message ID
    pub message_id: String,
    /// Recipient email
    pub recipient_email: String,
    /// Total score
    pub total_score: f64,
    /// Rules matched (JSON)
    pub rules_matched: String,
    /// Action taken
    pub action_taken: SpamAction,
    /// Timestamp
    pub created_at: DateTime<Utc>,
}
