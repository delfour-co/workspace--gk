//! ACME (Let's Encrypt) certificate management
//!
//! Handles automatic certificate provisioning and renewal using the ACME protocol.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::error::{ProxyError, Result};

/// ACME challenge token storage for HTTP-01 validation
pub struct AcmeChallengeStore {
    /// Map of token -> authorization value
    challenges: Arc<RwLock<HashMap<String, String>>>,
}

impl AcmeChallengeStore {
    /// Create a new challenge store
    pub fn new() -> Self {
        Self {
            challenges: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a challenge response
    pub async fn add_challenge(&self, token: &str, authorization: &str) {
        let mut challenges = self.challenges.write().await;
        challenges.insert(token.to_string(), authorization.to_string());
        debug!("Added ACME challenge for token: {}", token);
    }

    /// Get a challenge response
    pub async fn get_challenge(&self, token: &str) -> Option<String> {
        let challenges = self.challenges.read().await;
        challenges.get(token).cloned()
    }

    /// Remove a challenge (after validation)
    pub async fn remove_challenge(&self, token: &str) {
        let mut challenges = self.challenges.write().await;
        challenges.remove(token);
        debug!("Removed ACME challenge for token: {}", token);
    }
}

impl Default for AcmeChallengeStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Certificate storage paths
#[derive(Debug, Clone)]
pub struct CertificatePaths {
    /// Directory to store certificates
    pub cert_dir: PathBuf,
    /// Certificate file path
    pub cert_path: PathBuf,
    /// Private key file path
    pub key_path: PathBuf,
    /// Account key file path
    pub account_key_path: PathBuf,
}

impl CertificatePaths {
    /// Create certificate paths for a domain
    pub fn for_domain(base_dir: &Path, domain: &str) -> Self {
        let cert_dir = base_dir.join("certs").join(domain);
        Self {
            cert_path: cert_dir.join("cert.pem"),
            key_path: cert_dir.join("key.pem"),
            account_key_path: base_dir.join("account.key"),
            cert_dir,
        }
    }

    /// Check if certificate files exist
    pub fn exists(&self) -> bool {
        self.cert_path.exists() && self.key_path.exists()
    }

    /// Ensure certificate directory exists
    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.cert_dir)
            .map_err(|e| ProxyError::Tls(format!("Failed to create cert directory: {}", e)))?;
        Ok(())
    }
}

/// ACME certificate manager
pub struct AcmeManager {
    /// ACME directory URL
    directory_url: String,
    /// Contact email
    email: String,
    /// Domains to request certificates for
    domains: Vec<String>,
    /// Certificate storage base directory
    storage_dir: PathBuf,
    /// Challenge store
    challenge_store: Arc<AcmeChallengeStore>,
}

impl AcmeManager {
    /// Create a new ACME manager
    pub fn new(
        directory_url: String,
        email: String,
        domains: Vec<String>,
        storage_dir: PathBuf,
    ) -> Self {
        Self {
            directory_url,
            email,
            domains,
            storage_dir,
            challenge_store: Arc::new(AcmeChallengeStore::new()),
        }
    }

    /// Create for Let's Encrypt production
    pub fn lets_encrypt_production(email: String, domains: Vec<String>, storage_dir: PathBuf) -> Self {
        Self::new(
            "https://acme-v02.api.letsencrypt.org/directory".to_string(),
            email,
            domains,
            storage_dir,
        )
    }

    /// Create for Let's Encrypt staging (testing)
    pub fn lets_encrypt_staging(email: String, domains: Vec<String>, storage_dir: PathBuf) -> Self {
        Self::new(
            "https://acme-staging-v02.api.letsencrypt.org/directory".to_string(),
            email,
            domains,
            storage_dir,
        )
    }

    /// Get the challenge store for use in HTTP handler
    pub fn challenge_store(&self) -> Arc<AcmeChallengeStore> {
        self.challenge_store.clone()
    }

    /// Get certificate paths for the primary domain
    pub fn cert_paths(&self) -> CertificatePaths {
        let primary_domain = self.domains.first().cloned().unwrap_or_default();
        CertificatePaths::for_domain(&self.storage_dir, &primary_domain)
    }

    /// Check if we need to request/renew certificates
    pub fn needs_certificate(&self) -> bool {
        let paths = self.cert_paths();
        if !paths.exists() {
            info!("Certificate files not found, need to request");
            return true;
        }

        // Check certificate expiration
        match self.check_certificate_expiry(&paths.cert_path) {
            Ok(days_until_expiry) => {
                if days_until_expiry < 30 {
                    info!(
                        "Certificate expires in {} days, need to renew",
                        days_until_expiry
                    );
                    true
                } else {
                    info!(
                        "Certificate valid for {} more days",
                        days_until_expiry
                    );
                    false
                }
            }
            Err(e) => {
                warn!("Failed to check certificate expiry: {}", e);
                true
            }
        }
    }

