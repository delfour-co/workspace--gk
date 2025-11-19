use mail_rs::config::Config;
use mail_rs::smtp::SmtpServer;
use mail_rs::storage::MaildirStorage;
use std::sync::Arc;
use tracing::{info, Level};
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
    info!("  Maildir path: {}", config.storage.maildir_path);
    info!("  Domain: {}", config.server.domain);

    // Initialize storage
    let storage = Arc::new(MaildirStorage::new(config.storage.maildir_path.clone()));

    // Start SMTP server with security features
    let smtp_server = SmtpServer::with_security(config, storage).await?;
    smtp_server.run().await?;

    Ok(())
}
