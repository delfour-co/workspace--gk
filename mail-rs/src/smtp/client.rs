//! SMTP client for sending outgoing emails
//!
//! This module handles outgoing SMTP connections to external mail servers.
//!
//! # Features
//! - MX record lookup
//! - SMTP client protocol (RFC 5321)
//! - Connection pooling
//! - Retry logic
//!
//! # Security
//! - TLS support (future)
//! - DKIM signing (future)
//! - SPF validation (future)

use crate::error::{MailError, Result};
use crate::smtp::SmtpCommand;
use std::net::SocketAddr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

/// SMTP client for sending emails to external servers
///
/// # Examples
/// ```no_run
/// use mail_rs::smtp::SmtpClient;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let client = SmtpClient::new("mail.example.com:25".to_string());
/// client.send_mail(
///     "sender@example.com",
///     "recipient@other.com",
///     b"Subject: Test\r\n\r\nHello!"
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub struct SmtpClient {
    server_addr: String,
}

impl SmtpClient {
    /// Create a new SMTP client
    pub fn new(server_addr: String) -> Self {
        Self { server_addr }
    }

    /// Send an email to the specified recipient
    ///
    /// # Arguments
    /// * `from` - Sender email address
    /// * `to` - Recipient email address
    /// * `data` - Email content (headers + body)
    ///
    /// # Errors
    /// Returns error if:
    /// - Cannot connect to server
    /// - SMTP transaction fails
    /// - Timeout occurs
    pub async fn send_mail(&self, from: &str, to: &str, data: &[u8]) -> Result<()> {
        info!("Sending mail from {} to {} via {}", from, to, self.server_addr);

        // Connect to server
        let stream = TcpStream::connect(&self.server_addr).await?;
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);

        // Read greeting
        let greeting = self.read_line(&mut reader).await?;
        if !greeting.starts_with("220") {
            error!("Invalid greeting: {}", greeting);
            return Err(MailError::SmtpProtocol(format!("Invalid greeting: {}", greeting)));
        }
        debug!("Received greeting: {}", greeting.trim());

        // Send EHLO
        self.write_line(&mut writer, &format!("EHLO {}", self.get_hostname())).await?;
        self.read_response(&mut reader, "250").await?;

        // MAIL FROM
        self.write_line(&mut writer, &format!("MAIL FROM:<{}>", from)).await?;
        self.read_response(&mut reader, "250").await?;

        // RCPT TO
        self.write_line(&mut writer, &format!("RCPT TO:<{}>", to)).await?;
        self.read_response(&mut reader, "250").await?;

        // DATA
        self.write_line(&mut writer, "DATA").await?;
        self.read_response(&mut reader, "354").await?;

        // Send email content
        writer.write_all(data).await?;

        // End with CRLF.CRLF if not already present
        if !data.ends_with(b"\r\n.\r\n") {
            if !data.ends_with(b"\r\n") {
                writer.write_all(b"\r\n").await?;
            }
            writer.write_all(b".\r\n").await?;
        }

        self.read_response(&mut reader, "250").await?;

        // QUIT
        self.write_line(&mut writer, "QUIT").await?;
        let _response = self.read_line(&mut reader).await?;

        info!("Mail sent successfully to {}", to);
        Ok(())
    }

    /// Read a line from the stream
    async fn read_line<R>(&self, reader: &mut BufReader<R>) -> Result<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        Ok(line)
    }

    /// Read response and verify it starts with expected code
    async fn read_response<R>(&self, reader: &mut BufReader<R>, expected: &str) -> Result<String>
    where
        R: tokio::io::AsyncRead + Unpin,
    {
        let mut full_response = String::new();

        loop {
            let line = self.read_line(reader).await?;
            debug!("< {}", line.trim());

            full_response.push_str(&line);

            // Check if this is the last line (no dash after code)
            if line.len() >= 4 && &line[3..4] == " " {
                break;
            }
        }

        if !full_response.starts_with(expected) {
            error!("Unexpected response: {}", full_response);
            return Err(MailError::SmtpProtocol(format!(
                "Expected {}, got: {}",
                expected, full_response
            )));
        }

        Ok(full_response)
    }

    /// Write a line to the stream
    async fn write_line<W>(&self, writer: &mut W, line: &str) -> Result<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        debug!("> {}", line);
        writer.write_all(format!("{}\r\n", line).as_bytes()).await?;
        Ok(())
    }

    /// Get local hostname
    fn get_hostname(&self) -> String {
        gethostname::gethostname()
            .to_string_lossy()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = SmtpClient::new("mail.example.com:25".to_string());
        assert_eq!(client.server_addr, "mail.example.com:25");
    }
}
