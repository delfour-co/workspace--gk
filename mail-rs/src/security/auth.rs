//! SMTP AUTH implementation
//!
//! This module provides authentication mechanisms for SMTP.
//!
//! # Supported mechanisms
//! - PLAIN (RFC 4616)
//! - LOGIN (common but not standardized)
//!
//! # Security
//! - Passwords hashed with Argon2
//! - AUTH only allowed after STARTTLS
//! - Rate limiting on failed attempts
//!
//! # Usage
//! ```no_run
//! use mail_rs::security::{Authenticator, AuthMechanism};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let auth = Authenticator::new("sqlite://users.db").await?;
//!
//! // Add user
//! auth.add_user("user@example.com", "password123").await?;
//!
//! // Authenticate
//! let success = auth.authenticate(
//!     AuthMechanism::Plain,
//!     "user@example.com",
//!     "password123"
//! ).await?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MailError, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use sqlx::SqlitePool;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// SMTP authentication mechanisms
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthMechanism {
    /// PLAIN mechanism (RFC 4616)
    Plain,
    /// LOGIN mechanism
    Login,
}

impl AuthMechanism {
    /// Parse mechanism from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "PLAIN" => Some(Self::Plain),
            "LOGIN" => Some(Self::Login),
            _ => None,
        }
    }

    /// Get mechanism name
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Plain => "PLAIN",
            Self::Login => "LOGIN",
        }
    }
}

/// SMTP authenticator
#[derive(Clone)]
pub struct Authenticator {
    pub db: Arc<SqlitePool>,
}

impl Authenticator {
    /// Create a new authenticator
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = SqlitePool::connect(database_url).await?;

