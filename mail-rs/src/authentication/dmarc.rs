use anyhow::{anyhow, Result};
use mail_auth::{DmarcResult as MailAuthDmarcResult, Resolver};
use std::sync::Arc;

use super::types::{AuthenticationStatus, DkimAuthResult, SpfAuthResult};

/// DMARC policy extracted from DNS
#[derive(Debug, Clone, PartialEq)]
pub enum DmarcPolicy {
    None,       // p=none : monitoring only
    Quarantine, // p=quarantine : mark as spam
    Reject,     // p=reject : block email
}

impl Default for DmarcPolicy {
    fn default() -> Self {
        DmarcPolicy::None
    }
}

impl std::fmt::Display for DmarcPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DmarcPolicy::None => write!(f, "none"),
            DmarcPolicy::Quarantine => write!(f, "quarantine"),
            DmarcPolicy::Reject => write!(f, "reject"),
        }
    }
}

/// DMARC alignment mode
#[derive(Debug, Clone, PartialEq)]
pub enum DmarcAlignment {
    Relaxed, // Subdomain alignment allowed
    Strict,  // Exact domain match required
}

impl Default for DmarcAlignment {
    fn default() -> Self {
        DmarcAlignment::Relaxed
    }
}

/// DMARC validation result
#[derive(Debug, Clone)]
pub struct DmarcResult {
    pub policy: DmarcPolicy,
    pub spf_aligned: bool,
    pub dkim_aligned: bool,
    pub pass: bool,
    pub reason: Option<String>,
}

impl Default for DmarcResult {
    fn default() -> Self {
        DmarcResult {
            policy: DmarcPolicy::None,
            spf_aligned: false,
            dkim_aligned: false,
            pass: false,
            reason: Some("No DMARC policy found".to_string()),
        }
    }
}

/// DMARC validator
pub struct DmarcValidator {
    resolver: Arc<Resolver>,
}

impl DmarcValidator {
    /// Create a new DMARC validator
    pub fn new() -> Self {
        let resolver = Resolver::new_system_conf().unwrap_or_else(|_| {
            Resolver::new_cloudflare_tls().expect("Failed to create DNS resolver")
        });

        DmarcValidator {
            resolver: Arc::new(resolver),
        }
    }

    /// Validate email against DMARC policy
    pub async fn validate(
        &self,
        from_domain: &str,
        spf_result: &SpfAuthResult,
        dkim_result: &DkimAuthResult,
    ) -> Result<DmarcResult> {
        // For now, we'll implement DMARC alignment checking without DNS lookups
        // In a production system, we would query _dmarc.{domain} TXT record
        // and extract p=none|quarantine|reject policy

        // Default policy is None (monitoring only)
        let policy = DmarcPolicy::None;

        // Check SPF alignment
        let spf_aligned = self.check_spf_alignment(from_domain, spf_result);

        // Check DKIM alignment
        let dkim_aligned = self.check_dkim_alignment(from_domain, dkim_result);

        // Determine if DMARC passes
        // DMARC passes if either SPF or DKIM is aligned and passes
        let spf_pass = spf_result.status == AuthenticationStatus::Pass && spf_aligned;
        let dkim_pass = dkim_result.status == AuthenticationStatus::Pass && dkim_aligned;
        let pass = spf_pass || dkim_pass;

        let reason = if pass {
            Some("DMARC validation passed".to_string())
        } else if !spf_aligned && !dkim_aligned {
            Some("Neither SPF nor DKIM aligned".to_string())
        } else if !spf_pass && !dkim_pass {
            Some("Neither SPF nor DKIM passed authentication".to_string())
        } else {
            Some("DMARC validation failed".to_string())
        };

        Ok(DmarcResult {
            policy,
            spf_aligned,
            dkim_aligned,
            pass,
            reason,
        })
    }

    /// Check if SPF domain aligns with From header domain
    fn check_spf_alignment(&self, from_domain: &str, spf_result: &SpfAuthResult) -> bool {
        // Extract domain from envelope-from
        let envelope_domain = spf_result
            .envelope_from
            .split('@')
            .nth(1)
            .unwrap_or(&spf_result.envelope_from);

        // Relaxed alignment: domains match or envelope is subdomain of from
        self.domains_align(from_domain, envelope_domain, &DmarcAlignment::Relaxed)
    }

