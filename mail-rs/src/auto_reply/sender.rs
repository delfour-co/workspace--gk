//! Auto-reply email sender

use crate::auto_reply::{AutoReplyConfig, AutoReplyManager};
use crate::error::MailError;
use chrono::Utc;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpStream;
use tracing::{debug, info};

/// Handles sending auto-reply emails
pub struct AutoReplySender {
    manager: Arc<AutoReplyManager>,
    smtp_host: String,
    smtp_port: u16,
}

impl AutoReplySender {
    /// Create a new auto-reply sender
    pub fn new(manager: Arc<AutoReplyManager>, smtp_host: String, smtp_port: u16) -> Self {
        Self {
            manager,
            smtp_host,
            smtp_port,
        }
    }

    /// Process an incoming message and send auto-reply if needed
    ///
    /// # Arguments
    /// * `recipient_email` - The mailbox that received the message
    /// * `sender_email` - The sender of the incoming message
    /// * `original_subject` - Subject of the original message (optional)
    pub async fn process_incoming_message(
        &self,
        recipient_email: &str,
        sender_email: &str,
        original_subject: Option<&str>,
    ) -> Result<bool, MailError> {
        // Check if auto-reply should be sent
        if !self
            .manager
            .should_send_auto_reply(recipient_email, sender_email)
            .await?
        {
            debug!(
                "Auto-reply not needed for {} -> {}",
                sender_email, recipient_email
            );
            return Ok(false);
        }

        // Get auto-reply configuration
        let config = self
            .manager
            .get_config(recipient_email)
            .await?
            .ok_or_else(|| {
                MailError::NotFound(format!(
                    "Auto-reply config not found for {}",
                    recipient_email
                ))
            })?;

        // Send the auto-reply
        self.send_auto_reply(&config, sender_email, original_subject)
            .await?;

        // Record that we sent it
        self.manager
            .record_auto_reply_sent(recipient_email, sender_email)
            .await?;

        info!(
            "Sent auto-reply from {} to {}",
            recipient_email, sender_email
        );
        Ok(true)
    }

    /// Send an auto-reply email
    async fn send_auto_reply(
        &self,
        config: &AutoReplyConfig,
        to_email: &str,
        original_subject: Option<&str>,
    ) -> Result<(), MailError> {
        // Build subject line (optionally include original subject)
        let subject = if let Some(orig_subj) = original_subject {
            if orig_subj.starts_with("Re:") {
                // Already a reply, just use our subject
                config.subject.clone()
            } else {
                // Add Re: prefix with original subject reference
                format!("{} (Re: {})", config.subject, orig_subj)
            }
        } else {
            config.subject.clone()
        };

        // Build email message (RFC 5322 format)
        let message = self.build_email_message(
            &config.email,
            to_email,
            &subject,
            &config.body_html,
            &config.body_text,
        );

        // Send via SMTP
        self.send_via_smtp(&config.email, to_email, &message)
            .await?;

        Ok(())
    }

    /// Build RFC 5322 compliant email message
    fn build_email_message(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        html_body: &str,
        text_body: &str,
    ) -> String {
        let date = Utc::now().format("%a, %d %b %Y %H:%M:%S %z");
        let boundary = format!("----=_Part_{}", uuid::Uuid::new_v4().simple());

        format!(
            "From: <{}>\r\n\
             To: <{}>\r\n\
             Subject: {}\r\n\
             Date: {}\r\n\
             Auto-Submitted: auto-replied\r\n\
             X-Auto-Response-Suppress: All\r\n\
             MIME-Version: 1.0\r\n\
             Content-Type: multipart/alternative; boundary=\"{}\"\r\n\
             \r\n\
             --{}\r\n\
             Content-Type: text/plain; charset=\"UTF-8\"\r\n\
             Content-Transfer-Encoding: 7bit\r\n\
             \r\n\
             {}\r\n\
             --{}\r\n\
             Content-Type: text/html; charset=\"UTF-8\"\r\n\
             Content-Transfer-Encoding: 7bit\r\n\
             \r\n\
             {}\r\n\
             --{}--",
            from,
            to,
            subject,
            date,
            boundary,
            boundary,
            text_body,
            boundary,
            html_body,
            boundary
        )
    }

    /// Send email via SMTP (simple implementation)
    async fn send_via_smtp(
        &self,
        from: &str,
        to: &str,
        message: &str,
    ) -> Result<(), MailError> {
        let addr = format!("{}:{}", self.smtp_host, self.smtp_port);

        // Connect to SMTP server
        let mut stream = TcpStream::connect(&addr).await.map_err(|e| {
            MailError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionRefused,
                format!("Failed to connect to SMTP server {}: {}", addr, e),
            ))
        })?;

        // Read greeting
        let mut buf = vec![0u8; 1024];
        let _ = stream.peek(&mut buf).await?;

        // Send HELO
        stream
            .write_all(b"HELO localhost\r\n")
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send MAIL FROM
        let mail_from = format!("MAIL FROM:<{}>\r\n", from);
        stream
            .write_all(mail_from.as_bytes())
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send RCPT TO
        let rcpt_to = format!("RCPT TO:<{}>\r\n", to);
        stream
            .write_all(rcpt_to.as_bytes())
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send DATA
        stream
            .write_all(b"DATA\r\n")
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send message
        stream
            .write_all(message.as_bytes())
            .await
            .map_err(|e| MailError::Io(e))?;
        stream
            .write_all(b"\r\n.\r\n")
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Send QUIT
        stream
            .write_all(b"QUIT\r\n")
            .await
            .map_err(|e| MailError::Io(e))?;
        stream.flush().await.map_err(|e| MailError::Io(e))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auto_reply::CreateAutoReplyRequest;
    use sqlx::SqlitePool;

    async fn setup_test_manager() -> Arc<AutoReplyManager> {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let manager = Arc::new(AutoReplyManager::new(pool));
        manager.init_db().await.unwrap();
        manager
    }

    #[tokio::test]
    async fn test_should_process_when_active() {
        let manager = setup_test_manager().await;

        // Create active auto-reply config
        let request = CreateAutoReplyRequest {
            is_active: true,
            start_date: None,
            end_date: None,
            subject: "Out of Office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: Some(24),
        };

        manager
            .set_config("test@example.com", request)
            .await
            .unwrap();

        // Should return true for first check
        let should_send = manager
            .should_send_auto_reply("test@example.com", "sender@example.com")
            .await
            .unwrap();

        assert!(should_send);
    }

    #[tokio::test]
    async fn test_no_process_when_inactive() {
        let manager = setup_test_manager().await;

        // Create inactive auto-reply config
        let request = CreateAutoReplyRequest {
            is_active: false,
            start_date: None,
            end_date: None,
            subject: "Out of Office".to_string(),
            body_html: "<p>I'm away</p>".to_string(),
            body_text: "I'm away".to_string(),
            reply_interval_hours: Some(24),
        };

        manager
            .set_config("test@example.com", request)
            .await
            .unwrap();

        // Should return false when inactive
        let should_send = manager
            .should_send_auto_reply("test@example.com", "sender@example.com")
            .await
            .unwrap();

        assert!(!should_send);
    }
}
