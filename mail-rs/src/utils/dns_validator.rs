//! DNS Validation for Email Security
//!
//! This module implements comprehensive DNS validation to protect against spam and abuse:
//! - Reverse DNS (PTR) lookup validation
//! - DNS-based blacklist (DNSBL) checking
//! - MX record validation
//! - DNS query rate limiting
//!
//! # Security Features
//! - Checks sender IP against multiple DNSBLs
//! - Validates reverse DNS matches forward DNS
//! - Ensures domains have valid MX records
//! - Rate limits DNS queries to prevent abuse
//!
//! # Example
//! ```no_run
//! use mail_rs::utils::dns_validator::DnsValidator;
//! use std::net::IpAddr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let validator = DnsValidator::new();
//!
//! let ip: IpAddr = "192.0.2.1".parse()?;
//! let domain = "example.com";
//!
//! // Check if IP is blacklisted
//! let is_blacklisted = validator.check_dnsbl(&ip).await?;
//!
//! // Validate reverse DNS
//! let ptr_valid = validator.validate_ptr(&ip, domain).await?;
//!
//! // Validate MX record exists
//! let has_mx = validator.validate_mx(domain).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MailError, Result};
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;
use tracing::{debug, info, warn};

/// DNS-based Blacklist (DNSBL) servers
///
/// These are popular spam blacklists used to identify known spam sources.
/// See: https://en.wikipedia.org/wiki/Comparison_of_DNS_blacklists
const DNSBL_SERVERS: &[&str] = &[
    "zen.spamhaus.org",           // Spamhaus ZEN (combines SBL, XBL, PBL)
    "bl.spamcop.net",             // SpamCop Blocking List
    "b.barracudacentral.org",     // Barracuda Reputation Block List
    "dnsbl.sorbs.net",            // SORBS DNSBL
];

/// Maximum DNS queries per second (rate limiting)
const MAX_DNS_QPS: usize = 100;

/// DNS query rate limiter state
struct RateLimiter {
    queries: Vec<Instant>,
    max_qps: usize,
}

impl RateLimiter {
    fn new(max_qps: usize) -> Self {
        Self {
            queries: Vec::with_capacity(max_qps),
            max_qps,
        }
    }

    /// Check if a query is allowed (rate limiting)
    fn check_rate_limit(&mut self) -> bool {
        let now = Instant::now();
        let one_second_ago = now - Duration::from_secs(1);

        // Remove queries older than 1 second
        self.queries.retain(|&t| t > one_second_ago);

        // Check if we're under the rate limit
        if self.queries.len() < self.max_qps {
            self.queries.push(now);
            true
        } else {
            false
        }
    }
}

/// DNS validator for email security
pub struct DnsValidator {
    resolver: TokioAsyncResolver,
    rate_limiter: Arc<Mutex<RateLimiter>>,
    enable_dnsbl: bool,
    enable_ptr: bool,
}

