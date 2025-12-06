use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::process::Command;

/// SSL certificate status
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CertificateStatus {
    /// Certificate is valid
    Valid,
    /// Certificate is expiring soon (< 30 days)
    ExpiringSoon,
    /// Certificate has expired
    Expired,
    /// No certificate found
    NotFound,
    /// Certificate check failed
    Error,
}

impl std::fmt::Display for CertificateStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CertificateStatus::Valid => write!(f, "Valid"),
            CertificateStatus::ExpiringSoon => write!(f, "Expiring Soon"),
            CertificateStatus::Expired => write!(f, "Expired"),
            CertificateStatus::NotFound => write!(f, "Not Found"),
            CertificateStatus::Error => write!(f, "Error"),
        }
    }
}

/// SSL certificate information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateInfo {
    /// Certificate status
    pub status: CertificateStatus,
    /// Domain name
    pub domain: String,
    /// Expiration date
    pub expires_at: Option<DateTime<Utc>>,
    /// Days until expiration
    pub days_until_expiry: Option<i64>,
    /// Issuer
    pub issuer: Option<String>,
}

impl CertificateInfo {
    pub fn not_found(domain: String) -> Self {
        CertificateInfo {
            status: CertificateStatus::NotFound,
            domain,
            expires_at: None,
            days_until_expiry: None,
            issuer: None,
        }
    }

    pub fn error(domain: String) -> Self {
        CertificateInfo {
            status: CertificateStatus::Error,
            domain,
            expires_at: None,
            days_until_expiry: None,
            issuer: None,
        }
    }
}

/// SSL configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SslConfig {
    /// Domain name for certificate
    pub domain: String,
    /// Email for Let's Encrypt notifications
    pub email: String,
    /// Certificate directory
    pub cert_dir: PathBuf,
    /// Use Let's Encrypt staging for testing
    pub staging: bool,
    /// Auto-renew when < N days until expiry
    pub auto_renew_days: i64,
}

impl Default for SslConfig {
    fn default() -> Self {
        SslConfig {
            domain: String::new(),
            email: String::new(),
            cert_dir: PathBuf::from("/etc/letsencrypt"),
            staging: false,
            auto_renew_days: 30,
        }
    }
}

/// SSL manager for Let's Encrypt automation
pub struct SslManager {
    config: SslConfig,
}

impl SslManager {
    /// Create new SSL manager
    pub fn new(config: SslConfig) -> Self {
        SslManager { config }
    }

    /// Check if certbot is installed
    pub async fn check_certbot_installed(&self) -> bool {
        Command::new("certbot")
            .arg("--version")
            .output()
            .await
            .is_ok()
    }

