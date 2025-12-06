use super::types::{AuthenticationStatus, SpfAuthResult};
use anyhow::Result;
use mail_auth::{Resolver, SpfResult as MailAuthSpfResult};
use std::net::IpAddr;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// SPF validator for incoming emails
pub struct SpfValidator {
    resolver: Arc<Resolver>,
}

/// SPF validation result
pub type SpfResult = Result<SpfAuthResult>;

impl SpfValidator {
    /// Create a new SPF validator
    pub fn new() -> Self {
        let resolver = Resolver::new_system_conf().unwrap_or_else(|_| {
            warn!("Failed to load system DNS config, using default resolver");
            Resolver::new_cloudflare_tls().expect("Failed to create DNS resolver")
        });

        Self {
            resolver: Arc::new(resolver),
        }
    }

    /// Validate SPF for an incoming email
    ///
    /// # Arguments
    /// * `client_ip` - IP address of the SMTP client
    /// * `envelope_from` - MAIL FROM address
    /// * `helo_domain` - HELO/EHLO domain
    ///
    /// # Returns
    /// SPF validation result with status and details
    pub async fn validate(
        &self,
        client_ip: IpAddr,
        envelope_from: &str,
        helo_domain: &str,
    ) -> SpfResult {
        info!(
            "Validating SPF for {} from {} (HELO: {})",
            envelope_from, client_ip, helo_domain
        );

        // Extract domain from envelope_from (email address)
        let domain = envelope_from
            .split('@')
            .nth(1)
            .unwrap_or(envelope_from);

        // Perform SPF check using mail-auth Resolver
        let spf_output = self.resolver
            .verify_spf_sender(client_ip, helo_domain, domain, envelope_from)
            .await;

        // Get the SPF result
        let spf_result = spf_output.result();
        debug!("SPF result: {:?}", spf_result);

        // Convert mail-auth result to our AuthenticationStatus
        let status = match spf_result {
            MailAuthSpfResult::Pass => {
                info!("SPF validation passed for {}", envelope_from);
                AuthenticationStatus::Pass
            }
            MailAuthSpfResult::Fail => {
                warn!("SPF validation failed for {}", envelope_from);
                AuthenticationStatus::Fail
            }
            MailAuthSpfResult::SoftFail => {
                info!("SPF soft fail for {}", envelope_from);
                AuthenticationStatus::SoftFail
            }
            MailAuthSpfResult::Neutral => {
                info!("SPF neutral for {}", envelope_from);
                AuthenticationStatus::Neutral
            }
            MailAuthSpfResult::TempError => {
                warn!("SPF temporary error for {}", envelope_from);
                AuthenticationStatus::TempError
            }
            MailAuthSpfResult::PermError => {
                warn!("SPF permanent error for {}", envelope_from);
                AuthenticationStatus::PermError
            }
            MailAuthSpfResult::None => {
                debug!("No SPF record found for {}", envelope_from);
                AuthenticationStatus::None
            }
        };

        Ok(SpfAuthResult {
            status,
            client_ip: client_ip.to_string(),
            envelope_from: envelope_from.to_string(),
            reason: Some(self.get_reason_message(spf_result)),
        })
    }

    /// Get human-readable reason message for SPF result
    fn get_reason_message(&self, result: MailAuthSpfResult) -> String {
        match result {
            MailAuthSpfResult::Pass => {
                "Client IP is authorized to send for this domain".to_string()
            }
            MailAuthSpfResult::Fail => {
                "Client IP is not authorized to send for this domain".to_string()
            }
            MailAuthSpfResult::SoftFail => {
                "Client IP may not be authorized (soft fail policy)".to_string()
            }
            MailAuthSpfResult::Neutral => {
                "Domain owner does not assert whether IP is authorized".to_string()
            }
            MailAuthSpfResult::TempError => {
                "Temporary DNS error during SPF check".to_string()
            }
            MailAuthSpfResult::PermError => {
                "SPF record has a permanent error".to_string()
            }
            MailAuthSpfResult::None => {
                "Domain has no SPF record".to_string()
            }
        }
    }

    /// Check if SPF result should cause email rejection
    ///
    /// Returns true if email should be rejected based on SPF
    pub fn should_reject(&self, result: &SpfAuthResult) -> bool {
        matches!(result.status, AuthenticationStatus::Fail)
    }

    /// Check if SPF result should cause email to be marked as spam
    ///
    /// Returns true if email should be marked as suspicious
    pub fn should_flag_as_spam(&self, result: &SpfAuthResult) -> bool {
        matches!(
            result.status,
            AuthenticationStatus::SoftFail | AuthenticationStatus::Fail
        )
    }
}

