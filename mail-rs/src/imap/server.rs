//! IMAP server implementation
//!
//! Handles TCP connections and IMAP protocol

use crate::config::Config;
use crate::error::MailError;
use crate::imap::{ImapCommand, ImapSession, SessionState};
use crate::security::Authenticator;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

/// IMAP server
pub struct ImapServer {
    config: Arc<Config>,
}

impl ImapServer {
    /// Create a new IMAP server
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    /// Start the IMAP server
    pub async fn start(&self) -> Result<(), MailError> {
        let addr = &self.config.imap.listen_addr;
        let listener = TcpListener::bind(addr).await?;

        info!("🌐 IMAP server listening on {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, peer_addr)) => {
                    info!("📨 New IMAP connection from {}", peer_addr);
                    let config = Arc::clone(&self.config);

                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, config).await {
                            error!("Error handling IMAP connection: {}", e);
                        }
                    });
                }
                Err(e) => {
                    error!("Failed to accept IMAP connection: {}", e);
                }
            }
        }
    }
}

/// Handle a single IMAP connection
async fn handle_connection(stream: TcpStream, config: Arc<Config>) -> Result<(), MailError> {
    let peer_addr = stream.peer_addr()?;
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Send greeting
    writer
        .write_all(b"* OK IMAP4rev1 Service Ready\r\n")
        .await?;

    // Create session
    let authenticator = Authenticator::new(&config.storage.database_url).await?;
    let mut session = ImapSession::new(authenticator, config.storage.maildir_path.clone());

    let mut line = String::new();

    loop {
        line.clear();

        // Read command
        match reader.read_line(&mut line).await {
            Ok(0) => {
                // Connection closed
                info!("Connection closed by {}", peer_addr);
                break;
            }
            Ok(_) => {
                debug!("Received from {}: {}", peer_addr, line.trim());

                // Parse command
                match ImapCommand::parse(&line) {
                    Ok((tag, command)) => {
                        // Handle command
                        match session.handle_command(tag.clone(), command).await {
                            Ok(response) => {
                                debug!("Sending to {}: {}", peer_addr, response.trim());
                                writer.write_all(response.as_bytes()).await?;

                                // Check if we should close connection
                                if matches!(session.state(), SessionState::Logout) {
                                    info!("Logging out connection from {}", peer_addr);
                                    break;
                                }
                            }
                            Err(e) => {
                                warn!("Error handling command: {}", e);
                                let error_response =
                                    format!("{} BAD Error: {}\r\n", tag, e);
                                writer.write_all(error_response.as_bytes()).await?;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed to parse command: {}", e);
                        writer
                            .write_all(b"* BAD Failed to parse command\r\n")
                            .await?;
                    }
                }
            }
            Err(e) => {
                error!("Error reading from {}: {}", peer_addr, e);
                break;
            }
        }
    }

    info!("IMAP connection from {} closed", peer_addr);
    Ok(())
}
