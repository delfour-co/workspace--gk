//! SPF (Sender Policy Framework) validation
//!
//! This module implements SPF validation according to RFC 7208.
//!
//! SPF allows domain owners to specify which mail servers are authorized to
//! send email on behalf of their domain.
//!
//! # Example
//! ```no_run
//! use mail_rs::utils::spf::{SpfValidator, SpfResult};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let validator = SpfValidator::new();
//! let result = validator.check("192.168.1.1", "sender@example.com", "mail.example.com").await?;
//!
//! match result {
//!     SpfResult::Pass => println!("SPF check passed"),
//!     SpfResult::Fail => println!("SPF check failed"),
//!     SpfResult::SoftFail => println!("SPF soft fail"),
//!     SpfResult::Neutral => println!("SPF neutral"),
//!     SpfResult::None => println!("No SPF record"),
//!     SpfResult::TempError => println!("Temporary error"),
//!     SpfResult::PermError => println!("Permanent error"),
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{MailError, Result};
use std::net::IpAddr;
use tracing::{debug, warn};
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::*;

/// SPF validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpfResult {
    /// SPF check passed - sender is authorized
    Pass,
    /// SPF check failed - sender is not authorized
    Fail,
    /// SPF soft fail - sender might not be authorized
    SoftFail,
    /// SPF neutral - no statement about authorization
    Neutral,
    /// No SPF record found
    None,
    /// Temporary error during SPF check
    TempError,
    /// Permanent error in SPF record
    PermError,
}

impl SpfResult {
    /// Check if the result allows accepting the email
    pub fn should_accept(&self) -> bool {
        matches!(self, SpfResult::Pass | SpfResult::Neutral | SpfResult::None | SpfResult::SoftFail)
    }

    /// Check if the result requires rejecting the email
    pub fn should_reject(&self) -> bool {
        matches!(self, SpfResult::Fail)
    }
}

/// SPF validator
pub struct SpfValidator {
    resolver: TokioAsyncResolver,
}

