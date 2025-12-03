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
}
