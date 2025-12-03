use mail_rs::api::ApiServer;
use mail_rs::config::Config;
use mail_rs::imap::ImapServer;
use mail_rs::smtp::SmtpServer;
use mail_rs::storage::MaildirStorage;
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)
        .expect("Failed to set tracing subscriber");

    info!("Starting mail-rs server");

    // Load configuration
    let config = if std::path::Path::new("config.toml").exists() {
        Config::from_file("config.toml")?
    } else {
        info!("No config file found, using defaults");
        Config::default()
    };

    info!("Configuration loaded");
    info!("  SMTP listening on: {}", config.smtp.listen_addr);
    info!("  IMAP listening on: {}", config.imap.listen_addr);
    info!("  Maildir path: {}", config.storage.maildir_path);
    info!("  Domain: {}", config.server.domain);

    let config = Arc::new(config);

    // Initialize storage
    let storage = Arc::new(MaildirStorage::new(config.storage.maildir_path.clone()));

    // Start SMTP server in a separate task
    let smtp_config = Arc::clone(&config);
    let smtp_storage = Arc::clone(&storage);
    let smtp_handle = tokio::spawn(async move {
        let smtp_server = match SmtpServer::with_security((*smtp_config).clone(), smtp_storage).await {
            Ok(server) => server,
            Err(e) => {
                error!("Failed to create SMTP server: {}", e);
                return Err(e);
            }
        };

        info!("Starting SMTP server...");
        smtp_server.run().await
    });

    // Start IMAP server in a separate task
    let imap_config = Arc::clone(&config);
    let imap_handle = tokio::spawn(async move {
        let imap_server = ImapServer::new(imap_config);
        info!("Starting IMAP server...");
        imap_server.start().await
    });

    // Start API server in a separate task
    let api_config = Arc::clone(&config);
    let api_handle = tokio::spawn(async move {
        // Create authenticator for API
        let authenticator = match mail_rs::security::Authenticator::new(&api_config.smtp.auth_database_url.as_ref().unwrap_or(&"sqlite://data/users.db".to_string())).await {
            Ok(auth) => auth,
            Err(e) => {
                error!("Failed to create authenticator for API: {}", e);
                return Err(e);
            }
        };

        info!("Starting API server on 0.0.0.0:8080...");
        let api_server = ApiServer::new(
            authenticator,
            "dev-secret-key-change-in-production".to_string(),
            api_config.storage.maildir_path.clone(),
            "0.0.0.0:8080".to_string(),
        );

        api_server.run().await.map_err(Into::into)
    });

    // Wait for any server to exit (or error)
    tokio::select! {
        result = smtp_handle => {
            match result {
                Ok(Ok(())) => info!("SMTP server exited successfully"),
                Ok(Err(e)) => error!("SMTP server error: {}", e),
                Err(e) => error!("SMTP task panic: {}", e),
            }
        }
        result = imap_handle => {
            match result {
                Ok(Ok(())) => info!("IMAP server exited successfully"),
                Ok(Err(e)) => error!("IMAP server error: {}", e),
                Err(e) => error!("IMAP task panic: {}", e),
            }
        }
        result = api_handle => {
            match result {
                Ok(Ok(())) => info!("API server exited successfully"),
                Ok(Err(e)) => error!("API server error: {}", e),
                Err(e) => error!("API task panic: {}", e),
            }
        }
    }

    Ok(())
}
