//! MFA types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// MFA configuration for a user
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaConfig {
    /// User email
    pub email: String,
    /// Encrypted TOTP secret (base32 encoded, then encrypted)
    pub secret_encrypted: String,
    /// Whether MFA is enabled for this user
    pub is_enabled: bool,
    /// Last time a TOTP code was successfully used
    pub last_used_at: Option<DateTime<Utc>>,
    /// When MFA was enabled
    pub enabled_at: Option<DateTime<Utc>>,
}

impl MfaConfig {
    pub fn new(email: String) -> Self {
        Self {
            email,
            secret_encrypted: String::new(),
            is_enabled: false,
            last_used_at: None,
            enabled_at: None,
        }
    }
}

/// Backup code for account recovery
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupCode {
    /// Unique ID
    pub id: String,
    /// User email
    pub email: String,
    /// Argon2 hash of the backup code
    pub code_hash: String,
    /// Whether this code has been used
    pub is_used: bool,
    /// When the code was used (if used)
    pub used_at: Option<DateTime<Utc>>,
    /// When the code was created
    pub created_at: DateTime<Utc>,
}

/// MFA setup response (returned when user initiates MFA setup)
#[derive(Debug, Clone, Serialize)]
pub struct MfaSetupResponse {
    /// The secret in base32 format (for manual entry)
    pub secret: String,
    /// QR code as data URI (base64 PNG)
    pub qr_code: String,
    /// The provisioning URI for authenticator apps
    pub provisioning_uri: String,
}

/// MFA status response
#[derive(Debug, Clone, Serialize)]
pub struct MfaStatusResponse {
    /// Whether MFA is enabled
    pub is_enabled: bool,
    /// Number of remaining backup codes
    pub backup_codes_remaining: usize,
    /// When MFA was enabled
    pub enabled_at: Option<DateTime<Utc>>,
    /// Last successful verification
    pub last_used_at: Option<DateTime<Utc>>,
}

/// MFA verification request
#[derive(Debug, Clone, Deserialize)]
pub struct MfaVerifyRequest {
    /// The TOTP code entered by user
    pub code: String,
}

/// MFA audit log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MfaAuditLog {
    /// Unique ID
    pub id: String,
    /// User email
    pub email: String,
    /// Event type
    pub event_type: MfaEventType,
    /// Client IP address
    pub ip_address: Option<String>,
    /// User agent string
    pub user_agent: Option<String>,
    /// When the event occurred
    pub created_at: DateTime<Utc>,
}

/// Types of MFA events for audit logging
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MfaEventType {
    /// MFA setup initiated
    SetupStarted,
    /// MFA setup completed (enabled)
    SetupCompleted,
    /// MFA verification successful
    VerifySuccess,
    /// MFA verification failed
    VerifyFailed,
    /// MFA disabled
    Disabled,
    /// Backup code used
    BackupCodeUsed,
    /// Backup codes regenerated
    BackupCodesRegenerated,
}

impl std::fmt::Display for MfaEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MfaEventType::SetupStarted => write!(f, "setup_started"),
            MfaEventType::SetupCompleted => write!(f, "setup_completed"),
            MfaEventType::VerifySuccess => write!(f, "verify_success"),
            MfaEventType::VerifyFailed => write!(f, "verify_failed"),
            MfaEventType::Disabled => write!(f, "disabled"),
            MfaEventType::BackupCodeUsed => write!(f, "backup_code_used"),
            MfaEventType::BackupCodesRegenerated => write!(f, "backup_codes_regenerated"),
        }
    }
}

/// Result of MFA verification
#[derive(Debug, Clone, PartialEq)]
pub enum MfaVerifyResult {
    /// Code is valid
    Valid,
    /// Code is invalid
    Invalid,
    /// MFA is not enabled for this user
    NotEnabled,
    /// Code was already used (replay attack protection)
    AlreadyUsed,
    /// Too many failed attempts
    RateLimited,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mfa_config_new() {
        let config = MfaConfig::new("test@example.com".to_string());
        assert_eq!(config.email, "test@example.com");
        assert!(!config.is_enabled);
        assert!(config.secret_encrypted.is_empty());
    }

    #[test]
    fn test_mfa_event_type_display() {
        assert_eq!(MfaEventType::SetupStarted.to_string(), "setup_started");
        assert_eq!(MfaEventType::VerifySuccess.to_string(), "verify_success");
        assert_eq!(MfaEventType::BackupCodeUsed.to_string(), "backup_code_used");
    }
}
