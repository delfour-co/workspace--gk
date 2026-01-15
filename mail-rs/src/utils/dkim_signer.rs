//! DKIM (DomainKeys Identified Mail) Signing
//!
//! This module implements DKIM signature generation according to RFC 6376.
//!
//! DKIM allows senders to sign outgoing emails with their domain's private key.
//! Recipients can verify the signature using the public key published in DNS.
//!
//! # Example
//! ```no_run
//! use mail_rs::utils::dkim_signer::DkimSigner;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let private_key_pem = std::fs::read_to_string("private_key.pem")?;
//! let signer = DkimSigner::new(
//!     "example.com",
//!     "default",
//!     &private_key_pem
//! )?;
//!
//! let email = "From: sender@example.com\r\n\r\nHello World";
//! let signed_email = signer.sign_email(email)?;
//! # Ok(())
//! # }
//! ```

use crate::error::{MailError, Result};
use rsa::{RsaPrivateKey, pkcs1::DecodeRsaPrivateKey, pkcs8::DecodePrivateKey};
use sha2::{Sha256, Digest};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use tracing::{debug, info};

/// DKIM signer for outgoing emails
pub struct DkimSigner {
    /// Domain to sign for (d= parameter)
    domain: String,
    /// Selector (s= parameter)
    selector: String,
    /// RSA private key
    private_key: RsaPrivateKey,
}

impl DkimSigner {
    /// Create a new DKIM signer
    ///
    /// # Arguments
    /// * `domain` - Domain to sign for (e.g., "example.com")
    /// * `selector` - Selector for DNS lookup (e.g., "default")
    /// * `private_key_pem` - RSA private key in PEM format
    ///
    /// # Returns
    /// DkimSigner instance or error
    pub fn new(domain: &str, selector: &str, private_key_pem: &str) -> Result<Self> {
        // Try PKCS#8 format first, then PKCS#1
        let private_key = RsaPrivateKey::from_pkcs8_pem(private_key_pem)
            .or_else(|_| RsaPrivateKey::from_pkcs1_pem(private_key_pem))
            .map_err(|e| MailError::Config(format!("Failed to parse DKIM private key: {}", e)))?;

        info!("DKIM signer initialized for domain: {}, selector: {}", domain, selector);

        Ok(Self {
            domain: domain.to_string(),
            selector: selector.to_string(),
            private_key,
        })
    }

    /// Sign an email with DKIM
    ///
    /// # Arguments
    /// * `email_data` - Complete email (headers + body)
    ///
    /// # Returns
    /// Email with DKIM-Signature header added
    pub fn sign_email(&self, email_data: &str) -> Result<String> {
        let (headers, body) = self.split_email(email_data);

        // Canonicalize headers and body (simple canonicalization)
        let canonicalized_headers = self.canonicalize_headers_simple(&headers);
        let canonicalized_body = self.canonicalize_body_simple(&body);

        // Compute body hash
        let body_hash = self.compute_body_hash(&canonicalized_body);

        // Select headers to sign (recommended headers)
        let headers_to_sign = vec!["from", "to", "subject", "date", "message-id"];
        let signed_headers = self.extract_signed_headers(&canonicalized_headers, &headers_to_sign);

        // Build DKIM signature header (without signature yet)
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let dkim_header_template = format!(
            "v=1; a=rsa-sha256; c=simple/simple; d={}; s={}; t={}; bh={}; h={}; b=",
            self.domain,
            self.selector,
            timestamp,
            body_hash,
            headers_to_sign.join(":")
        );

        // Compute signature data (headers + DKIM header template)
        let signature_data = format!("{}\r\nDKIM-Signature: {}", signed_headers.trim_end(), dkim_header_template);

        // Sign the data
        let signature_bytes = self.sign_data(signature_data.as_bytes())?;
        let signature_b64 = BASE64.encode(&signature_bytes);

        // Build complete DKIM header with signature
        let dkim_header = format!("DKIM-Signature: {}{}",
            dkim_header_template,
            signature_b64
        );

        // Add DKIM-Signature as first header
        let signed_email = format!("{}\r\n{}\r\n\r\n{}", dkim_header, headers.trim(), body);

        debug!("Email signed with DKIM");
        Ok(signed_email)
    }

