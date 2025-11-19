//! DNS utilities for mail server operations
//!
//! This module provides DNS lookup functionality, particularly for MX records.
//!
//! # Features
//! - MX record lookup
//! - Priority-based sorting
//! - Fallback to A/AAAA records
//! - Caching (future)

use crate::error::{MailError, Result};
use std::net::SocketAddr;
use tracing::{debug, info, warn};
use trust_dns_resolver::config::*;
use trust_dns_resolver::TokioAsyncResolver;

/// Resolve MX records for a domain and return mail servers in priority order
///
/// # Arguments
/// * `domain` - Domain to lookup (e.g., "example.com")
///
/// # Returns
/// Vec of mail server addresses sorted by priority (lowest first)
///
/// # Examples
/// ```no_run
/// use mail_rs::utils::dns::lookup_mx;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let servers = lookup_mx("gmail.com").await?;
/// println!("Mail servers: {:?}", servers);
/// # Ok(())
/// # }
/// ```
pub async fn lookup_mx(domain: &str) -> Result<Vec<String>> {
    info!("Looking up MX records for {}", domain);

    // Create resolver
    let resolver = TokioAsyncResolver::tokio(
        ResolverConfig::default(),
        ResolverOpts::default(),
    );

    // Lookup MX records
    let mx_lookup = match resolver.mx_lookup(domain).await {
        Ok(lookup) => lookup,
        Err(e) => {
            warn!("MX lookup failed for {}: {}", domain, e);
            // Fallback: try A record with domain directly
            debug!("Falling back to A record lookup for {}", domain);
            return Ok(vec![format!("{}:25", domain)]);
        }
    };

    // Extract and sort MX records by priority
    let mut mx_records: Vec<(u16, String)> = mx_lookup
        .iter()
        .map(|mx| {
            let priority = mx.preference();
            let exchange = mx.exchange().to_string().trim_end_matches('.').to_string();
            (priority, exchange)
        })
        .collect();

    // Sort by priority (lowest first)
    mx_records.sort_by_key(|(priority, _)| *priority);

    debug!("Found {} MX records for {}", mx_records.len(), domain);

    // Convert to server addresses (hostname:port)
    let servers: Vec<String> = mx_records
        .into_iter()
        .map(|(priority, host)| {
            debug!("  MX {} priority {}", host, priority);
            format!("{}:25", host)
        })
        .collect();

    if servers.is_empty() {
        warn!("No MX records found for {}", domain);
        // Ultimate fallback: use domain directly
        Ok(vec![format!("{}:25", domain)])
    } else {
        Ok(servers)
    }
}

/// Resolve a mail server hostname to socket addresses
///
/// This function handles both hostname:port and IP:port formats.
pub async fn resolve_mail_server(server: &str) -> Result<Vec<SocketAddr>> {
    debug!("Resolving mail server: {}", server);

    // Try to parse as socket address first (for IP:port)
    if let Ok(addr) = server.parse::<SocketAddr>() {
        return Ok(vec![addr]);
    }

    // Parse hostname:port format
    let parts: Vec<&str> = server.split(':').collect();
    if parts.len() != 2 {
        return Err(MailError::DnsLookup(format!("Invalid server format: {}", server)));
    }

    let hostname = parts[0];
    let port: u16 = parts[1].parse().map_err(|_| {
        MailError::DnsLookup(format!("Invalid port in: {}", server))
    })?;

    // Create resolver
    let resolver = TokioAsyncResolver::tokio(
        ResolverConfig::default(),
        ResolverOpts::default(),
    );

    // Try to resolve hostname
    let lookup = resolver
        .lookup_ip(hostname)
        .await
        .map_err(|e| MailError::DnsLookup(format!("Failed to resolve {}: {}", hostname, e)))?;

    let addresses: Vec<SocketAddr> = lookup
        .iter()
        .map(|ip| SocketAddr::new(ip, port))
        .collect();

    if addresses.is_empty() {
        return Err(MailError::DnsLookup(format!("No addresses found for {}", hostname)));
    }

    debug!("Resolved {} to {} address(es)", server, addresses.len());
    Ok(addresses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_lookup_mx_gmail() {
        // This test requires internet connection
        let result = lookup_mx("gmail.com").await;
        assert!(result.is_ok());
        let servers = result.unwrap();
        assert!(!servers.is_empty());
    }

    #[tokio::test]
    async fn test_lookup_mx_nonexistent() {
        // Should fallback to using domain directly
        let servers = lookup_mx("nonexistent-domain-12345.com").await.unwrap();
        assert_eq!(servers, vec!["nonexistent-domain-12345.com:25"]);
    }

    #[test]
    fn test_parse_socket_addr() {
        let addr = "127.0.0.1:25".parse::<SocketAddr>();
        assert!(addr.is_ok());
    }
}
