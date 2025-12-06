use serde::{Deserialize, Serialize};

/// Authentication status for SPF/DKIM/DMARC
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuthenticationStatus {
    /// Authentication passed
    Pass,
    /// Authentication failed
    Fail,
    /// Temporary error (DNS timeout, etc.)
    TempError,
    /// Permanent error (no SPF record, etc.)
    PermError,
    /// Neutral (policy allows but doesn't endorse)
    Neutral,
    /// Softfail (policy suggests reject but not enforced)
    SoftFail,
    /// No authentication attempted
    None,
}

impl std::fmt::Display for AuthenticationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthenticationStatus::Pass => write!(f, "pass"),
            AuthenticationStatus::Fail => write!(f, "fail"),
            AuthenticationStatus::TempError => write!(f, "temperror"),
            AuthenticationStatus::PermError => write!(f, "permerror"),
            AuthenticationStatus::Neutral => write!(f, "neutral"),
            AuthenticationStatus::SoftFail => write!(f, "softfail"),
            AuthenticationStatus::None => write!(f, "none"),
        }
    }
}

/// Combined authentication results for an email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticationResults {
    /// SPF authentication result
    pub spf: SpfAuthResult,
    /// DKIM authentication result
    pub dkim: DkimAuthResult,
    /// Overall authentication summary
    pub summary: String,
}

/// SPF authentication result details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpfAuthResult {
    /// Authentication status
    pub status: AuthenticationStatus,
    /// Client IP that sent the email
    pub client_ip: String,
    /// Envelope FROM domain
    pub envelope_from: String,
    /// Additional explanation
    pub reason: Option<String>,
}

/// DKIM authentication result details
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DkimAuthResult {
    /// Authentication status
    pub status: AuthenticationStatus,
    /// DKIM signature domain
    pub domain: String,
    /// DKIM selector
    pub selector: String,
    /// Additional explanation
    pub reason: Option<String>,
}

impl AuthenticationResults {
    /// Create new empty authentication results
    pub fn new() -> Self {
        Self {
            spf: SpfAuthResult {
                status: AuthenticationStatus::None,
                client_ip: String::new(),
                envelope_from: String::new(),
                reason: None,
            },
            dkim: DkimAuthResult {
                status: AuthenticationStatus::None,
                domain: String::new(),
                selector: String::new(),
                reason: None,
            },
            summary: String::new(),
        }
    }

    /// Generate Authentication-Results header value
    pub fn to_header(&self, authserv_id: &str) -> String {
        let mut parts = vec![authserv_id.to_string()];

        // Add SPF result
        if self.spf.status != AuthenticationStatus::None {
            parts.push(format!(
                "spf={} smtp.mailfrom={}",
                self.spf.status, self.spf.envelope_from
            ));
        }

        // Add DKIM result
        if self.dkim.status != AuthenticationStatus::None {
            parts.push(format!(
                "dkim={} header.d={}",
                self.dkim.status, self.dkim.domain
            ));
        }

        parts.join("; ")
    }
}

impl Default for AuthenticationResults {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authentication_status_display() {
        assert_eq!(AuthenticationStatus::Pass.to_string(), "pass");
        assert_eq!(AuthenticationStatus::Fail.to_string(), "fail");
        assert_eq!(AuthenticationStatus::TempError.to_string(), "temperror");
        assert_eq!(AuthenticationStatus::PermError.to_string(), "permerror");
        assert_eq!(AuthenticationStatus::Neutral.to_string(), "neutral");
        assert_eq!(AuthenticationStatus::SoftFail.to_string(), "softfail");
        assert_eq!(AuthenticationStatus::None.to_string(), "none");
    }

    #[test]
    fn test_authentication_results_header() {
        let mut results = AuthenticationResults::new();
        results.spf.status = AuthenticationStatus::Pass;
        results.spf.envelope_from = "example.com".to_string();
        results.dkim.status = AuthenticationStatus::Pass;
        results.dkim.domain = "example.com".to_string();

        let header = results.to_header("mail.example.com");
        assert!(header.contains("spf=pass"));
        assert!(header.contains("dkim=pass"));
        assert!(header.contains("mail.example.com"));
    }

    #[test]
    fn test_authentication_results_header_spf_only() {
        let mut results = AuthenticationResults::new();
        results.spf.status = AuthenticationStatus::Pass;
        results.spf.envelope_from = "sender@example.com".to_string();
        results.spf.client_ip = "192.0.2.1".to_string();

        let header = results.to_header("mail.test.com");
        assert!(header.contains("mail.test.com"));
        assert!(header.contains("spf=pass"));
        assert!(header.contains("sender@example.com"));
        // DKIM should not be in header when status is None
        assert!(!header.contains("dkim="));
    }