    /// Check if DKIM domain aligns with From header domain
    fn check_dkim_alignment(&self, from_domain: &str, dkim_result: &DkimAuthResult) -> bool {
        // Relaxed alignment: domains match or DKIM domain is subdomain of from
        self.domains_align(from_domain, &dkim_result.domain, &DmarcAlignment::Relaxed)
    }

    /// Check if two domains align according to DMARC alignment mode
    fn domains_align(&self, from_domain: &str, auth_domain: &str, mode: &DmarcAlignment) -> bool {
        match mode {
            DmarcAlignment::Strict => {
                // Exact match required
                from_domain.eq_ignore_ascii_case(auth_domain)
            }
            DmarcAlignment::Relaxed => {
                // Organizational domain match
                // For relaxed, "mail.example.com" aligns with "example.com"
                if from_domain.eq_ignore_ascii_case(auth_domain) {
                    return true;
                }

                // Check if one is subdomain of the other
                let from_lower = from_domain.to_lowercase();
                let auth_lower = auth_domain.to_lowercase();

                from_lower.ends_with(&format!(".{}", auth_lower))
                    || auth_lower.ends_with(&format!(".{}", from_lower))
            }
        }
    }

    /// Determine if message should be rejected based on DMARC policy
    pub fn should_reject(&self, result: &DmarcResult) -> bool {
        !result.pass && result.policy == DmarcPolicy::Reject
    }

    /// Determine if message should be quarantined based on DMARC policy
    pub fn should_quarantine(&self, result: &DmarcResult) -> bool {
        !result.pass && result.policy == DmarcPolicy::Quarantine
    }
}

