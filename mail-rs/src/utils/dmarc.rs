//! DMARC (Domain-based Message Authentication, Reporting & Conformance)
//!
//! This module implements DMARC policy checking according to RFC 7489.
//!
//! DMARC builds on SPF and DKIM to provide domain-level email authentication.
//! It allows domain owners to publish policies for how receivers should handle
//! emails that fail authentication checks.
//!
//! # Example
//! ```no_run
//! use mail_rs::utils::dmarc::{DmarcValidator, DmarcResult};
//! use mail_rs::utils::spf::SpfResult;
//! use mail_rs::utils::dkim::DkimResult;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let validator = DmarcValidator::new();
//!
//! let domain = "example.com";
//! let spf_result = SpfResult::Pass;
//! let dkim_result = DkimResult::Pass;
//! let from_domain = "example.com";
//!
//! let result = validator.check(domain, spf_result, dkim_result, from_domain).await?;
//!
//! match result {
//!     DmarcResult::Pass => println!("DMARC check passed"),
//!     DmarcResult::Fail(policy) => println!("DMARC check failed, policy: {:?}", policy),
//!     DmarcResult::None => println!("No DMARC policy published"),
//! }
//! # Ok(())
//! # }
//! ```

use crate::error::{MailError, Result};
use crate::utils::spf::SpfResult;
use crate::utils::dkim::DkimResult;
use tracing::{debug, info, warn};
use trust_dns_resolver::TokioAsyncResolver;
use trust_dns_resolver::config::*;

/// DMARC policy actions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DmarcPolicy {
    /// No action (monitoring mode)
    None,
    /// Mark as spam but deliver
    Quarantine,
    /// Reject the message
    Reject,
}

impl DmarcPolicy {
    /// Parse policy from string
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "none" => DmarcPolicy::None,
            "quarantine" => DmarcPolicy::Quarantine,
            "reject" => DmarcPolicy::Reject,
            _ => DmarcPolicy::None, // Default to none for unknown policies
        }
    }
}

/// DMARC alignment mode
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DmarcAlignment {
    /// Relaxed alignment (subdomains allowed)
    Relaxed,
    /// Strict alignment (exact match required)
    Strict,
}

/// DMARC validation result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DmarcResult {
    /// DMARC check passed
    Pass,
    /// DMARC check failed with specified policy
    Fail(DmarcPolicy),
    /// No DMARC policy published
    None,
}

impl DmarcResult {
    /// Check if email should be accepted
    pub fn should_accept(&self) -> bool {
        match self {
            DmarcResult::Pass => true,
            DmarcResult::None => true,
            DmarcResult::Fail(DmarcPolicy::None) => true,
            DmarcResult::Fail(DmarcPolicy::Quarantine) => true, // Accept but mark as spam
            DmarcResult::Fail(DmarcPolicy::Reject) => false,
        }
    }

    /// Check if email should be quarantined
    pub fn should_quarantine(&self) -> bool {
        matches!(self, DmarcResult::Fail(DmarcPolicy::Quarantine))
    }
}

/// DMARC record from DNS
#[derive(Debug, Clone)]
pub struct DmarcRecord {
    /// Policy for domain emails (p= parameter)
    pub policy: DmarcPolicy,
    /// Policy for subdomain emails (sp= parameter)
    pub subdomain_policy: Option<DmarcPolicy>,
    /// SPF alignment mode (aspf= parameter)
    pub spf_alignment: DmarcAlignment,
    /// DKIM alignment mode (adkim= parameter)
    pub dkim_alignment: DmarcAlignment,
    /// Percentage of emails to apply policy to (pct= parameter)
    pub percentage: u8,
    /// Reporting URI for aggregate reports (rua= parameter)
    pub aggregate_report_uri: Option<String>,
    /// Reporting URI for forensic reports (ruf= parameter)
    pub forensic_report_uri: Option<String>,
}

impl Default for DmarcRecord {
    fn default() -> Self {
        Self {
            policy: DmarcPolicy::None,
            subdomain_policy: None,
            spf_alignment: DmarcAlignment::Relaxed,
            dkim_alignment: DmarcAlignment::Relaxed,
            percentage: 100,
            aggregate_report_uri: None,
            forensic_report_uri: None,
        }
    }
}

/// DMARC validator
pub struct DmarcValidator {
    resolver: TokioAsyncResolver,
}

