use crate::error::{MailError, Result};
use std::net::IpAddr;

/// Maximum length for local part (before @)
const MAX_LOCAL_LENGTH: usize = 64;
/// Maximum length for domain part (after @)
const MAX_DOMAIN_LENGTH: usize = 255;
/// Maximum total email length
const MAX_EMAIL_LENGTH: usize = 320;

/// Comprehensive email validation following RFC 5321
///
/// # Security considerations
/// - Prevents injection attacks via length limits
/// - Rejects dangerous characters
/// - Validates domain structure
/// - Prevents null bytes and control characters
///
/// # Examples
/// ```
/// # use mail_rs::utils::validate_email;
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid").is_err());
/// ```
pub fn validate_email(email: &str) -> Result<()> {
    // Check for null bytes (security: prevent injection)
    if email.contains('\0') {
        return Err(MailError::InvalidEmail(
            "Email contains null byte".to_string(),
        ));
    }

    // Check overall length
    if email.is_empty() {
        return Err(MailError::InvalidEmail("Email is empty".to_string()));
    }

    if email.len() > MAX_EMAIL_LENGTH {
        return Err(MailError::InvalidEmail(format!(
            "Email too long (max {} chars)",
            MAX_EMAIL_LENGTH
        )));
    }

    // Check for @
    if !email.contains('@') {
        return Err(MailError::InvalidEmail(
            "Email must contain @".to_string(),
        ));
    }

    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(MailError::InvalidEmail("Invalid email format".to_string()));
    }

    let local = parts[0];
    let domain = parts[1];

    // Validate local part
    validate_local_part(local)?;

    // Validate domain part
    validate_domain_part(domain)?;

    Ok(())
}

/// Validate the local part (before @) of an email address
fn validate_local_part(local: &str) -> Result<()> {
    if local.is_empty() {
        return Err(MailError::InvalidEmail(
            "Local part cannot be empty".to_string(),
        ));
    }

    if local.len() > MAX_LOCAL_LENGTH {
        return Err(MailError::InvalidEmail(format!(
            "Local part too long (max {} chars)",
            MAX_LOCAL_LENGTH
        )));
    }

    // Check for dangerous characters (control chars, spaces at edges)
    if local.starts_with(' ') || local.ends_with(' ') {
        return Err(MailError::InvalidEmail(
            "Local part cannot start or end with space".to_string(),
        ));
    }

    if local.starts_with('.') || local.ends_with('.') {
        return Err(MailError::InvalidEmail(
            "Local part cannot start or end with dot".to_string(),
        ));
    }

    if local.contains("..") {
        return Err(MailError::InvalidEmail(
            "Local part cannot contain consecutive dots".to_string(),
        ));
    }

    // Check for control characters (security)
    for c in local.chars() {
        if c.is_control() {
            return Err(MailError::InvalidEmail(
                "Local part contains control characters".to_string(),
            ));
        }
    }

    Ok(())
}

/// Validate the domain part (after @) of an email address
fn validate_domain_part(domain: &str) -> Result<()> {
    if domain.is_empty() {
        return Err(MailError::InvalidEmail(
            "Domain cannot be empty".to_string(),
        ));
    }

    if domain.len() > MAX_DOMAIN_LENGTH {
        return Err(MailError::InvalidEmail(format!(
            "Domain too long (max {} chars)",
            MAX_DOMAIN_LENGTH
        )));
    }

    // Check if it's an IP address literal [192.168.1.1] or [IPv6:...]
    if domain.starts_with('[') && domain.ends_with(']') {
        return validate_ip_literal(&domain[1..domain.len() - 1]);
    }

    // Validate domain name structure
    if !domain.contains('.') {
        return Err(MailError::InvalidEmail(
            "Domain must contain at least one dot".to_string(),
        ));
    }

    if domain.starts_with('.') || domain.ends_with('.') {
        return Err(MailError::InvalidEmail(
            "Domain cannot start or end with dot".to_string(),
        ));
    }

    if domain.contains("..") {
        return Err(MailError::InvalidEmail(
            "Domain cannot contain consecutive dots".to_string(),
        ));
    }

    // Validate each label
    for label in domain.split('.') {
        validate_domain_label(label)?;
    }

    Ok(())
}

/// Validate a single domain label (between dots)
fn validate_domain_label(label: &str) -> Result<()> {
    if label.is_empty() {
        return Err(MailError::InvalidEmail("Empty domain label".to_string()));
    }

    if label.len() > 63 {
        return Err(MailError::InvalidEmail(
            "Domain label too long (max 63 chars)".to_string(),
        ));
    }

    // Labels must start and end with alphanumeric
    let first = label.chars().next().unwrap();
    let last = label.chars().last().unwrap();

    if !first.is_ascii_alphanumeric() {
        return Err(MailError::InvalidEmail(
            "Domain label must start with alphanumeric character".to_string(),
        ));
    }

    if !last.is_ascii_alphanumeric() {
        return Err(MailError::InvalidEmail(
            "Domain label must end with alphanumeric character".to_string(),
        ));
    }

    // Check for invalid characters
    for c in label.chars() {
        if !c.is_ascii_alphanumeric() && c != '-' {
            return Err(MailError::InvalidEmail(format!(
                "Invalid character '{}' in domain label",
                c
            )));
        }
    }

    Ok(())
}

/// Validate IP address literal
fn validate_ip_literal(ip_str: &str) -> Result<()> {
    // Handle IPv6 prefix
    let ip_str = if let Some(stripped) = ip_str.strip_prefix("IPv6:") {
        stripped
    } else {
        ip_str
    };

    // Try to parse as IP address
    ip_str.parse::<IpAddr>().map_err(|_| {
        MailError::InvalidEmail(format!("Invalid IP address literal: {}", ip_str))
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name@example.co.uk").is_ok());
        assert!(validate_email("user+tag@example.com").is_ok());
        assert!(validate_email("test123@sub.example.com").is_ok());
    }

    #[test]
    fn test_valid_ip_literals() {
        assert!(validate_email("user@[192.168.1.1]").is_ok());
        assert!(validate_email("user@[IPv6:::1]").is_ok());
    }

    #[test]
    fn test_invalid_emails() {
        // Empty or malformed
        assert!(validate_email("").is_err());
        assert!(validate_email("test").is_err());
        assert!(validate_email("test@").is_err());
        assert!(validate_email("@example.com").is_err());

        // Domain issues
        assert!(validate_email("test@domain").is_err());
        assert!(validate_email("test@.domain.com").is_err());
        assert!(validate_email("test@domain..com").is_err());

        // Local part issues
        assert!(validate_email(".test@domain.com").is_err());
        assert!(validate_email("test.@domain.com").is_err());
        assert!(validate_email("te..st@domain.com").is_err());
    }

    #[test]
    fn test_length_limits() {
        // Local part too long
        let long_local = "a".repeat(65);
        assert!(validate_email(&format!("{}@example.com", long_local)).is_err());

        // Domain too long
        let long_domain = "a".repeat(256);
        assert!(validate_email(&format!("test@{}.com", long_domain)).is_err());

        // Total too long
        let total_long = "a".repeat(321);
        assert!(validate_email(&total_long).is_err());
    }

    #[test]
    fn test_security_checks() {
        // Null byte injection
        assert!(validate_email("test\0@example.com").is_err());

        // Control characters
        assert!(validate_email("test\n@example.com").is_err());
        assert!(validate_email("test\r@example.com").is_err());
    }
}