impl Default for SpfValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[tokio::test]
    async fn test_spf_validator_creation() {
        let validator = SpfValidator::new();
        // Just check it was created successfully
        assert!(Arc::strong_count(&validator.resolver) >= 1);
    }

    #[tokio::test]
    async fn test_spf_pass_result() {
        let validator = SpfValidator::new();

        // Test with a known good SPF domain (Google)
        // Note: This is a real DNS test, may fail if DNS is unavailable
        let result = validator
            .validate(
                IpAddr::from_str("209.85.220.41").unwrap(), // Google SMTP server
                "test@gmail.com",
                "mail-wr1-f41.google.com",
            )
            .await;

        assert!(result.is_ok());
        let spf_result = result.unwrap();
        // Gmail should have SPF configured, so we should get some result
        assert_ne!(spf_result.status, AuthenticationStatus::None);
    }

    #[test]
    fn test_should_reject() {
        let validator = SpfValidator::new();

        let fail_result = SpfAuthResult {
            status: AuthenticationStatus::Fail,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: None,
        };

        assert!(validator.should_reject(&fail_result));

        let pass_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: None,
        };

        assert!(!validator.should_reject(&pass_result));
    }

    #[test]
    fn test_should_flag_as_spam() {
        let validator = SpfValidator::new();

        let softfail_result = SpfAuthResult {
            status: AuthenticationStatus::SoftFail,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: None,
        };

        assert!(validator.should_flag_as_spam(&softfail_result));

        let pass_result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: None,
        };

        assert!(!validator.should_flag_as_spam(&pass_result));
    }

    #[test]
    fn test_should_not_reject_softfail() {
        let validator = SpfValidator::new();

        let softfail_result = SpfAuthResult {
            status: AuthenticationStatus::SoftFail,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: Some("Soft fail policy".to_string()),
        };

        // SoftFail should flag as spam but NOT reject
        assert!(!validator.should_reject(&softfail_result));
        assert!(validator.should_flag_as_spam(&softfail_result));
    }

    #[test]
    fn test_should_not_reject_neutral() {
        let validator = SpfValidator::new();

        let neutral_result = SpfAuthResult {
            status: AuthenticationStatus::Neutral,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: Some("No assertion".to_string()),
        };

        assert!(!validator.should_reject(&neutral_result));
        assert!(!validator.should_flag_as_spam(&neutral_result));
    }

    #[test]
    fn test_should_not_reject_temperror() {
        let validator = SpfValidator::new();

        let temperror_result = SpfAuthResult {
            status: AuthenticationStatus::TempError,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: Some("DNS timeout".to_string()),
        };

        // Temporary errors should not cause rejection
        assert!(!validator.should_reject(&temperror_result));
    }

    #[test]
    fn test_should_not_reject_none() {
        let validator = SpfValidator::new();

        let none_result = SpfAuthResult {
            status: AuthenticationStatus::None,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: Some("No SPF record".to_string()),
        };

        // Missing SPF record should not cause rejection
        assert!(!validator.should_reject(&none_result));
    }

    #[test]
    fn test_get_reason_message_all_statuses() {
        let validator = SpfValidator::new();

        // Test that all status types have reason messages
        let statuses = vec![
            (MailAuthSpfResult::Pass, "authorized"),
            (MailAuthSpfResult::Fail, "not authorized"),
            (MailAuthSpfResult::SoftFail, "may not be authorized"),
            (MailAuthSpfResult::Neutral, "does not assert"),
            (MailAuthSpfResult::TempError, "error"),
            (MailAuthSpfResult::PermError, "error"),
            (MailAuthSpfResult::None, "no spf"),
        ];

        for (status, expected_keyword) in statuses {
            let reason = validator.get_reason_message(status);
            assert!(!reason.is_empty(), "Reason should not be empty for {:?}", status);
            assert!(
                reason.to_lowercase().contains(expected_keyword),
                "Reason '{}' should contain '{}'",
                reason,
                expected_keyword
            );
        }
    }

    #[test]
    fn test_spf_validator_default() {
        let validator = SpfValidator::default();
        // Should successfully create validator using default trait
        assert!(Arc::strong_count(&validator.resolver) >= 1);
    }

    #[test]
    fn test_spf_result_with_ipv6() {
        let result = SpfAuthResult {
            status: AuthenticationStatus::Pass,
            client_ip: "2001:db8::1".to_string(),
            envelope_from: "test@example.com".to_string(),
            reason: Some("IPv6 address authorized".to_string()),
        };

        assert_eq!(result.client_ip, "2001:db8::1");
        assert!(result.client_ip.contains(":"));
    }

    #[test]
    fn test_fail_result_should_be_flagged() {
        let validator = SpfValidator::new();

        let fail_result = SpfAuthResult {
            status: AuthenticationStatus::Fail,
            client_ip: "1.2.3.4".to_string(),
            envelope_from: "spammer@evil.com".to_string(),
            reason: Some("IP not authorized".to_string()),
        };

        // Fail should both reject AND flag as spam
        assert!(validator.should_reject(&fail_result));
        assert!(validator.should_flag_as_spam(&fail_result));
    }
}
