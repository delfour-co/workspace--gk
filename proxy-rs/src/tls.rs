//! TLS configuration and certificate management
//!
//! Supports both static certificates and automatic ACME (Let's Encrypt).

use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;
use tracing::{info, warn};

use crate::config::TlsConfig;
use crate::error::{ProxyError, Result};

/// TLS manager for handling certificates
pub struct TlsManager {
    config: TlsConfig,
}

impl TlsManager {
    /// Create a new TLS manager
    pub fn new(config: TlsConfig) -> Self {
        Self { config }
    }

    /// Build a TLS acceptor from configuration
    pub fn build_acceptor(&self) -> Result<TlsAcceptor> {
        let server_config = self.build_server_config()?;
        Ok(TlsAcceptor::from(Arc::new(server_config)))
    }

    /// Build rustls server config
    fn build_server_config(&self) -> Result<ServerConfig> {
        // Check if we have static certificate files
        if let (Some(cert_path), Some(key_path)) = (&self.config.cert_path, &self.config.key_path) {
            info!("Loading TLS certificate from {} and {}", cert_path, key_path);
            return self.load_static_certs(Path::new(cert_path), Path::new(key_path));
        }

        // Check if we should use ACME
        if self.config.acme_email.is_some() {
            warn!("ACME support is not yet implemented, using self-signed certificate");
            return self.generate_self_signed();
        }

        // Fall back to self-signed for development
        info!("No TLS configuration provided, generating self-signed certificate");
        self.generate_self_signed()
    }

    /// Load certificates from files
    fn load_static_certs(&self, cert_path: &Path, key_path: &Path) -> Result<ServerConfig> {
        // Load certificates
        let cert_file = File::open(cert_path).map_err(|e| {
            ProxyError::Tls(format!("Failed to open certificate file: {}", e))
        })?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs_der = certs(&mut cert_reader)
            .map_err(|e| ProxyError::Tls(format!("Failed to read certificates: {}", e)))?;

        if certs_der.is_empty() {
            return Err(ProxyError::Tls("No certificates found in file".to_string()));
        }

        // Load private key
        let key_file = File::open(key_path).map_err(|e| {
            ProxyError::Tls(format!("Failed to open key file: {}", e))
        })?;
        let mut key_reader = BufReader::new(key_file);

        // Try PKCS8 first, then RSA
        let keys = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| ProxyError::Tls(format!("Failed to read PKCS8 keys: {}", e)))?;

        let key = if !keys.is_empty() {
            rustls::PrivateKey(keys[0].clone())
        } else {
            // Reset reader and try RSA
            let key_file = File::open(key_path).map_err(|e| {
                ProxyError::Tls(format!("Failed to open key file: {}", e))
            })?;
            let mut key_reader = BufReader::new(key_file);
            let rsa_keys = rsa_private_keys(&mut key_reader)
                .map_err(|e| ProxyError::Tls(format!("Failed to read RSA keys: {}", e)))?;

            if rsa_keys.is_empty() {
                return Err(ProxyError::Tls("No private key found in file".to_string()));
            }
            rustls::PrivateKey(rsa_keys[0].clone())
        };

        // Convert certificates to rustls format
        let certs: Vec<rustls::Certificate> = certs_der
            .into_iter()
            .map(|c| rustls::Certificate(c.to_vec()))
            .collect();

        // Build server config
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| ProxyError::Tls(format!("TLS config error: {}", e)))?;

        Ok(config)
    }

    /// Generate a self-signed certificate for development
    fn generate_self_signed(&self) -> Result<ServerConfig> {
        // Generate a simple self-signed certificate using rcgen
        let subject_alt_names: Vec<String> = if self.config.domains.is_empty() {
            vec!["localhost".to_string()]
        } else {
            self.config.domains.clone()
        };

        // For now, create a minimal self-signed cert
        // In production, this should use rcgen properly
        let cert = rcgen::generate_simple_self_signed(subject_alt_names)
            .map_err(|e| ProxyError::Tls(format!("Failed to generate self-signed cert: {}", e)))?;

        let cert_der = cert.cert.der().to_vec();
        let key_der = cert.key_pair.serialize_der();

        let certs = vec![rustls::Certificate(cert_der)];
        let key = rustls::PrivateKey(key_der);

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| ProxyError::Tls(format!("TLS config error: {}", e)))?;

        Ok(config)
    }
}

/// ACME client for Let's Encrypt certificates
pub struct AcmeClient {
    email: String,
    directory_url: String,
    domains: Vec<String>,
}

impl AcmeClient {
    /// Create a new ACME client
    pub fn new(email: String, directory_url: String, domains: Vec<String>) -> Self {
        Self {
            email,
            directory_url,
            domains,
        }
    }

    /// Request a new certificate from Let's Encrypt
    ///
    /// This is a placeholder for future ACME implementation.
    /// Full implementation would:
    /// 1. Create account with ACME provider
    /// 2. Request certificate for domains
    /// 3. Complete HTTP-01 or DNS-01 challenge
    /// 4. Download and store certificate
    pub async fn request_certificate(&self) -> Result<(Vec<u8>, Vec<u8>)> {
        // TODO: Implement actual ACME flow
        // This requires:
        // - HTTP challenge server on port 80
        // - Account key management
        // - Certificate storage

        warn!("ACME certificate request not yet implemented");
        warn!("Email: {}, Directory: {}", self.email, self.directory_url);
        warn!("Domains: {:?}", self.domains);

        Err(ProxyError::Tls("ACME not yet implemented".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_signed_generation() {
        let config = TlsConfig {
            cert_path: None,
            key_path: None,
            acme_email: None,
            acme_directory: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
            domains: vec!["localhost".to_string()],
        };

        let manager = TlsManager::new(config);
        let result = manager.generate_self_signed();
        assert!(result.is_ok());
    }

    #[test]
    fn test_tls_manager_build_acceptor() {
        let config = TlsConfig {
            cert_path: None,
            key_path: None,
            acme_email: None,
            acme_directory: "https://acme-v02.api.letsencrypt.org/directory".to_string(),
            domains: vec!["localhost".to_string()],
        };

        let manager = TlsManager::new(config);
        let acceptor = manager.build_acceptor();
        assert!(acceptor.is_ok());
    }
}