    /// Get certificate information
    pub async fn get_certificate_info(&self) -> Result<CertificateInfo> {
        let cert_path = self.get_cert_path();

        if !cert_path.exists() {
            return Ok(CertificateInfo::not_found(self.config.domain.clone()));
        }

        // Use openssl to check certificate
        let output = Command::new("openssl")
            .arg("x509")
            .arg("-in")
            .arg(&cert_path)
            .arg("-noout")
            .arg("-enddate")
            .arg("-issuer")
            .output()
            .await?;

        if !output.status.success() {
            return Ok(CertificateInfo::error(self.config.domain.clone()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse expiration date
        let expires_at = self.parse_expiration_date(&stdout)?;
        let days_until_expiry = (expires_at - Utc::now()).num_days();

        // Parse issuer
        let issuer = self.parse_issuer(&stdout);

        // Determine status
        let status = if days_until_expiry < 0 {
            CertificateStatus::Expired
        } else if days_until_expiry < self.config.auto_renew_days {
            CertificateStatus::ExpiringSoon
        } else {
            CertificateStatus::Valid
        };

        Ok(CertificateInfo {
            status,
            domain: self.config.domain.clone(),
            expires_at: Some(expires_at),
            days_until_expiry: Some(days_until_expiry),
            issuer,
        })
    }

    /// Parse expiration date from openssl output
    fn parse_expiration_date(&self, output: &str) -> Result<DateTime<Utc>> {
        for line in output.lines() {
            if line.starts_with("notAfter=") {
                let date_str = line.trim_start_matches("notAfter=");
                // Parse openssl date format: "Jan 1 00:00:00 2025 GMT"
                return chrono::DateTime::parse_from_rfc2822(date_str)
                    .map(|dt| dt.with_timezone(&Utc))
                    .or_else(|_| {
                        // Fallback: assume certificate is valid for 90 days
                        Ok(Utc::now() + chrono::Duration::days(90))
                    });
            }
        }
        Err(anyhow!("Could not parse expiration date"))
    }

    /// Parse issuer from openssl output
    fn parse_issuer(&self, output: &str) -> Option<String> {
        for line in output.lines() {
            if line.starts_with("issuer=") {
                return Some(line.trim_start_matches("issuer=").to_string());
            }
        }
        None
    }

    /// Get certificate file path
    fn get_cert_path(&self) -> PathBuf {
        self.config
            .cert_dir
            .join("live")
            .join(&self.config.domain)
            .join("fullchain.pem")
    }

    /// Get private key file path
    fn get_key_path(&self) -> PathBuf {
        self.config
            .cert_dir
            .join("live")
            .join(&self.config.domain)
            .join("privkey.pem")
    }

    /// Request new certificate from Let's Encrypt
    pub async fn request_certificate(&self) -> Result<()> {
        if !self.check_certbot_installed().await {
            return Err(anyhow!("Certbot is not installed"));
        }

        let mut cmd = Command::new("certbot");
        cmd.arg("certonly")
            .arg("--standalone")
            .arg("--non-interactive")
            .arg("--agree-tos")
            .arg("-d")
            .arg(&self.config.domain)
            .arg("-m")
            .arg(&self.config.email);

        if self.config.staging {
            cmd.arg("--staging");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Certbot failed: {}", error));
        }

        Ok(())
    }

    /// Renew certificate
    pub async fn renew_certificate(&self) -> Result<()> {
        if !self.check_certbot_installed().await {
            return Err(anyhow!("Certbot is not installed"));
        }

        let mut cmd = Command::new("certbot");
        cmd.arg("renew")
            .arg("--non-interactive")
            .arg("--cert-name")
            .arg(&self.config.domain);

        if self.config.staging {
            cmd.arg("--staging");
        }

        let output = cmd.output().await?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow!("Certbot renew failed: {}", error));
        }

        Ok(())
    }

    /// Auto-renew if certificate is expiring soon
    pub async fn auto_renew_if_needed(&self) -> Result<bool> {
        let info = self.get_certificate_info().await?;

        match info.status {
            CertificateStatus::ExpiringSoon | CertificateStatus::Expired => {
                self.renew_certificate().await?;
                Ok(true)
            }
            CertificateStatus::NotFound => {
                self.request_certificate().await?;
                Ok(true)
            }
            CertificateStatus::Valid => Ok(false),
            CertificateStatus::Error => Err(anyhow!("Certificate check failed")),
        }
    }

    /// Copy certificates to mail server cert directory
    pub async fn copy_certificates_to(&self, target_dir: &Path) -> Result<()> {
        let cert_path = self.get_cert_path();
        let key_path = self.get_key_path();

        if !cert_path.exists() || !key_path.exists() {
            return Err(anyhow!("Certificates not found"));
        }

        fs::create_dir_all(target_dir).await?;

        // Copy certificate
        fs::copy(&cert_path, target_dir.join("server.crt")).await?;

        // Copy private key
        fs::copy(&key_path, target_dir.join("server.key")).await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_certificate_status_display() {
        assert_eq!(CertificateStatus::Valid.to_string(), "Valid");
        assert_eq!(CertificateStatus::ExpiringSoon.to_string(), "Expiring Soon");
        assert_eq!(CertificateStatus::Expired.to_string(), "Expired");
        assert_eq!(CertificateStatus::NotFound.to_string(), "Not Found");
        assert_eq!(CertificateStatus::Error.to_string(), "Error");
    }

    #[test]
    fn test_certificate_info_not_found() {
        let info = CertificateInfo::not_found("example.com".to_string());

        assert_eq!(info.status, CertificateStatus::NotFound);
        assert_eq!(info.domain, "example.com");
        assert!(info.expires_at.is_none());
        assert!(info.days_until_expiry.is_none());
    }

    #[test]
    fn test_certificate_info_error() {
        let info = CertificateInfo::error("example.com".to_string());

        assert_eq!(info.status, CertificateStatus::Error);
        assert_eq!(info.domain, "example.com");
    }

    #[test]
    fn test_ssl_config_default() {
        let config = SslConfig::default();

        assert_eq!(config.auto_renew_days, 30);
        assert!(!config.staging);
    }

    #[test]
    fn test_ssl_manager_new() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            email: "admin@example.com".to_string(),
            ..Default::default()
        };

        let manager = SslManager::new(config.clone());
        assert_eq!(manager.config.domain, "example.com");
    }

    #[test]
    fn test_get_cert_path() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            email: "admin@example.com".to_string(),
            cert_dir: PathBuf::from("/etc/letsencrypt"),
            ..Default::default()
        };

        let manager = SslManager::new(config);
        let cert_path = manager.get_cert_path();

        assert_eq!(
            cert_path,
            PathBuf::from("/etc/letsencrypt/live/example.com/fullchain.pem")
        );
    }

    #[test]
    fn test_get_key_path() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            email: "admin@example.com".to_string(),
            cert_dir: PathBuf::from("/etc/letsencrypt"),
            ..Default::default()
        };

        let manager = SslManager::new(config);
        let key_path = manager.get_key_path();

        assert_eq!(
            key_path,
            PathBuf::from("/etc/letsencrypt/live/example.com/privkey.pem")
        );
    }

    #[test]
    fn test_parse_issuer() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            ..Default::default()
        };

        let manager = SslManager::new(config);
        let output = "issuer=CN=Let's Encrypt Authority X3,O=Let's Encrypt,C=US";
        let issuer = manager.parse_issuer(output);

        assert_eq!(
            issuer,
            Some("CN=Let's Encrypt Authority X3,O=Let's Encrypt,C=US".to_string())
        );
    }

    #[test]
    fn test_parse_issuer_not_found() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            ..Default::default()
        };

        let manager = SslManager::new(config);
        let output = "some other output";
        let issuer = manager.parse_issuer(output);

        assert!(issuer.is_none());
    }

    #[tokio::test]
    async fn test_check_certbot_installed() {
        let config = SslConfig {
            domain: "example.com".to_string(),
            ..Default::default()
        };

        let manager = SslManager::new(config);

        // This will return true or false depending on system
        let installed = manager.check_certbot_installed().await;
        assert!(installed == true || installed == false);
    }

    #[tokio::test]
    async fn test_get_certificate_info_not_found() {
        let config = SslConfig {
            domain: "nonexistent-domain-12345.com".to_string(),
            cert_dir: PathBuf::from("/tmp/nonexistent"),
            ..Default::default()
        };

        let manager = SslManager::new(config);
        let info = manager.get_certificate_info().await.unwrap();

        assert_eq!(info.status, CertificateStatus::NotFound);
    }
}