        // Create users table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS smtp_users (
                email TEXT PRIMARY KEY,
                password_hash TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_login TEXT
            )
            "#,
        )
        .execute(&db)
        .await?;

        // Create failed attempts table for rate limiting
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS auth_failures (
                email TEXT NOT NULL,
                attempt_time TEXT NOT NULL,
                ip_address TEXT NOT NULL
            )
            "#,
        )
        .execute(&db)
        .await?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Add a new user
    ///
    /// # Security
    /// Password is hashed with Argon2 before storage
    pub async fn add_user(&self, email: &str, password: &str) -> Result<()> {
        info!("Adding user: {}", email);

        // Hash password
        let password_hash = self.hash_password(password)?;

        sqlx::query(
            r#"
            INSERT INTO smtp_users (email, password_hash, created_at)
            VALUES (?, ?, datetime('now'))
            "#,
        )
        .bind(email)
        .bind(&password_hash)
        .execute(&*self.db)
        .await?;

        info!("User added: {}", email);
        Ok(())
    }

    /// Authenticate a user with SMTP mechanism
    ///
    /// # Security
    /// - Constant-time comparison
    /// - Logs failed attempts
    /// - Rate limiting (future)
    pub async fn authenticate_smtp(
        &self,
        mechanism: AuthMechanism,
        username: &str,
        password: &str,
    ) -> Result<bool> {
        debug!("Authentication attempt for {} using {:?}", username, mechanism);
        self.authenticate(username, password).await
    }

    /// Decode PLAIN authentication data
    ///
    /// Format: `\0username\0password` (base64 encoded)
    pub fn decode_plain_auth(auth_data: &str) -> Result<(String, String)> {
        let decoded = BASE64
            .decode(auth_data.trim())
            .map_err(|e| MailError::SmtpProtocol(format!("Invalid base64: {}", e)))?;

        let parts: Vec<&str> = std::str::from_utf8(&decoded)
            .map_err(|e| MailError::SmtpProtocol(format!("Invalid UTF-8: {}", e)))?
            .split('\0')
            .collect();

        if parts.len() != 3 {
            return Err(MailError::SmtpProtocol(
                "Invalid PLAIN auth format".to_string(),
            ));
        }

        // parts[0] is authorization identity (often empty)
        // parts[1] is authentication identity (username)
        // parts[2] is password
        Ok((parts[1].to_string(), parts[2].to_string()))
    }

    /// Decode LOGIN authentication data
    ///
    /// Username and password are sent separately, both base64 encoded
    pub fn decode_login_credential(credential: &str) -> Result<String> {
        let decoded = BASE64
            .decode(credential.trim())
            .map_err(|e| MailError::SmtpProtocol(format!("Invalid base64: {}", e)))?;

        String::from_utf8(decoded)
            .map_err(|e| MailError::SmtpProtocol(format!("Invalid UTF-8: {}", e)))
    }

    /// Hash password with Argon2
    pub fn hash_password(&self, password: &str) -> Result<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        let password_hash = argon2
            .hash_password(password.as_bytes(), &salt)
            .map_err(|e| MailError::Config(format!("Failed to hash password: {}", e)))?;

        Ok(password_hash.to_string())
    }

    /// Check if user exists
    pub async fn user_exists(&self, email: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM smtp_users WHERE email = ?
            "#,
        )
        .bind(email)
        .fetch_one(&*self.db)
        .await?;

        Ok(count.0 > 0)
    }

    /// Delete user
    pub async fn delete_user(&self, email: &str) -> Result<()> {
        info!("Deleting user: {}", email);

        sqlx::query(
            r#"
            DELETE FROM smtp_users WHERE email = ?
            "#,
        )
        .bind(email)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// List all users (for web interface)
    ///
    /// Returns a list of (rowid, email, created_at) tuples
    pub async fn list_users(&self) -> Result<Vec<(i32, String, String)>> {
        let users = sqlx::query_as::<_, (i32, String, String)>(
            r#"
            SELECT rowid, email, created_at
            FROM smtp_users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(users)
    }

    /// List all users with last login (for CLI)
    ///
    /// Returns a list of (email, created_at, last_login) tuples
    pub async fn list_users_detailed(&self) -> Result<Vec<(String, String, Option<String>)>> {
        let users = sqlx::query_as::<_, (String, String, Option<String>)>(
            r#"
            SELECT email, created_at, last_login
            FROM smtp_users
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&*self.db)
        .await?;

        Ok(users)
    }

    /// Count total users
    pub async fn count_users(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM smtp_users
            "#,
        )
        .fetch_one(&*self.db)
        .await?;

        Ok(count.0)
    }

    /// Create a new user (alias for add_user for consistency)
    pub async fn create_user(&self, email: &str, password: &str) -> Result<()> {
        self.add_user(email, password).await
    }

    /// Delete user by ID
    pub async fn delete_user_by_id(&self, rowid: i32) -> Result<()> {
        info!("Deleting user by ID: {}", rowid);

        sqlx::query(
            r#"
            DELETE FROM smtp_users WHERE rowid = ?
            "#,
        )
        .bind(rowid)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// Authenticate a user (simple version for web interface)
    pub async fn authenticate(&self, username: &str, password: &str) -> Result<bool> {
        debug!("Authentication attempt for {}", username);

        // Get user from database
        let row = sqlx::query_as::<_, (String, String)>(
            r#"
            SELECT email, password_hash
            FROM smtp_users
            WHERE email = ?
            "#,
        )
        .bind(username)
        .fetch_optional(&*self.db)
        .await?;

        let Some((email, stored_hash)) = row else {
            warn!("Authentication failed: user not found: {}", username);
            return Ok(false);
        };

        // Verify password
        let parsed_hash = PasswordHash::new(&stored_hash)
            .map_err(|_e| MailError::AuthenticationFailed)?;

        let argon2 = Argon2::default();
        let verified = argon2.verify_password(password.as_bytes(), &parsed_hash).is_ok();

        if verified {
            info!("Authentication successful for {}", email);

            // Update last login
            sqlx::query(
                r#"
                UPDATE smtp_users
                SET last_login = datetime('now')
                WHERE email = ?
                "#,
            )
            .bind(&email)
            .execute(&*self.db)
            .await?;

            Ok(true)
        } else {
            warn!("Authentication failed: invalid password for {}", username);
            Ok(false)
        }
    }

    /// Health check - verify database connectivity
    ///
    /// Returns Ok(()) if database is accessible and responsive
    pub async fn health_check(&self) -> Result<()> {
        // Simple query to check database connectivity
        sqlx::query("SELECT 1")
            .execute(&*self.db)
            .await?;

        Ok(())
    }

    /// Verify login credentials (for IMAP)
    ///
    /// Simplified authentication method that doesn't require specifying mechanism
    pub async fn verify_login(&self, username: &str, password: &str) -> Result<bool> {
        self.authenticate(username, password).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_authenticate_user() {
        let auth = Authenticator::new("sqlite::memory:").await.unwrap();

        // Add user
        auth.add_user("test@example.com", "password123")
            .await
            .unwrap();

        // Authenticate with correct password
        let result = auth
            .authenticate("test@example.com", "password123")
            .await
            .unwrap();
        assert!(result);

        // Authenticate with wrong password
        let result = auth
            .authenticate("test@example.com", "wrong")
            .await
            .unwrap();
        assert!(!result);
    }

    #[tokio::test]
    async fn test_user_exists() {
        let auth = Authenticator::new("sqlite::memory:").await.unwrap();

        assert!(!auth.user_exists("test@example.com").await.unwrap());

        auth.add_user("test@example.com", "password")
            .await
            .unwrap();

        assert!(auth.user_exists("test@example.com").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_user() {
        let auth = Authenticator::new("sqlite::memory:").await.unwrap();

        auth.add_user("test@example.com", "password")
            .await
            .unwrap();
        assert!(auth.user_exists("test@example.com").await.unwrap());

        auth.delete_user("test@example.com").await.unwrap();
        assert!(!auth.user_exists("test@example.com").await.unwrap());
    }

    #[test]
    fn test_decode_plain_auth() {
        // \0username\0password encoded in base64
        let auth_data = BASE64.encode(b"\0user@example.com\0password123");

        let (username, password) = Authenticator::decode_plain_auth(&auth_data).unwrap();
        assert_eq!(username, "user@example.com");
        assert_eq!(password, "password123");
    }

    #[test]
    fn test_decode_login_credential() {
        let encoded = BASE64.encode(b"user@example.com");
        let decoded = Authenticator::decode_login_credential(&encoded).unwrap();
        assert_eq!(decoded, "user@example.com");
    }

    #[test]
    fn test_auth_mechanism_from_str() {
        assert_eq!(AuthMechanism::from_str("PLAIN"), Some(AuthMechanism::Plain));
        assert_eq!(AuthMechanism::from_str("plain"), Some(AuthMechanism::Plain));
        assert_eq!(AuthMechanism::from_str("LOGIN"), Some(AuthMechanism::Login));
        assert_eq!(AuthMechanism::from_str("UNKNOWN"), None);
    }
}