    /// Split email into headers and body
    fn split_email(&self, email_data: &str) -> (String, String) {
        if let Some(pos) = email_data.find("\r\n\r\n") {
            let headers = &email_data[..pos];
            let body = &email_data[pos + 4..];
            (headers.to_string(), body.to_string())
        } else if let Some(pos) = email_data.find("\n\n") {
            let headers = &email_data[..pos];
            let body = &email_data[pos + 2..];
            (headers.to_string(), body.to_string())
        } else {
            // No body separator found
            (email_data.to_string(), String::new())
        }
    }

    /// Canonicalize headers (simple algorithm)
    fn canonicalize_headers_simple(&self, headers: &str) -> String {
        // Simple canonicalization: just ensure CRLF line endings
        headers.replace("\n", "\r\n")
    }

    /// Canonicalize body (simple algorithm)
    fn canonicalize_body_simple(&self, body: &str) -> String {
        // Simple canonicalization:
        // 1. Convert line endings to CRLF
        // 2. Remove trailing empty lines
        // 3. If empty, use single CRLF
        let mut canonical = body.replace("\n", "\r\n");

        // Remove trailing whitespace from each line and trailing empty lines
        let lines: Vec<&str> = canonical.lines().collect();
        let mut result = Vec::new();

        for line in lines {
            result.push(line.trim_end());
        }

        // Remove trailing empty lines
        while result.last().map_or(false, |l| l.is_empty()) {
            result.pop();
        }

        if result.is_empty() {
            "\r\n".to_string()
        } else {
            format!("{}\r\n", result.join("\r\n"))
        }
    }

    /// Compute body hash (bh= parameter)
    fn compute_body_hash(&self, body: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(body.as_bytes());
        let hash = hasher.finalize();
        BASE64.encode(&hash)
    }

    /// Extract headers that will be signed
    fn extract_signed_headers(&self, all_headers: &str, headers_to_sign: &[&str]) -> String {
        let mut result = String::new();

        for header_name in headers_to_sign {
            // Find all occurrences of this header (case-insensitive)
            for line in all_headers.lines() {
                if line.to_lowercase().starts_with(&format!("{}:", header_name)) {
                    result.push_str(line);
                    result.push_str("\r\n");
                }
            }
        }

        result
    }

