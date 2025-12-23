//! JWT Authentication for REST API

use axum::http::{header, HeaderMap};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Session cookie name for admin interface
const SESSION_COOKIE: &str = "admin_session";

/// JWT Claims
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (email address)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: u64,
    /// Issued at (Unix timestamp)
    pub iat: u64,
}

/// JWT configuration
pub struct JwtConfig {
    /// Secret key for signing tokens
    secret: String,
    /// Token expiration duration
    expiration: Duration,
}

impl JwtConfig {
    /// Create a new JWT configuration
    pub fn new(secret: String, expiration_hours: u64) -> Self {
        Self {
            secret,
            expiration: Duration::from_secs(expiration_hours * 3600),
        }
    }

    /// Create a new JWT token for a user
    pub fn create_token(&self, email: &str) -> Result<String, jsonwebtoken::errors::Error> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: email.to_string(),
            exp: now + self.expiration.as_secs(),
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
    }

    /// Validate a JWT token and extract claims
    pub fn validate_token(&self, token: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        Self::new("change-me-in-production".to_string(), 24)
    }
}

/// Extract email from session cookie
pub fn get_session_email(headers: &HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?;
    let cookies = cookie_header.to_str().ok()?;

    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE)) {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_validate_token() {
        let config = JwtConfig::new("test-secret".to_string(), 1);

        let token = config.create_token("test@example.com").unwrap();
        assert!(!token.is_empty());

        let claims = config.validate_token(&token).unwrap();
        assert_eq!(claims.sub, "test@example.com");
    }

    #[test]
    fn test_invalid_token() {
        let config = JwtConfig::new("test-secret".to_string(), 1);

        let result = config.validate_token("invalid-token");
        assert!(result.is_err());
    }
}