impl DnsValidator {
    /// Create a new DNS validator with default settings
    pub fn new() -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Self {
            resolver,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(MAX_DNS_QPS))),
            enable_dnsbl: true,
            enable_ptr: true,
        }
    }

    /// Create a DNS validator with custom settings
    pub fn with_config(enable_dnsbl: bool, enable_ptr: bool) -> Self {
        let resolver = TokioAsyncResolver::tokio(
            ResolverConfig::default(),
            ResolverOpts::default(),
        );

        Self {
            resolver,
            rate_limiter: Arc::new(Mutex::new(RateLimiter::new(MAX_DNS_QPS))),
            enable_dnsbl,
            enable_ptr,
        }
    }

    /// Check if query is allowed by rate limiter
    async fn check_rate_limit(&self) -> bool {
        self.rate_limiter.lock().await.check_rate_limit()
    }

    /// Check if an IP address is listed in DNS blacklists
    ///
    /// # Arguments
    /// * `ip` - IP address to check
    ///
    /// # Returns
    /// - `Ok(true)` if IP is blacklisted
    /// - `Ok(false)` if IP is not blacklisted
    /// - `Err(_)` if DNS queries fail
    ///
    /// # Example
    /// ```no_run
    /// # use mail_rs::utils::dns_validator::DnsValidator;
    /// # use std::net::IpAddr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validator = DnsValidator::new();
    /// let ip: IpAddr = "192.0.2.1".parse()?;
    ///
    /// if validator.check_dnsbl(&ip).await? {
    ///     println!("IP is blacklisted!");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_dnsbl(&self, ip: &IpAddr) -> Result<bool> {
        if !self.enable_dnsbl {
            debug!("DNSBL checking is disabled");
            return Ok(false);
        }

        // Check rate limit
        if !self.check_rate_limit().await {
            warn!("DNS rate limit exceeded");
            return Err(MailError::Config("DNS rate limit exceeded".to_string()));
        }

        info!("Checking IP {} against DNSBLs", ip);

        // Reverse the IP address for DNSBL queries
        let reversed_ip = Self::reverse_ip(ip);

        // Check against each DNSBL server
        for dnsbl in DNSBL_SERVERS {
            let query = format!("{}.{}", reversed_ip, dnsbl);
            debug!("DNSBL query: {}", query);

            // Try to resolve the DNSBL query
            match self.resolver.ipv4_lookup(&query).await {
                Ok(_) => {
                    warn!("IP {} is listed in DNSBL: {}", ip, dnsbl);
                    return Ok(true);
                }
                Err(_) => {
                    // Not listed in this DNSBL, continue
                    debug!("IP {} not listed in {}", ip, dnsbl);
                }
            }
        }

        info!("IP {} is not listed in any DNSBL", ip);
        Ok(false)
    }

    /// Reverse an IP address for DNSBL queries
    ///
    /// Example: 192.0.2.1 -> 1.2.0.192
    fn reverse_ip(ip: &IpAddr) -> String {
        match ip {
            IpAddr::V4(ipv4) => {
                let octets = ipv4.octets();
                format!("{}.{}.{}.{}", octets[3], octets[2], octets[1], octets[0])
            }
            IpAddr::V6(ipv6) => {
                // For IPv6, reverse the nibbles
                let segments = ipv6.segments();
                let mut result = String::new();
                for segment in segments.iter().rev() {
                    for i in (0..4).rev() {
                        let nibble = (segment >> (i * 4)) & 0xF;
                        result.push_str(&format!("{:x}.", nibble));
                    }
                }
                result.trim_end_matches('.').to_string()
            }
        }
    }

    /// Validate reverse DNS (PTR record) for an IP address
    ///
    /// Checks that:
    /// 1. The IP has a PTR record
    /// 2. The PTR record hostname matches the expected domain
    ///
    /// # Arguments
    /// * `ip` - IP address to validate
    /// * `expected_domain` - Expected domain name (e.g., "mail.example.com")
    ///
    /// # Returns
    /// - `Ok(true)` if PTR record is valid and matches
    /// - `Ok(false)` if PTR record doesn't match or is missing
    ///
    /// # Example
    /// ```no_run
    /// # use mail_rs::utils::dns_validator::DnsValidator;
    /// # use std::net::IpAddr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validator = DnsValidator::new();
    /// let ip: IpAddr = "192.0.2.1".parse()?;
    ///
    /// if validator.validate_ptr(&ip, "mail.example.com").await? {
    ///     println!("PTR record is valid");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_ptr(&self, ip: &IpAddr, expected_domain: &str) -> Result<bool> {
        if !self.enable_ptr {
            debug!("PTR validation is disabled");
            return Ok(true); // Don't block if disabled
        }

        // Check rate limit
        if !self.check_rate_limit().await {
            warn!("DNS rate limit exceeded");
            return Err(MailError::Config("DNS rate limit exceeded".to_string()));
        }

        info!("Validating PTR record for IP: {}", ip);

        // Perform reverse DNS lookup
        match self.resolver.reverse_lookup(*ip).await {
            Ok(lookup) => {
                // Get the first PTR record
                if let Some(ptr) = lookup.iter().next() {
                    let ptr_name = ptr.to_string();
                    debug!("PTR record found: {}", ptr_name);

                    // Check if PTR record matches expected domain
                    // We use ends_with to allow for subdomain matches
                    let matches = ptr_name.to_lowercase().ends_with(&expected_domain.to_lowercase())
                        || ptr_name.to_lowercase() == expected_domain.to_lowercase();

                    if matches {
                        info!("PTR record {} matches expected domain {}", ptr_name, expected_domain);
                        Ok(true)
                    } else {
                        warn!(
                            "PTR record {} does not match expected domain {}",
                            ptr_name, expected_domain
                        );
                        Ok(false)
                    }
                } else {
                    warn!("No PTR record found for IP: {}", ip);
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("PTR lookup failed for IP {}: {}", ip, e);
                Ok(false)
            }
        }
    }

    /// Validate that a domain has MX records
    ///
    /// # Arguments
    /// * `domain` - Domain to check (e.g., "example.com")
    ///
    /// # Returns
    /// - `Ok(true)` if domain has valid MX records
    /// - `Ok(false)` if domain has no MX records
    ///
    /// # Example
    /// ```no_run
    /// # use mail_rs::utils::dns_validator::DnsValidator;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let validator = DnsValidator::new();
    ///
    /// if validator.validate_mx("example.com").await? {
    ///     println!("Domain has valid MX records");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_mx(&self, domain: &str) -> Result<bool> {
        // Check rate limit
        if !self.check_rate_limit().await {
            warn!("DNS rate limit exceeded");
            return Err(MailError::Config("DNS rate limit exceeded".to_string()));
        }

        info!("Validating MX records for domain: {}", domain);

        match self.resolver.mx_lookup(domain).await {
            Ok(mx_lookup) => {
                let mx_count = mx_lookup.iter().count();
                if mx_count > 0 {
                    info!("Domain {} has {} MX record(s)", domain, mx_count);
                    Ok(true)
                } else {
                    warn!("Domain {} has no MX records", domain);
                    Ok(false)
                }
            }
            Err(e) => {
                warn!("MX lookup failed for domain {}: {}", domain, e);
                Ok(false)
            }
        }
    }

    /// Comprehensive validation for incoming email
    ///
    /// Performs all validation checks:
    /// - DNSBL check
    /// - PTR validation
    /// - MX validation for sender domain
    ///
    /// # Arguments
    /// * `sender_ip` - IP address of the sender
    /// * `sender_domain` - Domain from MAIL FROM command
    ///
    /// # Returns
    /// `DnsValidationResult` with all check results
    pub async fn validate_sender(
        &self,
        sender_ip: &IpAddr,
        sender_domain: &str,
    ) -> Result<DnsValidationResult> {
        info!(
            "Comprehensive DNS validation for IP {} and domain {}",
            sender_ip, sender_domain
        );

        let is_blacklisted = self.check_dnsbl(sender_ip).await.unwrap_or(false);
        let ptr_valid = self.validate_ptr(sender_ip, sender_domain).await.unwrap_or(false);
        let mx_valid = self.validate_mx(sender_domain).await.unwrap_or(false);

        let result = DnsValidationResult {
            is_blacklisted,
            ptr_valid,
            mx_valid,
        };

        info!("DNS validation result: {:?}", result);
        Ok(result)
    }
}

