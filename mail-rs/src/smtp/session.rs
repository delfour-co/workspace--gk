use crate::error::{MailError, Result};
use crate::smtp::commands::SmtpCommand;
use crate::storage::MaildirStorage;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tracing::{debug, error, info};

#[derive(Debug, Clone, PartialEq)]
enum SmtpState {
    Fresh,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
}

pub struct SmtpSession {
    state: SmtpState,
    from: Option<String>,
    to: Vec<String>,
    data: Vec<u8>,
    domain: String,
    hostname: String,
    storage: Arc<MaildirStorage>,
}

impl SmtpSession {
    pub fn new(domain: String, hostname: String, storage: Arc<MaildirStorage>) -> Self {
        Self {
            state: SmtpState::Fresh,
            from: None,
            to: Vec::new(),
            data: Vec::new(),
            domain,
            hostname,
            storage,
        }
    }

    pub async fn handle(mut self, mut stream: TcpStream) -> Result<()> {
        let (reader, mut writer) = stream.split();
        let mut reader = BufReader::new(reader);

        // Send greeting
        writer
            .write_all(format!("220 {} ESMTP Service Ready\r\n", self.hostname).as_bytes())
            .await?;

        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;

            if n == 0 {
                debug!("Client disconnected");
                break;
            }

            let line_trimmed = line.trim_end();
            debug!("Received: {}", line_trimmed);

            match SmtpCommand::parse(line_trimmed) {
                Ok(cmd) => {
                    let response = self.handle_command(cmd).await?;
                    writer.write_all(response.as_bytes()).await?;

                    if response.starts_with("221") {
                        // QUIT command
                        break;
                    }

                    // Handle DATA mode
                    if self.state == SmtpState::Data {
                        self.receive_data(&mut reader, &mut writer).await?;
                    }
                }
                Err(e) => {
                    error!("Command parse error: {}", e);
                    writer
                        .write_all(b"500 Syntax error, command unrecognized\r\n")
                        .await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_command(&mut self, cmd: SmtpCommand) -> Result<String> {
        match (&self.state, cmd) {
            (SmtpState::Fresh, SmtpCommand::Helo(domain)) => {
                info!("HELO from {}", domain);
                self.state = SmtpState::Greeted;
                Ok(format!("250 {} Hello {}\r\n", self.hostname, domain))
            }
            (SmtpState::Fresh, SmtpCommand::Ehlo(domain)) => {
                info!("EHLO from {}", domain);
                self.state = SmtpState::Greeted;
                Ok(format!(
                    "250-{} Hello {}\r\n250-SIZE 10240000\r\n250 HELP\r\n",
                    self.hostname, domain
                ))
            }
            (SmtpState::Greeted | SmtpState::MailFrom | SmtpState::RcptTo, SmtpCommand::MailFrom(from)) => {
                info!("MAIL FROM: {}", from);
                self.from = Some(from);
                self.to.clear();
                self.data.clear();
                self.state = SmtpState::MailFrom;
                Ok("250 OK\r\n".to_string())
            }
            (SmtpState::MailFrom | SmtpState::RcptTo, SmtpCommand::RcptTo(to)) => {
                info!("RCPT TO: {}", to);
                self.to.push(to);
                self.state = SmtpState::RcptTo;
                Ok("250 OK\r\n".to_string())
            }
            (SmtpState::RcptTo, SmtpCommand::Data) => {
                info!("DATA command received");
                self.state = SmtpState::Data;
                Ok("354 Start mail input; end with <CRLF>.<CRLF>\r\n".to_string())
            }
            (_, SmtpCommand::Rset) => {
                info!("RSET command");
                self.from = None;
                self.to.clear();
                self.data.clear();
                self.state = SmtpState::Greeted;
                Ok("250 OK\r\n".to_string())
            }
            (_, SmtpCommand::Noop) => {
                Ok("250 OK\r\n".to_string())
            }
            (_, SmtpCommand::Quit) => {
                info!("QUIT command");
                Ok(format!("221 {} closing connection\r\n", self.hostname))
            }
            (_, SmtpCommand::Unknown(cmd)) => {
                error!("Unknown command: {}", cmd);
                Ok("502 Command not implemented\r\n".to_string())
            }
            _ => {
                error!("Invalid command sequence");
                Ok("503 Bad sequence of commands\r\n".to_string())
            }
        }
    }

    async fn receive_data<R, W>(
        &mut self,
        reader: &mut BufReader<R>,
        writer: &mut W,
    ) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        let mut line = String::new();

        loop {
            line.clear();
            let n = reader.read_line(&mut line).await?;

            if n == 0 {
                return Err(MailError::SmtpProtocol(
                    "Connection closed during DATA".to_string(),
                ));
            }

            // Check for end of data (.)
            if line.trim_end() == "." {
                info!("End of DATA received");
                break;
            }

            // Handle transparency (lines starting with .)
            if line.starts_with("..") {
                self.data.extend_from_slice(&line.as_bytes()[1..]);
            } else {
                self.data.extend_from_slice(line.as_bytes());
            }
        }

        // Store the email
        self.store_email().await?;

        // Send response
        writer.write_all(b"250 OK: Message accepted\r\n").await?;

        // Reset state for next message
        self.state = SmtpState::Greeted;
        self.from = None;
        self.to.clear();
        self.data.clear();

        Ok(())
    }

    async fn store_email(&self) -> Result<()> {
        if let Some(from) = &self.from {
            for recipient in &self.to {
                info!("Storing email from {} to {}", from, recipient);
                self.storage.store(recipient, &self.data).await?;
            }
            Ok(())
        } else {
            Err(MailError::SmtpProtocol("No sender specified".to_string()))
        }
    }
}
