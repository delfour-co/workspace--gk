use crate::config::Config;
use crate::error::Result;
use crate::security::{Authenticator, TlsConfig};
use crate::smtp::session::SmtpSession;
use crate::storage::MaildirStorage;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

pub struct SmtpServer {
    config: Config,
    storage: Arc<MaildirStorage>,
    tls_config: Option<Arc<TlsConfig>>,
    authenticator: Option<Arc<Authenticator>>,
}

impl SmtpServer {
    pub fn new(config: Config, storage: Arc<MaildirStorage>) -> Self {
        Self {
            config,
            storage,
            tls_config: None,
            authenticator: None,
        }
    }

    /// Create server with TLS and AUTH support
    pub async fn with_security(
        config: Config,
        storage: Arc<MaildirStorage>,
    ) -> Result<Self> {
        // Load TLS config if enabled
        let tls_config = if config.smtp.enable_tls {
            match (&config.smtp.tls_cert_path, &config.smtp.tls_key_path) {
                (Some(cert_path), Some(key_path)) => {
                    info!("Loading TLS configuration");
                    match TlsConfig::from_pem_files(cert_path, key_path) {
                        Ok(tls) => Some(Arc::new(tls)),
                        Err(e) => {
                            warn!("Failed to load TLS config: {}", e);
                            None
                        }
                    }
                }
                _ => {
                    warn!("TLS enabled but certificate paths not configured");
                    None
                }
            }
        } else {
            None
        };

        // Load authenticator if enabled
        let authenticator = if config.smtp.enable_auth {
            match &config.smtp.auth_database_url {
                Some(db_url) => {
                    info!("Initializing SMTP authenticator");
                    match Authenticator::new(db_url).await {
                        Ok(auth) => Some(Arc::new(auth)),
                        Err(e) => {
                            warn!("Failed to initialize authenticator: {}", e);
                            None
                        }
                    }
                }
                None => {
                    warn!("AUTH enabled but database URL not configured");
                    None
                }
            }
        } else {
            None
        };

        Ok(Self {
            config,
            storage,
            tls_config,
            authenticator,
        })
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.smtp.listen_addr).await?;
        info!("SMTP server listening on {}", self.config.smtp.listen_addr);

        // Log security features
        if self.tls_config.is_some() {
            info!("TLS/STARTTLS support enabled");
        }
        if self.authenticator.is_some() {
            info!("SMTP AUTH support enabled (PLAIN, LOGIN)");
            if self.config.smtp.require_auth {
                info!("Authentication is REQUIRED for sending mail");
            }
        }

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New SMTP connection from {}", addr);

                    let session = SmtpSession::with_security(
                        self.config.server.hostname.clone(),
                        self.storage.clone(),
                        self.config.smtp.max_message_size,
                        self.tls_config.clone(),
                        self.authenticator.clone(),
                        self.config.smtp.require_auth,
                    );

                    tokio::spawn(async move {
                        if let Err(e) = session.handle(socket).await {
                            error!("Session error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                }
            }
        }
    }
}
