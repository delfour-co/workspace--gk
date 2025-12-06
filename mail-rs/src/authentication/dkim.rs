use super::types::{AuthenticationStatus, DkimAuthResult};
use anyhow::{anyhow, Result};
use mail_auth::common::crypto::{RsaKey, Sha256};
use mail_auth::common::headers::HeaderWriter;
use mail_auth::common::verify::VerifySignature;
use mail_auth::{AuthenticatedMessage, DkimResult as MailAuthDkimResult, Resolver};
use mail_auth::dkim::DkimSigner as MailAuthDkimSigner;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// DKIM signer for outgoing emails
pub struct DkimSigner {
    domain: String,
    selector: String,
    private_key: Vec<u8>,
}

/// DKIM validator for incoming emails
pub struct DkimValidator {
    resolver: Arc<Resolver>,
}

/// DKIM validation result
pub type DkimResult = Result<DkimAuthResult>;

impl DkimSigner {
    /// Create a new DKIM signer
    ///
    /// # Arguments
    /// * `domain` - Domain name (e.g., "example.com")
    /// * `selector` - DKIM selector (e.g., "default", "mail")
    /// * `private_key_path` - Path to private key file (PEM format)
    pub fn new(domain: String, selector: String, private_key_path: &Path) -> Result<Self> {
        let private_key = fs::read(private_key_path)?;

        Ok(Self {
            domain,
            selector,
            private_key,
        })
    }

    /// Sign an email message with DKIM
    ///
    /// # Arguments
    /// * `message` - Complete email message (headers + body)
    ///
    /// # Returns
    /// DKIM-Signature header value
    pub fn sign(&self, message: &[u8]) -> Result<String> {
        info!(
            "Signing email with DKIM (domain: {}, selector: {})",
            self.domain, self.selector
        );

        // Load RSA key from PEM
        let private_key_str = String::from_utf8(self.private_key.clone())?;
        let rsa_key = RsaKey::<Sha256>::from_rsa_pem(&private_key_str)
            .map_err(|e| anyhow!("Failed to load RSA key: {}", e))?;

        // Create DKIM signature using mail-auth
        let signature = MailAuthDkimSigner::from_key(rsa_key)
            .domain(&self.domain)
            .selector(&self.selector)
            .headers(["From", "To", "Subject", "Date", "Message-ID"])
            .sign(message)
            .map_err(|e| anyhow!("DKIM signing failed: {}", e))?;

        debug!("DKIM signature generated successfully");

        Ok(signature.to_header())
    }

    /// Add DKIM signature to email headers
    ///
    /// # Arguments
    /// * `message` - Original email message
    ///
    /// # Returns
    /// Modified email with DKIM-Signature header prepended
    pub fn sign_and_prepend(&self, message: &[u8]) -> Result<Vec<u8>> {
        let signature = self.sign(message)?;

        // Prepend DKIM-Signature header to message
        let mut signed_message = Vec::new();
        signed_message.extend_from_slice(b"DKIM-Signature: ");
        signed_message.extend_from_slice(signature.as_bytes());
        signed_message.extend_from_slice(b"\r\n");
        signed_message.extend_from_slice(message);

        Ok(signed_message)
    }

    /// Get the DNS TXT record value for DKIM public key
    ///
    /// Returns the value that should be published in DNS at:
    /// `{selector}._domainkey.{domain}`
    pub fn get_public_key_dns_record(&self) -> Result<String> {
        // For now, this is a placeholder. In a real implementation,
        // you would extract the public key from the private key
        // and format it for DNS TXT record.
        //
        // The format is: v=DKIM1; k=rsa; p=<base64_public_key>
        Ok(format!(
            "v=DKIM1; k=rsa; p=<public_key_here>; n=DKIM key for {}",
            self.domain
        ))
    }
}

impl DkimValidator {
    /// Create a new DKIM validator
    pub fn new() -> Self {
        let resolver = Resolver::new_system_conf().unwrap_or_else(|_| {
            warn!("Failed to load system DNS config, using default resolver");
            Resolver::new_cloudflare_tls().expect("Failed to create DNS resolver")
        });

        Self {
            resolver: Arc::new(resolver),
        }
    }

