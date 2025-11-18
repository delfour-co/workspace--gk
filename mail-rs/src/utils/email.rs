use crate::error::{MailError, Result};

/// Basic email validation
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() {
        return Err(MailError::InvalidEmail("Email is empty".to_string()));
    }

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

    if local.is_empty() || domain.is_empty() {
        return Err(MailError::InvalidEmail(
            "Email parts cannot be empty".to_string(),
        ));
    }

    if !domain.contains('.') {
        return Err(MailError::InvalidEmail(
            "Domain must contain a dot".to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_email() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("user.name@example.co.uk").is_ok());
    }

    #[test]
    fn test_invalid_email() {
        assert!(validate_email("").is_err());
        assert!(validate_email("test").is_err());
        assert!(validate_email("test@").is_err());
        assert!(validate_email("@example.com").is_err());
        assert!(validate_email("test@domain").is_err());
    }
}