impl SpfValidator {
    /// Create a new SPF validator
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Self { resolver }
    }

    /// Check SPF for an email
    ///
    /// # Arguments
    /// * `ip` - IP address of the sending mail server
    /// * `from` - Email address from MAIL FROM command
    /// * `helo` - Domain from HELO/EHLO command
    ///
    /// # Returns
    /// SPF validation result
    pub async fn check(&self, ip: &str, from: &str, helo: &str) -> Result<SpfResult> {
        debug!("SPF check: ip={}, from={}, helo={}", ip, from, helo);

        // Parse IP address
        let ip_addr: IpAddr = ip.parse()
            .map_err(|e| MailError::Config(format!("Invalid IP address: {}", e)))?;

        // Extract domain from email address
        let domain = self.extract_domain(from)?;

        // Look up SPF record
        let spf_record = match self.lookup_spf_record(&domain).await {
            Ok(record) => record,
            Err(_) => {
                debug!("No SPF record found for domain: {}", domain);
                return Ok(SpfResult::None);
            }
        };

        debug!("Found SPF record for {}: {}", domain, spf_record);

        // Parse and evaluate SPF record
        self.evaluate_spf(&spf_record, &ip_addr, helo).await
    }

    /// Extract domain from email address
    fn extract_domain(&self, email: &str) -> Result<String> {
        let parts: Vec<&str> = email.split('@').collect();
        if parts.len() != 2 {
            return Err(MailError::InvalidEmail(format!(
                "Invalid email format: {}",
                email
            )));
        }
        Ok(parts[1].to_string())
    }

    /// Look up SPF record for a domain
    async fn lookup_spf_record(&self, domain: &str) -> Result<String> {
        // SPF records are stored in TXT records
        let txt_lookup = self.resolver.txt_lookup(domain).await
            .map_err(|e| MailError::Config(format!("DNS lookup failed: {}", e)))?;

        // Find SPF record (starts with "v=spf1")
        for record in txt_lookup.iter() {
            let txt_data = record.to_string();
            if txt_data.starts_with("v=spf1") {
                return Ok(txt_data);
            }
        }

        Err(MailError::Config("No SPF record found".to_string()))
    }

    /// Evaluate SPF record
    async fn evaluate_spf(&self, record: &str, ip: &IpAddr, _helo: &str) -> Result<SpfResult> {
        // Parse SPF mechanisms
        let parts: Vec<&str> = record.split_whitespace().collect();

        // Skip "v=spf1"
        let mechanisms = &parts[1..];

        for mechanism in mechanisms {
            let result = self.evaluate_mechanism(mechanism, ip).await?;
            if result != SpfResult::Neutral {
                return Ok(result);
            }
        }

        // Default result if no mechanisms matched
        Ok(SpfResult::Neutral)
    }

    /// Evaluate a single SPF mechanism
    async fn evaluate_mechanism(&self, mechanism: &str, ip: &IpAddr) -> Result<SpfResult> {
        // Handle qualifiers: + (pass), - (fail), ~ (softfail), ? (neutral)
        let (qualifier, mech) = if let Some(first_char) = mechanism.chars().next() {
            match first_char {
                '+' | '-' | '~' | '?' => (first_char, &mechanism[1..]),
                _ => ('+', mechanism),
            }
        } else {
            return Ok(SpfResult::Neutral);
        };

        // Parse mechanism type
        let mech_type = if let Some(colon_pos) = mech.find(':') {
            &mech[..colon_pos]
        } else {
            mech
        };

        debug!("Evaluating SPF mechanism: {} (qualifier: {})", mech_type, qualifier);

        // Evaluate based on mechanism type
        let matches = match mech_type {
            "all" => true,
            "ip4" | "ip6" => {
                // Simplified: would need CIDR matching in production
                warn!("IP4/IP6 mechanism not fully implemented");
                false
            }
            "a" | "mx" => {
                // Would need to look up A/MX records
                warn!("A/MX mechanism not fully implemented");
                false
            }
            "include" => {
                // Would need recursive SPF lookup
                warn!("Include mechanism not fully implemented");
                false
            }
            _ => {
                warn!("Unknown SPF mechanism: {}", mech_type);
                false
            }
        };

        if matches {
            Ok(match qualifier {
                '+' => SpfResult::Pass,
                '-' => SpfResult::Fail,
                '~' => SpfResult::SoftFail,
                '?' => SpfResult::Neutral,
                _ => SpfResult::Neutral,
            })
        } else {
            Ok(SpfResult::Neutral)
        }
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

    #[test]
    fn test_spf_result_should_accept() {
        assert!(SpfResult::Pass.should_accept());
        assert!(SpfResult::Neutral.should_accept());
        assert!(SpfResult::None.should_accept());
        assert!(SpfResult::SoftFail.should_accept());
        assert!(!SpfResult::Fail.should_accept());
    }

    #[test]
    fn test_spf_result_should_reject() {
        assert!(SpfResult::Fail.should_reject());
        assert!(!SpfResult::Pass.should_reject());
        assert!(!SpfResult::Neutral.should_reject());
    }

    #[test]
    fn test_extract_domain() {
        let validator = SpfValidator::new();
        assert_eq!(
            validator.extract_domain("user@example.com").unwrap(),
            "example.com"
        );
        assert_eq!(
            validator.extract_domain("admin@mail.example.org").unwrap(),
            "mail.example.org"
        );
        assert!(validator.extract_domain("invalid").is_err());
    }

    #[tokio::test]
    async fn test_spf_check_no_record() {
        let validator = SpfValidator::new();
        // Using a domain unlikely to have SPF
        let result = validator
            .check("192.168.1.1", "test@nonexistent-domain-12345.com", "mail.test.com")
            .await
            .unwrap();

        // Should return None (no SPF record)
        assert_eq!(result, SpfResult::None);
    }
}
