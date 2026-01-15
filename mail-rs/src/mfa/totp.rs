//! TOTP (Time-based One-Time Password) service
//!
//! Implements RFC 6238 for TOTP generation and validation.

use anyhow::{anyhow, Result};
use totp_rs::{Algorithm, Secret, TOTP};

use super::types::MfaSetupResponse;

/// TOTP service configuration
pub struct TotpConfig {
    /// Issuer name (shown in authenticator apps)
    pub issuer: String,
    /// Number of digits in the TOTP code
    pub digits: usize,
    /// Time step in seconds (default: 30)
    pub step: u64,
    /// Algorithm to use
    pub algorithm: Algorithm,
}

impl Default for TotpConfig {
    fn default() -> Self {
        Self {
            issuer: "GK Mail".to_string(),
            digits: 6,
            step: 30,
            algorithm: Algorithm::SHA1,
        }
    }
}

/// TOTP service for generating and validating codes
pub struct TotpService {
    config: TotpConfig,
}

impl TotpService {
    /// Create a new TOTP service with default configuration
    pub fn new() -> Self {
        Self {
            config: TotpConfig::default(),
        }
    }

    /// Create a new TOTP service with custom configuration
    pub fn with_config(config: TotpConfig) -> Self {
        Self { config }
    }

    /// Generate a new secret for a user
    pub fn generate_secret(&self) -> String {
        let secret = Secret::generate_secret();
        secret.to_encoded().to_string()
    }

    /// Create TOTP setup data for a user (secret + QR code)
    pub fn setup(&self, email: &str) -> Result<MfaSetupResponse> {
        let secret_str = self.generate_secret();
        let secret = Secret::Encoded(secret_str.clone());

        let totp = TOTP::new(
            self.config.algorithm,
            self.config.digits,
            1, // Skew (allow 1 step before/after)
            self.config.step,
            secret.to_bytes().map_err(|e| anyhow!("Invalid secret: {}", e))?,
            Some(self.config.issuer.clone()),
            email.to_string(),
        )
        .map_err(|e| anyhow!("Failed to create TOTP: {}", e))?;

        let provisioning_uri = totp.get_url();

        // Generate QR code as data URI
        let qr_code = totp
            .get_qr_base64()
            .map_err(|e| anyhow!("Failed to generate QR code: {}", e))?;

        Ok(MfaSetupResponse {
            secret: secret_str,
            qr_code: format!("data:image/png;base64,{}", qr_code),
            provisioning_uri,
        })
    }

    /// Validate a TOTP code against a secret
    pub fn validate(&self, secret_base32: &str, code: &str) -> Result<bool> {
        let secret = Secret::Encoded(secret_base32.to_string());

        let totp = TOTP::new(
            self.config.algorithm,
            self.config.digits,
            1, // Skew (allow 1 step before/after)
            self.config.step,
            secret.to_bytes().map_err(|e| anyhow!("Invalid secret: {}", e))?,
            Some(self.config.issuer.clone()),
            String::new(), // Account name not needed for validation
        )
        .map_err(|e| anyhow!("Failed to create TOTP: {}", e))?;

        Ok(totp.check_current(code).unwrap_or(false))
    }

    /// Generate the current TOTP code (for testing)
    #[cfg(test)]
    pub fn generate_current(&self, secret_base32: &str) -> Result<String> {
        let secret = Secret::Encoded(secret_base32.to_string());

        let totp = TOTP::new(
            self.config.algorithm,
            self.config.digits,
            1,
            self.config.step,
            secret.to_bytes().map_err(|e| anyhow!("Invalid secret: {}", e))?,
            Some(self.config.issuer.clone()),
            String::new(),
        )
        .map_err(|e| anyhow!("Failed to create TOTP: {}", e))?;

        Ok(totp.generate_current().map_err(|e| anyhow!("Failed to generate code: {}", e))?)
    }
}

impl Default for TotpService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secret() {
        let service = TotpService::new();
        let secret = service.generate_secret();

        // Base32 encoded secret should be non-empty
        assert!(!secret.is_empty());
        // Should be valid base32
        assert!(secret.chars().all(|c| c.is_ascii_alphanumeric() || c == '='));
    }

    #[test]
    fn test_setup_generates_qr() {
        let service = TotpService::new();
        let setup = service.setup("test@example.com").unwrap();

        assert!(!setup.secret.is_empty());
        assert!(setup.qr_code.starts_with("data:image/png;base64,"));
        assert!(setup.provisioning_uri.contains("otpauth://"));
        // Email may be URL-encoded (@ as %40)
        assert!(
            setup.provisioning_uri.contains("test@example.com")
                || setup.provisioning_uri.contains("test%40example.com")
        );
    }

    #[test]
    fn test_validate_correct_code() {
        let service = TotpService::new();
        let secret = service.generate_secret();

        // Generate current code and validate it
        let code = service.generate_current(&secret).unwrap();
        assert!(service.validate(&secret, &code).unwrap());
    }

    #[test]
    fn test_validate_incorrect_code() {
        let service = TotpService::new();
        let secret = service.generate_secret();

        // Invalid code should fail
        assert!(!service.validate(&secret, "000000").unwrap());
        assert!(!service.validate(&secret, "123456").unwrap());
    }

    #[test]
    fn test_validate_wrong_length() {
        let service = TotpService::new();
        let secret = service.generate_secret();

        // Wrong length codes should fail
        assert!(!service.validate(&secret, "12345").unwrap());
        assert!(!service.validate(&secret, "1234567").unwrap());
    }
}
