use super::types::{AuthenticationStatus, DkimAuthResult};
use anyhow::{anyhow, Result};
use mail_auth::dkim::{Dkim, DkimResult as MailAuthDkimResult, HashAlgorithm, Signature};
use mail_auth::Resolver;
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

        // Create DKIM signature
        let signature = Signature::new()
            .domain(&self.domain)
            .selector(&self.selector)
            .headers(&["From", "To", "Subject", "Date", "Message-ID"])
            .canonicalization_relaxed()
            .hash_algo(HashAlgorithm::Sha256)
            .sign(&self.private_key, message)
            .map_err(|e| anyhow!("DKIM signing failed: {}", e))?;

        debug!("DKIM signature generated successfully");

        Ok(signature.to_string())
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

        // Parse and verify DKIM signatures using mail-auth
        let dkim_result = Dkim::verify(message, &self.resolver).await;

        debug!("DKIM verification result: {:?}", dkim_result);

        // Extract first signature result (emails can have multiple signatures)
        let (status, domain, selector, reason) = match dkim_result {
            MailAuthDkimResult::Pass => {
                info!("DKIM validation passed");
                (
                    AuthenticationStatus::Pass,
                    self.extract_domain_from_message(message),
                    "unknown".to_string(),
                    Some("DKIM signature valid".to_string()),
                )
            }
            MailAuthDkimResult::Fail(_) => {
                warn!("DKIM validation failed");
                (
                    AuthenticationStatus::Fail,
                    self.extract_domain_from_message(message),
                    "unknown".to_string(),
                    Some("DKIM signature invalid".to_string()),
                )
            }
            MailAuthDkimResult::Neutral(_) => {
                info!("DKIM validation neutral");
                (
                    AuthenticationStatus::Neutral,
                    self.extract_domain_from_message(message),
                    "unknown".to_string(),
                    Some("DKIM signature validation inconclusive".to_string()),
                )
            }
            MailAuthDkimResult::TempError(_) => {
                warn!("DKIM temporary error");
                (
                    AuthenticationStatus::TempError,
                    self.extract_domain_from_message(message),
                    "unknown".to_string(),
                    Some("Temporary error during DKIM validation".to_string()),
                )
            }
            MailAuthDkimResult::PermError(_) => {
                warn!("DKIM permanent error");
                (
                    AuthenticationStatus::PermError,
                    self.extract_domain_from_message(message),
                    "unknown".to_string(),
                    Some("Permanent error in DKIM signature".to_string()),
                )
            }
            MailAuthDkimResult::None => {
                debug!("No DKIM signature found");
                (
                    AuthenticationStatus::None,
                    String::new(),
                    String::new(),
                    Some("No DKIM signature present".to_string()),
                )
            }
        };

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
        assert!(validator.resolver.is_locked().not());
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
}