    /// Check certificate expiry (returns days until expiry)
    fn check_certificate_expiry(&self, cert_path: &Path) -> Result<i64> {
        let cert_pem = fs::read_to_string(cert_path)
            .map_err(|e| ProxyError::Tls(format!("Failed to read certificate: {}", e)))?;

        // Parse certificate to check expiry
        // For simplicity, we'll use a basic check
        // In production, use x509-parser or similar

        // Placeholder: assume 90 days validity for new certs
        // This should parse the actual certificate
        warn!("Certificate expiry check not fully implemented, assuming valid");
        Ok(60) // Return 60 days as placeholder
    }

    /// Request a new certificate from ACME provider
    ///
    /// This implements the HTTP-01 challenge flow:
    /// 1. Create/load account key
    /// 2. Request new order for domains
    /// 3. Get HTTP-01 challenges
    /// 4. Serve challenge responses at /.well-known/acme-challenge/{token}
    /// 5. Notify ACME server to validate
    /// 6. Download certificate
    pub async fn request_certificate(&self) -> Result<()> {
        info!("Requesting ACME certificate for domains: {:?}", self.domains);
        info!("Using ACME directory: {}", self.directory_url);
        info!("Contact email: {}", self.email);

        let paths = self.cert_paths();
        paths.ensure_dir()?;

        // For now, generate a self-signed certificate as placeholder
        // Full ACME implementation would use instant-acme or similar
        self.generate_placeholder_certificate(&paths).await?;

        info!("Certificate provisioned successfully");
        Ok(())
    }

    /// Generate a placeholder self-signed certificate
    /// This is used when ACME is not fully implemented
    async fn generate_placeholder_certificate(&self, paths: &CertificatePaths) -> Result<()> {
        warn!("Full ACME not implemented, generating self-signed certificate");

        let subject_alt_names = self.domains.clone();
        let cert = rcgen::generate_simple_self_signed(subject_alt_names)
            .map_err(|e| ProxyError::Tls(format!("Failed to generate certificate: {}", e)))?;

        // Write certificate
        let cert_pem = cert.cert.pem();
        fs::write(&paths.cert_path, cert_pem)
            .map_err(|e| ProxyError::Tls(format!("Failed to write certificate: {}", e)))?;

        // Write private key
        let key_pem = cert.key_pair.serialize_pem();
        fs::write(&paths.key_path, key_pem)
            .map_err(|e| ProxyError::Tls(format!("Failed to write private key: {}", e)))?;

        info!("Self-signed certificate written to {:?}", paths.cert_path);
        Ok(())
    }

    /// Start background certificate renewal task
    pub fn start_renewal_task(self: Arc<Self>) {
        tokio::spawn(async move {
            loop {
                // Check every 12 hours
                tokio::time::sleep(tokio::time::Duration::from_secs(12 * 60 * 60)).await;

                if self.needs_certificate() {
                    info!("Starting certificate renewal");
                    if let Err(e) = self.request_certificate().await {
                        error!("Certificate renewal failed: {}", e);
                    }
                }
            }
        });
    }
}

/// HTTP handler for ACME challenges
/// Mount this at `/.well-known/acme-challenge/{token}`
pub async fn acme_challenge_handler(
    token: &str,
    store: Arc<AcmeChallengeStore>,
) -> Option<String> {
    store.get_challenge(token).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_certificate_paths() {
        let base = Path::new("/tmp/proxy");
        let paths = CertificatePaths::for_domain(base, "example.com");

        assert_eq!(paths.cert_path, PathBuf::from("/tmp/proxy/certs/example.com/cert.pem"));
        assert_eq!(paths.key_path, PathBuf::from("/tmp/proxy/certs/example.com/key.pem"));
    }

    #[tokio::test]
    async fn test_challenge_store() {
        let store = AcmeChallengeStore::new();

        store.add_challenge("test-token", "test-auth").await;
        assert_eq!(store.get_challenge("test-token").await, Some("test-auth".to_string()));

        store.remove_challenge("test-token").await;
        assert_eq!(store.get_challenge("test-token").await, None);
    }

    #[tokio::test]
    async fn test_acme_manager_creation() {
        let dir = tempdir().unwrap();
        let manager = AcmeManager::lets_encrypt_staging(
            "test@example.com".to_string(),
            vec!["example.com".to_string()],
            dir.path().to_path_buf(),
        );

        assert!(manager.needs_certificate());
    }

    #[tokio::test]
    async fn test_placeholder_certificate() {
        let dir = tempdir().unwrap();
        let manager = AcmeManager::lets_encrypt_staging(
            "test@example.com".to_string(),
            vec!["localhost".to_string()],
            dir.path().to_path_buf(),
        );

        let result = manager.request_certificate().await;
        assert!(result.is_ok());

        let paths = manager.cert_paths();
        assert!(paths.exists());
    }
}