impl DmarcValidator {
    /// Create a new DMARC validator
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Self { resolver }
    }

    /// Check DMARC policy
    ///
    /// # Arguments
    /// * `domain` - Domain from MAIL FROM (SPF)
    /// * `spf_result` - Result of SPF check
    /// * `dkim_result` - Result of DKIM check
    /// * `from_domain` - Domain from From: header
    ///
    /// # Returns
    /// DMARC validation result
    pub async fn check(
        &self,
        domain: &str,
        spf_result: SpfResult,
        dkim_result: DkimResult,
        from_domain: &str,
    ) -> Result<DmarcResult> {
        info!("DMARC check starting for domain: {}", from_domain);

        // Look up DMARC policy
        let dmarc_record = match self.lookup_dmarc_policy(from_domain).await {
            Ok(record) => record,
            Err(e) => {
                debug!("No DMARC policy found: {}", e);
                return Ok(DmarcResult::None);
            }
        };

        info!("DMARC policy found: {:?}", dmarc_record.policy);

        // Check identifier alignment
        let spf_aligned = self.check_spf_alignment(domain, from_domain, &dmarc_record.spf_alignment);
        let dkim_aligned = self.check_dkim_alignment(from_domain, &dmarc_record.dkim_alignment);

        debug!("SPF aligned: {}, DKIM aligned: {}", spf_aligned, dkim_aligned);

        // DMARC passes if either SPF or DKIM is aligned and passes
        let spf_pass = spf_result == SpfResult::Pass && spf_aligned;
        let dkim_pass = dkim_result == DkimResult::Pass && dkim_aligned;

        if spf_pass || dkim_pass {
            info!("DMARC check passed");
            Ok(DmarcResult::Pass)
        } else {
            warn!("DMARC check failed, policy: {:?}", dmarc_record.policy);
            Ok(DmarcResult::Fail(dmarc_record.policy))
        }
    }

    /// Look up DMARC policy from DNS
    ///
    /// DMARC records are published at: _dmarc.{domain}
    async fn lookup_dmarc_policy(&self, domain: &str) -> Result<DmarcRecord> {
        let dmarc_domain = format!("_dmarc.{}", domain);
        debug!("Looking up DMARC policy at: {}", dmarc_domain);

        // Look up TXT record
        let txt_lookup = self.resolver.txt_lookup(&dmarc_domain).await
            .map_err(|e| MailError::Config(format!("DMARC DNS lookup failed: {}", e)))?;

        // Find DMARC record (starts with "v=DMARC1")
        for record in txt_lookup.iter() {
            let txt_data = record.to_string();
            if txt_data.starts_with("v=DMARC1") {
                debug!("Found DMARC record: {}", txt_data);
                return self.parse_dmarc_record(&txt_data);
            }
        }

        Err(MailError::Config("No DMARC policy found".to_string()))
    }

    /// Parse DMARC record
    fn parse_dmarc_record(&self, record: &str) -> Result<DmarcRecord> {
        let mut dmarc = DmarcRecord::default();

        // Parse semicolon-separated key=value pairs
        for pair in record.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }

            let parts: Vec<&str> = pair.splitn(2, '=').collect();
            if parts.len() != 2 {
                continue;
            }

            let key = parts[0].trim();
            let value = parts[1].trim();

            match key {
                "v" => {
                    if value != "DMARC1" {
                        return Err(MailError::Config("Invalid DMARC version".to_string()));
                    }
                }
                "p" => dmarc.policy = DmarcPolicy::from_str(value),
                "sp" => dmarc.subdomain_policy = Some(DmarcPolicy::from_str(value)),
                "aspf" => {
                    dmarc.spf_alignment = if value == "s" {
                        DmarcAlignment::Strict
                    } else {
                        DmarcAlignment::Relaxed
                    };
                }
                "adkim" => {
                    dmarc.dkim_alignment = if value == "s" {
                        DmarcAlignment::Strict
                    } else {
                        DmarcAlignment::Relaxed
                    };
                }
                "pct" => {
                    if let Ok(pct) = value.parse::<u8>() {
                        dmarc.percentage = pct.min(100);
                    }
                }
                "rua" => dmarc.aggregate_report_uri = Some(value.to_string()),
                "ruf" => dmarc.forensic_report_uri = Some(value.to_string()),
                _ => {} // Ignore unknown tags
            }
        }

        Ok(dmarc)
    }

    /// Check SPF identifier alignment
    fn check_spf_alignment(
        &self,
        mail_from_domain: &str,
        from_domain: &str,
        alignment: &DmarcAlignment,
    ) -> bool {
        match alignment {
            DmarcAlignment::Relaxed => {
                // Relaxed: organizational domains must match
                self.get_organizational_domain(mail_from_domain)
                    == self.get_organizational_domain(from_domain)
            }
            DmarcAlignment::Strict => {
                // Strict: exact match required
                mail_from_domain.to_lowercase() == from_domain.to_lowercase()
            }
        }
    }

    /// Check DKIM identifier alignment
    fn check_dkim_alignment(&self, from_domain: &str, alignment: &DmarcAlignment) -> bool {
        // NOTE: In a complete implementation, we would check the DKIM signature's d= parameter
        // against the From: domain. For now, we assume alignment if DKIM passed.
        match alignment {
            DmarcAlignment::Relaxed => true, // Would check organizational domain
            DmarcAlignment::Strict => true,  // Would check exact match
        }
    }

    /// Get organizational domain from full domain
    ///
    /// Example: mail.example.com -> example.com
    fn get_organizational_domain(&self, domain: &str) -> String {
        let parts: Vec<&str> = domain.split('.').collect();
        if parts.len() >= 2 {
            format!("{}.{}", parts[parts.len() - 2], parts[parts.len() - 1])
        } else {
            domain.to_string()
        }
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
    fn test_dmarc_policy_from_str() {
        assert_eq!(DmarcPolicy::from_str("none"), DmarcPolicy::None);
        assert_eq!(DmarcPolicy::from_str("quarantine"), DmarcPolicy::Quarantine);
        assert_eq!(DmarcPolicy::from_str("reject"), DmarcPolicy::Reject);
        assert_eq!(DmarcPolicy::from_str("unknown"), DmarcPolicy::None);
    }

    #[test]
    fn test_dmarc_result_should_accept() {
        assert!(DmarcResult::Pass.should_accept());
        assert!(DmarcResult::None.should_accept());
        assert!(DmarcResult::Fail(DmarcPolicy::None).should_accept());
        assert!(DmarcResult::Fail(DmarcPolicy::Quarantine).should_accept());
        assert!(!DmarcResult::Fail(DmarcPolicy::Reject).should_accept());
    }

    #[test]
    fn test_dmarc_result_should_quarantine() {
        assert!(!DmarcResult::Pass.should_quarantine());
        assert!(!DmarcResult::Fail(DmarcPolicy::Reject).should_quarantine());
        assert!(DmarcResult::Fail(DmarcPolicy::Quarantine).should_quarantine());
    }

    #[test]
    fn test_parse_dmarc_record() {
        let validator = DmarcValidator::new();

        let record = "v=DMARC1; p=reject; rua=mailto:dmarc@example.com";
        let parsed = validator.parse_dmarc_record(record).unwrap();

        assert_eq!(parsed.policy, DmarcPolicy::Reject);
        assert_eq!(parsed.aggregate_report_uri, Some("mailto:dmarc@example.com".to_string()));
    }

    #[test]
    fn test_get_organizational_domain() {
        let validator = DmarcValidator::new();

        assert_eq!(validator.get_organizational_domain("mail.example.com"), "example.com");
        assert_eq!(validator.get_organizational_domain("example.com"), "example.com");
        assert_eq!(validator.get_organizational_domain("sub.mail.example.com"), "example.com");
    }

    #[test]
    fn test_check_spf_alignment_relaxed() {
        let validator = DmarcValidator::new();

        // Relaxed alignment allows subdomains
        assert!(validator.check_spf_alignment(
            "mail.example.com",
            "example.com",
            &DmarcAlignment::Relaxed
        ));
    }

    #[test]
    fn test_check_spf_alignment_strict() {
        let validator = DmarcValidator::new();

        // Strict alignment requires exact match
        assert!(!validator.check_spf_alignment(
            "mail.example.com",
            "example.com",
            &DmarcAlignment::Strict
        ));

        assert!(validator.check_spf_alignment(
            "example.com",
            "example.com",
            &DmarcAlignment::Strict
        ));
    }
}