    /// Sign data with RSA-SHA256
    fn sign_data(&self, data: &[u8]) -> Result<Vec<u8>> {
        use rsa::pkcs1v15::Pkcs1v15Sign;

        // Hash the data with SHA-256
        let mut hasher = Sha256::new();
        hasher.update(data);
        let hashed = hasher.finalize();

        // Sign the hash with PKCS#1 v1.5 padding
        // Use sign_with_rng for RSA signing
        let padding = Pkcs1v15Sign::new_raw();
        let mut rng = rand::thread_rng();
        let signature = self.private_key
            .sign_with_rng(&mut rng, padding, &hashed)
            .map_err(|e| MailError::Config(format!("DKIM signing failed: {}", e)))?;

        Ok(signature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvAIBADANBgkqhkiG9w0BAQEFAASCBKYwggSiAgEAAoIBAQC6qxxXMsY0dFDm
hKi6uQ1HsvElpeHguVLRqmWgyiXIe6VzKkq6DHvdAI0Z9y71+8QGPh9gudp6mL4d
8OuYAp6KWallmOtQLq9cqyy6QfajRKQ8GKMrMvRF3Ptc4ox7TqUYngjXyIaZ89oa
o6x1NEaHLa/KzPHoQbVpwverGSbL8UbUC26dGbBA2XAAYCveg+NgQXyiwSedwy1W
NYLSDpnZwKcIxA8ACYAXL++HXK3CBfzqJS5cydS9lweTi/EHNPnRHcM/PhPDiFeU
ImBU/s2bvZYuSWEX4hgKrnUv4c25a0aX22bDry+ZZLoxomrRxOBoUakYUPi57V7h
uRbohG2JAgMBAAECggEAGlKs0W27M4RqMBzplO3UKJXcREcrHhR5evrWD/bRndQ0
gD6o1lQ9Aezk+OdkJj3q5kfLsZ55ElazCz0rtn46ux8VBdsAO94JeT444B8TLPIu
CBIgAvnRe9SYrFV/d+1Jgg+EXGF4/Fa9x989dN2lSxoEiHWexfYWI07iEVefopyA
MTFKkF8ay8zjdEEKOz2sAUCYz54u4NPDjVTcTefJBqxzGP1S+AZ7tQu1vsbmTq4u
Hs4kY81SzroUOuCBmNSjI2zFnPVpRa5iEOUw45ITMTDEQGeFboKwJBh/UVW4BUFW
6SoG/HdoPT7a+Ta86TL4FdzU9ZWwd16PFZNEHk/IvQKBgQDhNli+TQpDWG3c0bdb
wKt5hleuwjsp35P3TwzCU1k+FKXH/PoOlxYs7lKaGX+Z/cDTUznWtCytxbpAU3C0
qRZpEMz4eZIvSXTXAhjErRuse/xUjbOR6rq0iLHuPTDbyYFNFmmNGDZr1N7yxbne
zXw5olEJTlbeSsGRD2EDY9BmYwKBgQDUL9xgLMdhc6JEc/+3ALdZwAF6LaIFC93t
SJFCDqqHiJDfgTpiW+QWcZOQQHZ9fj94i8Rj0h4LeoLO0FwwnP5LiO0RmdR1UiaR
OFX0+n3TVVQ9/ApebLk26BBlQA7b3EhfsEUN3y/pNcdSZMdixbFjjtM38jxX4cI5
bPfvkxw6IwKBgFCNkMZYZfBao8kU1eGik3K/I4Olg2Xhy1ns02J6T3RODFHw4Tvw
vITLpY3qyuI4igzMOubZwRVSoOC7N5fW1uYzlSzWxvvU6U6yOEu4oypZmpJfsLfj
Xm6Oah4Nkvk6JN5+wrgtzeFZ3Xi2Ulon42P188hBVf7ifEO3XXrJ69ODAoGAPerJ
b/chJzrK+xauZAR9fF8fDzwJr1dLJ1aprJEDChG71LytAM63vCTgXpOw/+W7N73x
TP+fbNIPIbKDdFwbiIcoRp6Q9LtILdA2pQoDdYvMzVmi3PL1CSSmxCTxMQ8Sv3l8
bRPXs0HSLCTkWdo+bULcN9Mj8LEcMA2wO+S9s+kCgYBCVHyxB0z/WEnXEednefz+
Npkc2KUgmPofupIf3bl57xEpe6tV9fh0dDUtbxq8Audes+VeGA58JRaLzMdV0eYz
LTo9YtmhebxJ3AVifHi7dNMTk6Q/ptpWS2QPSORkVo3jrl/p4mSB8v1QKOrjiX3a
VQ13EoBnzsbQpYnA8pUoDQ==
-----END PRIVATE KEY-----"#;

    #[test]
    fn test_dkim_signer_creation() {
        let signer = DkimSigner::new("example.com", "default", TEST_PRIVATE_KEY);
        assert!(signer.is_ok());
    }

    #[test]
    fn test_split_email() {
        let signer = DkimSigner::new("example.com", "default", TEST_PRIVATE_KEY).unwrap();

        let email = "From: sender@example.com\r\nSubject: Test\r\n\r\nHello World";
        let (headers, body) = signer.split_email(email);

        assert_eq!(headers, "From: sender@example.com\r\nSubject: Test");
        assert_eq!(body, "Hello World");
    }

    #[test]
    fn test_canonicalize_body_simple() {
        let signer = DkimSigner::new("example.com", "default", TEST_PRIVATE_KEY).unwrap();

        let body = "Hello World\n\n\n";
        let canonical = signer.canonicalize_body_simple(body);

        // Should remove trailing empty lines
        assert_eq!(canonical, "Hello World\r\n");
    }

    #[test]
    fn test_compute_body_hash() {
        let signer = DkimSigner::new("example.com", "default", TEST_PRIVATE_KEY).unwrap();

        let body = "Hello World\r\n";
        let hash = signer.compute_body_hash(body);

        // Hash should be base64 encoded SHA-256
        assert!(!hash.is_empty());
        assert!(BASE64.decode(&hash).is_ok());
    }

    #[test]
    fn test_sign_email() {
        let signer = DkimSigner::new("example.com", "default", TEST_PRIVATE_KEY).unwrap();

        let email = "From: sender@example.com\r\nTo: recipient@example.org\r\nSubject: Test\r\nDate: Mon, 25 Nov 2024 12:00:00 +0000\r\n\r\nHello World";
        let signed = signer.sign_email(email);

        assert!(signed.is_ok());
        let signed_email = signed.unwrap();

        // Should contain DKIM-Signature header
        assert!(signed_email.contains("DKIM-Signature:"));
        assert!(signed_email.contains("v=1"));
        assert!(signed_email.contains("a=rsa-sha256"));
        assert!(signed_email.contains("d=example.com"));
        assert!(signed_email.contains("s=default"));
        assert!(signed_email.contains("bh="));
        assert!(signed_email.contains("b="));
    }
}