impl Default for DmarcValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dmarc_policy_display() {
        assert_eq!(DmarcPolicy::None.to_string(), "none");
        assert_eq!(DmarcPolicy::Quarantine.to_string(), "quarantine");
        assert_eq!(DmarcPolicy::Reject.to_string(), "reject");
    }

    #[test]
    fn test_dmarc_policy_default() {
        let policy = DmarcPolicy::default();
        assert_eq!(policy, DmarcPolicy::None);
    }

    #[test]
    fn test_dmarc_alignment_default() {
        let alignment = DmarcAlignment::default();
        assert_eq!(alignment, DmarcAlignment::Relaxed);
    }

    #[test]
    fn test_dmarc_result_default() {
        let result = DmarcResult::default();
        assert_eq!(result.policy, DmarcPolicy::None);
        assert!(!result.spf_aligned);
        assert!(!result.dkim_aligned);
        assert!(!result.pass);
        assert!(result.reason.is_some());
    }

    #[test]
    fn test_dmarc_validator_creation() {
        let validator = DmarcValidator::new();
        assert!(Arc::strong_count(&validator.resolver) >= 1);
    }

    #[test]
    fn test_domains_align_exact_match() {
        let validator = DmarcValidator::new();

        assert!(validator.domains_align("example.com", "example.com", &DmarcAlignment::Strict));
        assert!(validator.domains_align("example.com", "example.com", &DmarcAlignment::Relaxed));
    }

    #[test]
    fn test_domains_align_case_insensitive() {
        let validator = DmarcValidator::new();

        assert!(validator.domains_align("Example.COM", "example.com", &DmarcAlignment::Strict));
        assert!(validator.domains_align("EXAMPLE.COM", "example.com", &DmarcAlignment::Relaxed));
    }

    #[test]
    fn test_domains_align_subdomain_relaxed() {
        let validator = DmarcValidator::new();

        // Relaxed mode allows subdomain alignment
        assert!(validator.domains_align(
            "mail.example.com",
            "example.com",
            &DmarcAlignment::Relaxed
        ));
        assert!(validator.domains_align(
            "example.com",
            "mail.example.com",
            &DmarcAlignment::Relaxed
        ));
    }

    #[test]
    fn test_domains_align_subdomain_strict() {
        let validator = DmarcValidator::new();

        // Strict mode requires exact match
        assert!(!validator.domains_align(
            "mail.example.com",
            "example.com",
            &DmarcAlignment::Strict
        ));
        assert!(!validator.domains_align(
            "example.com",
            "mail.example.com",
            &DmarcAlignment::Strict
        ));
    }

    #[test]
    fn test_domains_align_different_domains() {
        let validator = DmarcValidator::new();

        assert!(!validator.domains_align("example.com", "other.com", &DmarcAlignment::Strict));
        assert!(!validator.domains_align("example.com", "other.com", &DmarcAlignment::Relaxed));
    }

    #[test]
    fn test_check_spf_alignment() {
        let validator = DmarcValidator::new();

        let spf_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@example.com".to_string(),
            reason: None,
        };

        assert!(validator.check_spf_alignment("example.com", &spf_result));
        assert!(!validator.check_spf_alignment("other.com", &spf_result));
    }

    #[test]
    fn test_check_spf_alignment_subdomain() {
        let validator = DmarcValidator::new();

        let spf_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@mail.example.com".to_string(),
            reason: None,
        };

        // Relaxed alignment allows subdomain
        assert!(validator.check_spf_alignment("example.com", &spf_result));
    }

    #[test]
    fn test_check_dkim_alignment() {
        let validator = DmarcValidator::new();

        let dkim_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        assert!(validator.check_dkim_alignment("example.com", &dkim_result));
        assert!(!validator.check_dkim_alignment("other.com", &dkim_result));
    }

    #[test]
    fn test_check_dkim_alignment_subdomain() {
        let validator = DmarcValidator::new();

        let dkim_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "mail.example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        // Relaxed alignment allows subdomain
        assert!(validator.check_dkim_alignment("example.com", &dkim_result));
    }

    #[test]
    fn test_should_reject_with_reject_policy() {
        let validator = DmarcValidator::new();

        let result = DmarcResult {
            policy: DmarcPolicy::Reject,
            spf_aligned: false,
            dkim_aligned: false,
            pass: false,
            reason: Some("DMARC failed".to_string()),
        };

        assert!(validator.should_reject(&result));
    }

    #[test]
    fn test_should_not_reject_with_none_policy() {
        let validator = DmarcValidator::new();

        let result = DmarcResult {
            policy: DmarcPolicy::None,
            spf_aligned: false,
            dkim_aligned: false,
            pass: false,
            reason: Some("DMARC failed".to_string()),
        };

        assert!(!validator.should_reject(&result));
    }

    #[test]
    fn test_should_quarantine_with_quarantine_policy() {
        let validator = DmarcValidator::new();

        let result = DmarcResult {
            policy: DmarcPolicy::Quarantine,
            spf_aligned: false,
            dkim_aligned: false,
            pass: false,
            reason: Some("DMARC failed".to_string()),
        };

        assert!(validator.should_quarantine(&result));
    }

    #[test]
    fn test_should_not_reject_if_pass() {
        let validator = DmarcValidator::new();

        let result = DmarcResult {
            policy: DmarcPolicy::Reject,
            spf_aligned: true,
            dkim_aligned: true,
            pass: true,
            reason: Some("DMARC passed".to_string()),
        };

        assert!(!validator.should_reject(&result));
    }

    #[tokio::test]
    async fn test_validate_with_aligned_spf() {
        let validator = DmarcValidator::new();

        let spf_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@example.com".to_string(),
            reason: None,
        };

        let dkim_result = DkimAuthResult {
            status: AuthenticationStatus::None,
            domain: "other.com".to_string(),
            selector: "default".to_string(),
            reason: Some("No DKIM signature".to_string()),
        };

        let result = validator
            .validate("example.com", &spf_result, &dkim_result)
            .await
            .unwrap();

        assert!(result.spf_aligned);
        assert!(!result.dkim_aligned);
        assert!(result.pass); // SPF aligned and passed
    }

    #[tokio::test]
    async fn test_validate_with_aligned_dkim() {
        let validator = DmarcValidator::new();

        let spf_result = SpfAuthResult {
            status: AuthenticationStatus::Fail,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@other.com".to_string(),
            reason: Some("SPF failed".to_string()),
        };

        let dkim_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        let result = validator
            .validate("example.com", &spf_result, &dkim_result)
            .await
            .unwrap();

        assert!(!result.spf_aligned); // Different domain
        assert!(result.dkim_aligned);
        assert!(result.pass); // DKIM aligned and passed
    }

    #[tokio::test]
    async fn test_validate_with_no_alignment() {
        let validator = DmarcValidator::new();

        let spf_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "192.0.2.1".to_string(),
            envelope_from: "sender@other.com".to_string(),
            reason: None,
        };

        let dkim_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "different.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        let result = validator
            .validate("example.com", &spf_result, &dkim_result)
            .await
            .unwrap();

        assert!(!result.spf_aligned);
        assert!(!result.dkim_aligned);
        assert!(!result.pass); // Neither aligned
    }
}
