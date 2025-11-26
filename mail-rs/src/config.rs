use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Config {
    pub server: ServerConfig,
    pub smtp: SmtpConfig,
    pub imap: ImapConfig,
    pub storage: StorageConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ServerConfig {
    pub domain: String,
    pub hostname: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SmtpConfig {
    pub listen_addr: String,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub require_tls: bool, // Enforce STARTTLS for all connections
    pub enable_auth: bool,
    pub auth_database_url: Option<String>,
    pub require_auth: bool,
    pub max_message_size: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ImapConfig {
    pub listen_addr: String,
    pub enable_tls: bool,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StorageConfig {
    pub maildir_path: String,
    pub database_url: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
}

impl Config {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| crate::error::MailError::Config(e.to_string()))?;

        toml::from_str(&content)
            .map_err(|e| crate::error::MailError::Config(e.to_string()))
    }

    pub fn default() -> Self {
        Self {
            server: ServerConfig {
                domain: "localhost".to_string(),
                hostname: "mail.localhost".to_string(),
            },
            smtp: SmtpConfig {
                listen_addr: "0.0.0.0:2525".to_string(),
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
                require_tls: false,
                enable_auth: false,
                auth_database_url: None,
                require_auth: false,
                max_message_size: 10 * 1024 * 1024, // 10MB
            },
            imap: ImapConfig {
                listen_addr: "0.0.0.0:1993".to_string(),
                enable_tls: false,
                tls_cert_path: None,
                tls_key_path: None,
            },
            storage: StorageConfig {
                maildir_path: "/tmp/maildir".to_string(),
                database_url: "sqlite://mail.db".to_string(),
            },
            logging: LoggingConfig {
                level: "info".to_string(),
                format: "pretty".to_string(),
            },
        }
    }
}