    /// Validate DKIM signature(s) in an incoming email
    ///
    /// # Arguments
    /// * `message` - Complete email message including DKIM-Signature header(s)
    ///
    /// # Returns
    /// DKIM validation result
    pub async fn validate(&self, message: &[u8]) -> DkimResult {
        info!("Validating DKIM signature(s) in email");

        // Parse the authenticated message
        let parsed_message = match AuthenticatedMessage::parse(message) {
            Some(msg) => msg,
            None => {
                warn!("Failed to parse message for DKIM validation");
                return Ok(DkimAuthResult {
                    status: AuthenticationStatus::PermError,
                    domain: String::new(),
                    selector: String::new(),
                    reason: Some("Failed to parse message".to_string()),
                });
            }
        };

        // Verify DKIM signatures
        let dkim_results = self.resolver.verify_dkim(&parsed_message).await;

        debug!("DKIM verification returned {} results", dkim_results.len());

        // If no signatures found
        if dkim_results.is_empty() {
            debug!("No DKIM signature found");
            return Ok(DkimAuthResult {
                status: AuthenticationStatus::None,
                domain: String::new(),
                selector: String::new(),
                reason: Some("No DKIM signature present".to_string()),
            });
        }

        // Get the first result (emails can have multiple signatures)
        // In a real implementation, you'd want to check if ANY signature passes
        let first_result = &dkim_results[0];
        let dkim_result = first_result.result();

        debug!("DKIM verification result: {:?}", dkim_result);

        // Convert mail-auth result to our AuthenticationStatus
        let (status, reason) = match dkim_result {
            MailAuthDkimResult::Pass => {
                info!("DKIM validation passed");
                (
                    AuthenticationStatus::Pass,
                    Some("DKIM signature valid".to_string()),
                )
            }
            MailAuthDkimResult::Fail(err) => {
                warn!("DKIM validation failed: {:?}", err);
                (
                    AuthenticationStatus::Fail,
                    Some(format!("DKIM signature invalid: {:?}", err)),
                )
            }
            MailAuthDkimResult::Neutral(err) => {
                info!("DKIM validation neutral: {:?}", err);
                (
                    AuthenticationStatus::Neutral,
                    Some(format!("DKIM signature validation inconclusive: {:?}", err)),
                )
            }
            MailAuthDkimResult::TempError(err) => {
                warn!("DKIM temporary error: {:?}", err);
                (
                    AuthenticationStatus::TempError,
                    Some(format!("Temporary error during DKIM validation: {:?}", err)),
                )
            }
            MailAuthDkimResult::PermError(err) => {
                warn!("DKIM permanent error: {:?}", err);
                (
                    AuthenticationStatus::PermError,
                    Some(format!("Permanent error in DKIM signature: {:?}", err)),
                )
            }
            MailAuthDkimResult::None => {
                debug!("No DKIM signature in result");
                (
                    AuthenticationStatus::None,
                    Some("No DKIM signature present".to_string()),
                )
            }
        };

        // Extract domain from the signature (if available)
        let domain = first_result.signature()
            .map(|sig| sig.domain().to_string())
            .unwrap_or_else(|| self.extract_domain_from_message(message));

        let selector = first_result.signature()
            .map(|sig| sig.selector().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        Ok(DkimAuthResult {
            status,
            domain,
            selector,
            reason,
        })
    }

    /// Extract domain from message (from From header)
    fn extract_domain_from_message(&self, message: &[u8]) -> String {
        // Simple extraction - in production, use proper email parser
        let message_str = String::from_utf8_lossy(message);

        for line in message_str.lines() {
            if line.to_lowercase().starts_with("from:") {
                if let Some(email_start) = line.find('<') {
                    if let Some(email_end) = line.find('>') {
                        let email = &line[email_start + 1..email_end];
                        if let Some(at_pos) = email.find('@') {
                            return email[at_pos + 1..].to_string();
                        }
                    }
                }
                // Fallback: find @domain pattern
                if let Some(at_pos) = line.find('@') {
                    let after_at = &line[at_pos + 1..];
                    if let Some(space_pos) = after_at.find(|c: char| c.is_whitespace()) {
                        return after_at[..space_pos].to_string();
                    }
                    return after_at.trim().to_string();
                }
            }
        }

        "unknown".to_string()
    }

    /// Check if DKIM result should cause email rejection
    pub fn should_reject(&self, result: &DkimAuthResult) -> bool {
        // Generally, we don't reject on DKIM failure alone
        // (combine with SPF and DMARC for better decisions)
        matches!(result.status, AuthenticationStatus::Fail)
    }

    /// Check if lack of DKIM should cause suspicion
    pub fn should_flag_missing_signature(&self, result: &DkimAuthResult) -> bool {
        matches!(result.status, AuthenticationStatus::None)
    }
}

impl Default for DkimValidator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_dkim_signer_creation_with_invalid_key() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"invalid key data").unwrap();

        // Should succeed in creating signer (validation happens during signing)
        let result = DkimSigner::new(
            "example.com".to_string(),
            "default".to_string(),
            temp_file.path(),
        );

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_dkim_validator_creation() {
        let validator = DkimValidator::new();
        // Just check it was created successfully
        assert!(Arc::strong_count(&validator.resolver) >= 1);
    }

    #[tokio::test]
    async fn test_dkim_validation_no_signature() {
        let validator = DkimValidator::new();

        let message = b"From: test@example.com\r\n\
                       To: recipient@example.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let result = validator.validate(message).await;
        assert!(result.is_ok());

        let dkim_result = result.unwrap();
        assert_eq!(dkim_result.status, AuthenticationStatus::None);
    }

    #[test]
    fn test_extract_domain_from_message() {
        let validator = DkimValidator::new();

        let message = b"From: Test User <test@example.com>\r\n\
                       To: recipient@test.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let domain = validator.extract_domain_from_message(message);
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_should_reject() {
        let validator = DkimValidator::new();

        let fail_result = DkimAuthResult {
            status: AuthenticationStatus::Fail,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        assert!(validator.should_reject(&fail_result));

        let pass_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        assert!(!validator.should_reject(&pass_result));
    }

    #[test]
    fn test_should_not_reject_neutral() {
        let validator = DkimValidator::new();

        let neutral_result = DkimAuthResult {
            status: AuthenticationStatus::Neutral,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("Signature validation inconclusive".to_string()),
        };

        assert!(!validator.should_reject(&neutral_result));
    }

    #[test]
    fn test_should_not_reject_temperror() {
        let validator = DkimValidator::new();

        let temperror_result = DkimAuthResult {
            status: AuthenticationStatus::TempError,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("DNS timeout".to_string()),
        };

        // Temporary errors should not cause rejection
        assert!(!validator.should_reject(&temperror_result));
    }

    #[test]
    fn test_should_not_reject_permerror() {
        let validator = DkimValidator::new();

        let permerror_result = DkimAuthResult {
            status: AuthenticationStatus::PermError,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("Invalid signature format".to_string()),
        };

        // Permanent errors should not cause rejection (message was delivered, just couldn't verify)
        assert!(!validator.should_reject(&permerror_result));
    }

    #[test]
    fn test_should_not_reject_none() {
        let validator = DkimValidator::new();

        let none_result = DkimAuthResult {
            status: AuthenticationStatus::None,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("No DKIM signature present".to_string()),
        };

        // Missing DKIM signature should not cause rejection
        assert!(!validator.should_reject(&none_result));
    }

    #[test]
    fn test_should_flag_missing_signature() {
        let validator = DkimValidator::new();

        let none_result = DkimAuthResult {
            status: AuthenticationStatus::None,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: Some("No DKIM signature present".to_string()),
        };

        assert!(validator.should_flag_missing_signature(&none_result));

        let pass_result = DkimAuthResult {
            status: AuthenticationStatus::Pass,
            domain: "example.com".to_string(),
            selector: "default".to_string(),
            reason: None,
        };

        assert!(!validator.should_flag_missing_signature(&pass_result));
    }

    #[test]
    fn test_extract_domain_from_message_plain_email() {
        let validator = DkimValidator::new();

        let message = b"From: test@example.com\r\n\
                       To: recipient@test.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let domain = validator.extract_domain_from_message(message);
        assert_eq!(domain, "example.com");
    }

    #[test]
    fn test_extract_domain_from_message_with_name() {
        let validator = DkimValidator::new();

        let message = b"From: \"John Doe\" <john@example.org>\r\n\
                       To: recipient@test.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let domain = validator.extract_domain_from_message(message);
        assert_eq!(domain, "example.org");
    }

    #[test]
    fn test_extract_domain_from_message_unknown() {
        let validator = DkimValidator::new();

        let message = b"To: recipient@test.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let domain = validator.extract_domain_from_message(message);
        assert_eq!(domain, "unknown");
    }

    #[test]
    fn test_dkim_validator_default() {
        let validator = DkimValidator::default();
        // Should successfully create validator using default trait
        assert!(Arc::strong_count(&validator.resolver) >= 1);
    }

    #[test]
    fn test_dkim_result_with_reason() {
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
        assert_eq!(result.reason.unwrap(), "DKIM signature valid");
    }

    #[test]
    fn test_dkim_result_all_statuses() {
        let validator = DkimValidator::new();

        let statuses = vec![
            (AuthenticationStatus::Pass, false, false),
            (AuthenticationStatus::Fail, true, false),
            (AuthenticationStatus::Neutral, false, false),
            (AuthenticationStatus::TempError, false, false),
            (AuthenticationStatus::PermError, false, false),
            (AuthenticationStatus::None, false, true),
        ];

        for (status, should_reject, should_flag_missing) in statuses {
            let result = DkimAuthResult {
                status: status.clone(),
                domain: "example.com".to_string(),
                selector: "default".to_string(),
                reason: None,
            };

            assert_eq!(
                validator.should_reject(&result),
                should_reject,
                "should_reject mismatch for {:?}",
                status
            );
            assert_eq!(
                validator.should_flag_missing_signature(&result),
                should_flag_missing,
                "should_flag_missing mismatch for {:?}",
                status
            );
        }
    }

    #[test]
    fn test_fail_result_should_reject() {
        let validator = DkimValidator::new();

        let fail_result = DkimAuthResult {
            status: AuthenticationStatus::Fail,
            domain: "evil.com".to_string(),
            selector: "default".to_string(),
            reason: Some("Invalid signature".to_string()),
        };

        // Fail should cause rejection
        assert!(validator.should_reject(&fail_result));
        // But not flag as missing (it was present, just invalid)
        assert!(!validator.should_flag_missing_signature(&fail_result));
    }

    #[test]
    fn test_dkim_signer_get_public_key_dns_record() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"dummy key for test").unwrap();

        let signer = DkimSigner::new(
            "example.com".to_string(),
            "selector1".to_string(),
            temp_file.path(),
        )
        .unwrap();

        let dns_record = signer.get_public_key_dns_record().unwrap();
        assert!(dns_record.contains("v=DKIM1"));
        assert!(dns_record.contains("k=rsa"));
        assert!(dns_record.contains("example.com"));
    }

    #[test]
    fn test_extract_domain_with_multiple_at_signs() {
        let validator = DkimValidator::new();

        // Edge case: multiple @ signs (shouldn't happen in valid email but test robustness)
        let message = b"From: user@@example.com\r\n\
                       Subject: Test\r\n\
                       \r\n\
                       Body";

        let domain = validator.extract_domain_from_message(message);
        // Should extract after the first @
        assert!(domain.contains("@example.com") || domain == "example.com");
    }

    #[tokio::test]
    async fn test_dkim_validation_malformed_message() {
        let validator = DkimValidator::new();

        // Completely invalid message (not RFC 822 compliant)
        let message = b"This is not a valid email message at all";

        let result = validator.validate(message).await;
        assert!(result.is_ok());

        let dkim_result = result.unwrap();
        // Should handle gracefully with PermError or None
        assert!(
            dkim_result.status == AuthenticationStatus::PermError
                || dkim_result.status == AuthenticationStatus::None
        );
    }

    #[test]
    fn test_dkim_signer_domain_and_selector() {
        let mut temp_file = NamedTempFile::new().unwrap();
        temp_file.write_all(b"test key").unwrap();

        let signer = DkimSigner::new(
            "mail.example.com".to_string(),
            "selector2025".to_string(),
            temp_file.path(),
        )
        .unwrap();

        assert_eq!(signer.domain, "mail.example.com");
        assert_eq!(signer.selector, "selector2025");
    }
}
