//! proxy-rs: HTTP Reverse Proxy Server
//!
//! A high-performance reverse proxy for routing HTTP traffic
//! to backend services with automatic Let's Encrypt certificates.

use proxy_rs::acme::AcmeManager;
use proxy_rs::{ProxyConfig, ProxyServer};
use std::path::Path;
use std::sync::Arc;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "proxy_rs=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting proxy-rs v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = if let Some(config_path) = std::env::args().nth(1) {
        info!("Loading configuration from {}", config_path);
        ProxyConfig::from_file(Path::new(&config_path))?
    } else {
        info!("No config file specified, using development defaults");
        ProxyConfig::development()
    };

    // Handle ACME certificate provisioning if configured
    if let Some(ref tls_config) = config.tls {
        if let Some(ref email) = tls_config.acme_email {
            info!("ACME configured with email: {}", email);

            let storage_dir = std::env::var("PROXY_CERT_DIR")
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|_| std::path::PathBuf::from("/var/lib/proxy-rs/certs"));

            let acme_manager = Arc::new(AcmeManager::new(
                tls_config.acme_directory.clone(),
                email.clone(),
                tls_config.domains.clone(),
                storage_dir,
            ));

            // Check if we need to provision certificates
            if acme_manager.needs_certificate() {
                info!("Provisioning TLS certificate via ACME...");
                if let Err(e) = acme_manager.request_certificate().await {
                    error!("Failed to provision certificate: {}", e);
                    warn!("Falling back to self-signed certificate");
                }
            }

            // Start certificate renewal background task
            acme_manager.clone().start_renewal_task();
        }
    }

    // Create the proxy server
    let server = ProxyServer::new(config.clone())?;

    // Start HTTP redirect server if TLS is enabled
    if config.tls.is_some() {
        let http_addr = format!("0.0.0.0:{}", config.server.http_redirect_port);
        let redirect_server = ProxyServer::new(config.clone())?;

        tokio::spawn(async move {
            if let Err(e) = redirect_server.run_http_redirect(&http_addr).await {
                error!("HTTP redirect server error: {}", e);
            }
        });
    }

    // Run the main proxy server
    server.run().await?;

    Ok(())
}
