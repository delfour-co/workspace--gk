//! TLS/STARTTLS support for SMTP
//!
//! This module provides TLS encryption for SMTP connections.
//!
//! # Features
//! - STARTTLS (upgrade existing connection)
//! - Native TLS (port 465)
//! - Certificate loading
//! - Self-signed certificate generation (development)
//!
//! # Security
//! - TLS 1.2+ only
//! - Strong cipher suites
//! - Certificate validation

use crate::error::{MailError, Result};
use rustls::ServerConfig;
use rustls_pemfile::{certs, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// TLS configuration for SMTP
#[derive(Clone)]
pub struct TlsConfig {
    server_config: Arc<ServerConfig>,
}

impl TlsConfig {
    /// Create TLS config from certificate and key files
    ///
    /// # Arguments
    /// * `cert_path` - Path to PEM certificate file
    /// * `key_path` - Path to PEM private key file
    ///
    /// # Examples
    /// ```no_run
    /// use mail_rs::security::TlsConfig;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let tls_config = TlsConfig::from_pem_files(
    ///     "/etc/mail/cert.pem",
    ///     "/etc/mail/key.pem"
    /// )?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn from_pem_files<P: AsRef<Path>>(cert_path: P, key_path: P) -> Result<Self> {
        info!("Loading TLS certificate from {:?}", cert_path.as_ref());

        // Load certificate
        let cert_file = File::open(cert_path.as_ref()).map_err(|e| {
            MailError::Tls(format!("Failed to open certificate file: {}", e))
        })?;
        let mut cert_reader = BufReader::new(cert_file);

        let certs = certs(&mut cert_reader)
            .map_err(|e| MailError::Tls(format!("Failed to read certificates: {}", e)))?;

        if certs.is_empty() {
            return Err(MailError::Tls("No certificates found in file".to_string()));
        }

        debug!("Loaded {} certificate(s)", certs.len());

        // Load private key
        let key_file = File::open(key_path.as_ref()).map_err(|e| {
            MailError::Tls(format!("Failed to open key file: {}", e))
        })?;
        let mut key_reader = BufReader::new(key_file);

        let mut keys = pkcs8_private_keys(&mut key_reader)
            .map_err(|e| MailError::Tls(format!("Failed to read private keys: {}", e)))?;

        if keys.is_empty() {
            return Err(MailError::Tls("No private key found in file".to_string()));
        }

        let private_key = keys.remove(0);
        debug!("Loaded private key");

        // Create server config (rustls 0.21 API)
        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(
                certs.into_iter().map(rustls::Certificate).collect(),
                rustls::PrivateKey(private_key)
            )
            .map_err(|e| MailError::Tls(format!("Failed to create TLS config: {}", e)))?;

        info!("TLS configuration created successfully");

        Ok(Self {
            server_config: Arc::new(config),
        })
    }

    /// Get the rustls ServerConfig
    pub fn server_config(&self) -> Arc<ServerConfig> {
        self.server_config.clone()
    }

    /// Create a TLS acceptor for STARTTLS
    ///
    /// This creates a tokio_rustls::TlsAcceptor that can upgrade a TcpStream to TLS.
    ///
    /// # Examples
    /// ```no_run
    /// use mail_rs::security::TlsConfig;
    /// use tokio::net::TcpStream;
    ///
    /// # async fn example(stream: TcpStream) -> Result<(), Box<dyn std::error::Error>> {
    /// let tls_config = TlsConfig::from_pem_files("cert.pem", "key.pem")?;
    /// let acceptor = tls_config.acceptor();
    ///
    /// // Upgrade the stream to TLS
    /// let tls_stream = acceptor.accept(stream).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn acceptor(&self) -> tokio_rustls::TlsAcceptor {
        tokio_rustls::TlsAcceptor::from(self.server_config.clone())
    }
}

/// Generate self-signed certificate for development/testing
///
/// **WARNING**: Only use for development! Not secure for production.
///
/// # Examples
/// ```no_run
/// use mail_rs::security::tls::generate_self_signed_cert;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// generate_self_signed_cert("localhost", "dev-cert.pem", "dev-key.pem")?;
/// # Ok(())
/// # }
/// ```
pub fn generate_self_signed_cert(
    domain: &str,
    cert_output: &str,
    key_output: &str,
) -> Result<()> {
    use rcgen::{CertificateParams, DistinguishedName};

    info!("Generating self-signed certificate for {}", domain);

    let mut params = CertificateParams::default();
    params.distinguished_name = DistinguishedName::new();
    params.subject_alt_names = vec![
        rcgen::SanType::DnsName(domain.to_string()),
        rcgen::SanType::DnsName(format!("*.{}", domain)),
    ];

    let cert = rcgen::Certificate::from_params(params)
        .map_err(|e| MailError::Tls(format!("Failed to generate certificate: {}", e)))?;

    // Write certificate
    std::fs::write(cert_output, cert.serialize_pem()
        .map_err(|e| MailError::Tls(format!("Failed to serialize certificate: {}", e)))?)
        .map_err(|e| MailError::Tls(format!("Failed to write certificate: {}", e)))?;

    // Write private key
    std::fs::write(key_output, cert.serialize_private_key_pem())
        .map_err(|e| MailError::Tls(format!("Failed to write private key: {}", e)))?;

    info!(
        "Self-signed certificate generated: {} and {}",
        cert_output, key_output
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_generate_self_signed_cert() {
        let mut cert_file = NamedTempFile::new().unwrap();
        let mut key_file = NamedTempFile::new().unwrap();

        let cert_path = cert_file.path().to_str().unwrap();
        let key_path = key_file.path().to_str().unwrap();

        generate_self_signed_cert("test.local", cert_path, key_path).unwrap();

        // Verify files exist and have content
        let cert_content = std::fs::read_to_string(cert_path).unwrap();
        let key_content = std::fs::read_to_string(key_path).unwrap();

        assert!(cert_content.contains("BEGIN CERTIFICATE"));
        assert!(key_content.contains("BEGIN PRIVATE KEY"));
    }

    #[test]
    fn test_load_tls_config() {
        // Generate test certificate
        let mut cert_file = NamedTempFile::new().unwrap();
        let mut key_file = NamedTempFile::new().unwrap();

        let cert_path = cert_file.path();
        let key_path = key_file.path();

        generate_self_signed_cert(
            "test.local",
            cert_path.to_str().unwrap(),
            key_path.to_str().unwrap(),
        )
        .unwrap();

        // Load TLS config
        let tls_config = TlsConfig::from_pem_files(cert_path, key_path).unwrap();

        // Verify config exists
        assert!(Arc::strong_count(&tls_config.server_config) >= 1);
    }
}