    #[test]
    fn test_authentication_results_header_dkim_only() {
        let mut results = AuthenticationResults::new();
        results.dkim.status = AuthenticationStatus::Pass;
        results.dkim.domain = "example.com".to_string();
        results.dkim.selector = "default".to_string();

        let header = results.to_header("mail.test.com");
        assert!(header.contains("mail.test.com"));
        assert!(header.contains("dkim=pass"));
        assert!(header.contains("example.com"));
        // SPF should not be in header when status is None
        assert!(!header.contains("spf="));
    }

    #[test]
    fn test_authentication_results_header_failures() {
        let mut results = AuthenticationResults::new();
        results.spf.status = AuthenticationStatus::Fail;
        results.spf.envelope_from = "spammer@evil.com".to_string();
        results.dkim.status = AuthenticationStatus::Fail;
        results.dkim.domain = "evil.com".to_string();

        let header = results.to_header("mail.example.com");
        assert!(header.contains("spf=fail"));
        assert!(header.contains("dkim=fail"));
        assert!(header.contains("spammer@evil.com"));
        assert!(header.contains("evil.com"));
    }

    #[test]
    fn test_authentication_results_header_softfail() {
        let mut results = AuthenticationResults::new();
        results.spf.status = AuthenticationStatus::SoftFail;
        results.spf.envelope_from = "test@example.com".to_string();

        let header = results.to_header("mail.example.com");
        assert!(header.contains("spf=softfail"));
    }

    #[test]
    fn test_authentication_results_header_temperror() {
        let mut results = AuthenticationResults::new();
        results.spf.status = AuthenticationStatus::TempError;
        results.spf.envelope_from = "test@example.com".to_string();
        results.dkim.status = AuthenticationStatus::TempError;
        results.dkim.domain = "example.com".to_string();

        let header = results.to_header("mail.example.com");
        assert!(header.contains("spf=temperror"));
        assert!(header.contains("dkim=temperror"));
    }

    #[test]
    fn test_authentication_results_default() {
        let results = AuthenticationResults::default();
        assert_eq!(results.spf.status, AuthenticationStatus::None);
        assert_eq!(results.dkim.status, AuthenticationStatus::None);
        assert!(results.spf.client_ip.is_empty());
        assert!(results.dkim.domain.is_empty());
    }

    #[test]
    fn test_spf_auth_result_with_reason() {
        let result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@example.com".to_string(),
            reason: Some("IP authorized in SPF record".to_string()),
        };

        assert_eq!(result.status, AuthenticationStatus::Pass);
        assert_eq!(result.client_ip, "192.0.2.1");
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_dkim_auth_result_with_reason() {
        let result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("DKIM signature valid".to_string()),
        };

        assert_eq!(result.status, AuthenticationStatus::Pass);
        assert_eq!(result.domain, "example.com");
        assert_eq!(result.selector, "default");
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_authentication_status_equality() {
        assert_eq!(AuthenticationStatus::Pass, AuthenticationStatus::Pass);
        assert_ne!(AuthenticationStatus::Pass, AuthenticationStatus::Fail);
        assert_ne!(AuthenticationStatus::None, AuthenticationStatus::Neutral);
    }

    #[test]
    fn test_authentication_status_clone() {
        let status = AuthenticationStatus::Pass;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn test_serialization() {
        let results = AuthenticationResults {
            spf: SpfAuthResult {
                status: AuthenticationStatus::Pass,
                client_ip: "192.0.2.1".to_string(),
                envelope_from: "test@example.com".to_string(),
                reason: Some("Test".to_string()),
            },
            dkim: DkimAuthResult {
                status: AuthenticationStatus::Pass,
                domain: "example.com".to_string(),
                selector: "default".to_string(),
                reason: Some("Test".to_string()),
            },
            summary: "All checks passed".to_string(),
        };

        // Test that serialization works (will fail if Serialize is not properly implemented)
        let json = serde_json::to_string(&results);
        assert!(json.is_ok());

        // Test deserialization
        if let Ok(json_str) = json {
            let deserialized: Result<AuthenticationResults, _> = serde_json::from_str(&json_str);
            assert!(deserialized.is_ok());
        }
    }
}