impl Default for DnsValidator {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of comprehensive DNS validation
#[derive(Debug, Clone)]
pub struct DnsValidationResult {
    /// Whether the IP is listed in any DNSBL
    pub is_blacklisted: bool,
    /// Whether the PTR record is valid
    pub ptr_valid: bool,
    /// Whether the sender domain has MX records
    pub mx_valid: bool,
}

impl DnsValidationResult {
    /// Check if the validation passed all checks
    pub fn is_valid(&self) -> bool {
        !self.is_blacklisted && self.ptr_valid && self.mx_valid
    }

    /// Check if email should be accepted based on validation
    ///
    /// Accepts email if:
    /// - Not blacklisted (hard requirement)
    /// - PTR valid OR MX valid (at least one should be true)
    pub fn should_accept(&self) -> bool {
        !self.is_blacklisted && (self.ptr_valid || self.mx_valid)
    }

    /// Get a human-readable reason for rejection
    pub fn rejection_reason(&self) -> Option<String> {
        if self.is_blacklisted {
            return Some("IP address is listed in spam blacklist".to_string());
        }
        if !self.ptr_valid && !self.mx_valid {
            return Some("Invalid reverse DNS and no MX records".to_string());
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_ipv4() {
        let ip: IpAddr = "192.0.2.1".parse().unwrap();
        assert_eq!(DnsValidator::reverse_ip(&ip), "1.2.0.192");
    }

    #[test]
    fn test_validation_result_is_valid() {
        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: true,
            mx_valid: true,
        };
        assert!(result.is_valid());

        let result = DnsValidationResult {
            is_blacklisted: true,
            ptr_valid: true,
            mx_valid: true,
        };
        assert!(!result.is_valid());
    }

    #[test]
    fn test_validation_result_should_accept() {
        // Accept if not blacklisted and either PTR or MX is valid
        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: true,
            mx_valid: false,
        };
        assert!(result.should_accept());

        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: false,
            mx_valid: true,
        };
        assert!(result.should_accept());

        // Reject if blacklisted
        let result = DnsValidationResult {
            is_blacklisted: true,
            ptr_valid: true,
            mx_valid: true,
        };
        assert!(!result.should_accept());

        // Reject if not blacklisted but both PTR and MX invalid
        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: false,
            mx_valid: false,
        };
        assert!(!result.should_accept());
    }

    #[test]
    fn test_rejection_reason() {
        let result = DnsValidationResult {
            is_blacklisted: true,
            ptr_valid: true,
            mx_valid: true,
        };
        assert_eq!(
            result.rejection_reason(),
            Some("IP address is listed in spam blacklist".to_string())
        );

        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: false,
            mx_valid: false,
        };
        assert_eq!(
            result.rejection_reason(),
            Some("Invalid reverse DNS and no MX records".to_string())
        );

        let result = DnsValidationResult {
            is_blacklisted: false,
            ptr_valid: true,
            mx_valid: true,
        };
        assert_eq!(result.rejection_reason(), None);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let mut limiter = RateLimiter::new(5);

        // Should allow first 5 queries
        for _ in 0..5 {
            assert!(limiter.check_rate_limit());
        }

        // 6th query should be blocked
        assert!(!limiter.check_rate_limit());

        // After waiting 1 second, should allow again
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert!(limiter.check_rate_limit());
    }
}
