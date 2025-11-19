use crate::config::Config;
use crate::error::Result;
use crate::smtp::session::SmtpSession;
use crate::storage::MaildirStorage;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

pub struct SmtpServer {
    config: Config,
    storage: Arc<MaildirStorage>,
}

impl SmtpServer {
    pub fn new(config: Config, storage: Arc<MaildirStorage>) -> Self {
        Self { config, storage }
    }

    pub async fn run(&self) -> Result<()> {
        let listener = TcpListener::bind(&self.config.smtp.listen_addr).await?;
        info!("SMTP server listening on {}", self.config.smtp.listen_addr);

        loop {
            match listener.accept().await {
                Ok((socket, addr)) => {
                    info!("New SMTP connection from {}", addr);

                    let session = SmtpSession::new(
                        self.config.server.hostname.clone(),
                        self.storage.clone(),
                        self.config.smtp.max_message_size,
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
