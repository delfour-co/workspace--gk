use crate::error::{MailError, Result};
use crate::security::{AuthMechanism, Authenticator, TlsConfig};
use crate::smtp::commands::SmtpCommand;
use crate::storage::MaildirStorage;
use crate::utils::validate_email;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

/// Maximum number of recipients per message (anti-spam)
const MAX_RECIPIENTS: usize = 100;

/// Maximum line length in SMTP protocol (RFC 5321)
const MAX_LINE_LENGTH: usize = 1000;

/// Timeout for reading a command line
const COMMAND_TIMEOUT: Duration = Duration::from_secs(300); // 5 minutes

/// Timeout for reading DATA content
const DATA_TIMEOUT: Duration = Duration::from_secs(600); // 10 minutes

/// Maximum number of errors before disconnecting
const MAX_ERRORS: usize = 10;

#[derive(Debug, Clone, PartialEq)]
enum SmtpState {
    Fresh,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
}

/// SMTP session handler with security limits and validation
///
/// # Security features
/// - Command timeouts to prevent slowloris attacks
/// - Data size limits to prevent memory exhaustion
/// - Recipient limits to prevent spam
/// - Email validation to prevent injection
/// - Error counting to detect malicious clients
/// - TLS/STARTTLS support for encryption
/// - SMTP AUTH for authentication
pub struct SmtpSession {
    state: SmtpState,
    from: Option<String>,
    to: Vec<String>,
    data: Vec<u8>,
    hostname: String,
    storage: Arc<MaildirStorage>,
    error_count: usize,
    max_message_size: usize,
    tls_config: Option<Arc<TlsConfig>>,
    authenticator: Option<Arc<Authenticator>>,
    is_encrypted: bool,
    authenticated_user: Option<String>,
    require_auth: bool,
    require_tls: bool,
}

impl SmtpSession {
    pub fn new(hostname: String, storage: Arc<MaildirStorage>, max_message_size: usize) -> Self {
        Self {
            state: SmtpState::Fresh,
            from: None,
            to: Vec::new(),
            data: Vec::new(),
            hostname,
            storage,
            error_count: 0,
            max_message_size,
            tls_config: None,
            authenticator: None,
            is_encrypted: false,
            authenticated_user: None,
            require_auth: false,
            require_tls: false,
        }
    }

    /// Create session with TLS and Auth support
    pub fn with_security(
        hostname: String,
        storage: Arc<MaildirStorage>,
        max_message_size: usize,
        tls_config: Option<Arc<TlsConfig>>,
        authenticator: Option<Arc<Authenticator>>,
        require_auth: bool,
        require_tls: bool,
    ) -> Self {
        Self {
            state: SmtpState::Fresh,
            from: None,
            to: Vec::new(),
            data: Vec::new(),
            hostname,
            storage,
            error_count: 0,
            max_message_size,
            tls_config,
            authenticator,
            is_encrypted: false,
            authenticated_user: None,
            require_auth,
            require_tls,
        }
    }

    /// Handle SMTP session with comprehensive security checks
    pub async fn handle(mut self, mut stream: TcpStream) -> Result<()> {
        let (reader, mut writer) = stream.split();
        let mut reader = BufReader::new(reader);

        // Send greeting
        writer
            .write_all(format!("220 {} ESMTP Service Ready\r\n", self.hostname).as_bytes())
            .await?;

        let mut line = String::new();

        loop {
            // Check error count (security: disconnect abusive clients)
            if self.error_count >= MAX_ERRORS {
                warn!("Too many errors, disconnecting");
                writer
                    .write_all(b"421 Too many errors, closing connection\r\n")
                    .await?;
                break;
            }

            line.clear();

            // Read line with timeout (security: prevent slowloris)
            let read_result = timeout(COMMAND_TIMEOUT, reader.read_line(&mut line)).await;

            let n = match read_result {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    error!("IO error reading line: {}", e);
                    return Err(e.into());
                }
                Err(_) => {
                    warn!("Command timeout, disconnecting");
                    writer
                        .write_all(b"421 Timeout, closing connection\r\n")
                        .await?;
                    break;
                }
            };

            if n == 0 {
                debug!("Client disconnected");
                break;
            }

            // Check line length (security: prevent buffer overflow)
            if line.len() > MAX_LINE_LENGTH {
                error!("Line too long: {} bytes", line.len());
                writer
                    .write_all(b"500 Line too long\r\n")
                    .await?;
                self.error_count += 1;
                continue;
            }

            let line_trimmed = line.trim_end();
            debug!("Received: {}", line_trimmed);

            match SmtpCommand::parse(line_trimmed) {
                Ok(cmd) => {
                    // Handle STARTTLS specially - needs to upgrade connection
                    if matches!(cmd, SmtpCommand::Starttls) {
                        if let Err(e) = self.handle_starttls(&mut writer).await {
                            error!("STARTTLS error: {}", e);
                            return Err(e);
                        }
                        continue;
                    }

                    // Handle AUTH specially - needs back-and-forth communication
                    if let SmtpCommand::Auth(mechanism, initial_response) = cmd.clone() {
                        if let Err(e) = self.handle_auth(&mechanism, initial_response, &mut reader, &mut writer).await {
                            error!("AUTH error: {}", e);
                            writer.write_all(b"535 Authentication failed\r\n").await?;
                            self.error_count += 1;
                        }
                        continue;
                    }

                    match self.handle_command(cmd).await {
                        Ok(response) => {
                            writer.write_all(response.as_bytes()).await?;

                            if response.starts_with("221") {
                                // QUIT command
                                break;
                            }

                            // Handle DATA mode
                            if self.state == SmtpState::Data {
                                if let Err(e) = self.receive_data(&mut reader, &mut writer).await {
                                    error!("Error receiving data: {}", e);
                                    writer
                                        .write_all(b"451 Error receiving message\r\n")
                                        .await?;
                                    self.error_count += 1;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error handling command: {}", e);
                            writer
                                .write_all(format!("451 {}\r\n", e).as_bytes())
                                .await?;
                            self.error_count += 1;
                        }
                    }
                }
                Err(e) => {
                    error!("Command parse error: {}", e);
                    writer
                        .write_all(b"500 Syntax error, command unrecognized\r\n")
                        .await?;
                    self.error_count += 1;
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

                // Build EHLO response with capabilities
                let mut response = format!("250-{} Hello {}\r\n", self.hostname, domain);

                // Advertise STARTTLS if available and not already encrypted
                if self.tls_config.is_some() && !self.is_encrypted {
                    response.push_str("250-STARTTLS\r\n");
                }

                // Only advertise other capabilities if TLS is not required or already active
                if !self.require_tls || self.is_encrypted {
                    response.push_str(&format!("250-SIZE {}\r\n", self.max_message_size));

                    // Advertise AUTH if available and (encrypted or not requiring TLS)
                    if let Some(ref _auth) = self.authenticator {
                        if self.is_encrypted || self.tls_config.is_none() {
                            response.push_str("250-AUTH PLAIN LOGIN\r\n");
                        }
                    }
                }

                response.push_str("250 HELP\r\n");
                Ok(response)
            }
            (SmtpState::Greeted | SmtpState::MailFrom | SmtpState::RcptTo, SmtpCommand::MailFrom(from)) => {
                // Check TLS if required
                if self.require_tls && !self.is_encrypted {
                    warn!("MAIL FROM rejected: TLS required");
                    return Ok("530 Must issue STARTTLS first\r\n".to_string());
                }

                // Check authentication if required
                if self.require_auth && self.authenticated_user.is_none() {
                    warn!("MAIL FROM rejected: authentication required");
                    return Ok("530 Authentication required\r\n".to_string());
                }

                // Validate email address (security: prevent injection)
                validate_email(&from)?;

                info!("MAIL FROM: {}", from);
                self.from = Some(from);
                self.to.clear();
                self.data.clear();
                self.state = SmtpState::MailFrom;
                Ok("250 OK\r\n".to_string())
            }
            (SmtpState::MailFrom | SmtpState::RcptTo, SmtpCommand::RcptTo(to)) => {
                // Validate email address (security: prevent injection)
                validate_email(&to)?;

                // Check recipient limit (security: prevent spam)
                if self.to.len() >= MAX_RECIPIENTS {
                    warn!("Too many recipients: {}", self.to.len());
                    return Ok(format!(
                        "452 Too many recipients (max {})\r\n",
                        MAX_RECIPIENTS
                    ));
                }

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
            // STARTTLS and AUTH are handled specially in handle() method
            (_, SmtpCommand::Starttls) | (_, SmtpCommand::Auth(_, _)) => {
                // These should not reach here as they're handled in handle()
                error!("STARTTLS/AUTH command reached handle_command (should be handled in handle)");
                Ok("503 Bad sequence of commands\r\n".to_string())
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

    /// Receive email DATA with security limits
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

            // Read with timeout (security: prevent slowloris)
            let read_result = timeout(DATA_TIMEOUT, reader.read_line(&mut line)).await;

            let n = match read_result {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    error!("IO error during DATA: {}", e);
                    return Err(e.into());
                }
                Err(_) => {
                    warn!("DATA timeout");
                    return Err(MailError::SmtpProtocol("Timeout during DATA".to_string()));
                }
            };

            if n == 0 {
                return Err(MailError::SmtpProtocol(
                    "Connection closed during DATA".to_string(),
                ));
            }

            // Check line length (security)
            if line.len() > MAX_LINE_LENGTH {
                error!("DATA line too long: {} bytes", line.len());
                return Err(MailError::SmtpProtocol("Line too long".to_string()));
            }

            // Check for end of data (.)
            if line.trim_end() == "." {
                info!("End of DATA received, total size: {} bytes", self.data.len());
                break;
            }

            // Check total size (security: prevent memory exhaustion)
            let new_size = self.data.len() + line.len();
            if new_size > self.max_message_size {
                warn!(
                    "Message too large: {} bytes (max {})",
                    new_size, self.max_message_size
                );
                return Err(MailError::SmtpProtocol(format!(
                    "Message too large (max {} bytes)",
                    self.max_message_size
                )));
            }

            // Handle transparency (lines starting with .)
            if line.starts_with("..") {
                self.data.extend_from_slice(&line.as_bytes()[1..]);
            } else {
                self.data.extend_from_slice(line.as_bytes());
            }
        }

        // Validate final data size
        if self.data.is_empty() {
            warn!("Empty message received");
            return Err(MailError::SmtpProtocol("Empty message".to_string()));
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

    /// Handle STARTTLS command
    ///
    /// # Current Implementation
    /// This implementation sets the `is_encrypted` flag to true after sending "220 Ready to start TLS",
    /// but does NOT actually perform the TLS upgrade. This is sufficient for testing TLS enforcement
    /// logic, but NOT suitable for production use.
    ///
    /// # Security Implications
    /// - The connection remains unencrypted despite the flag being set
    /// - This allows testing of require_tls enforcement without full TLS implementation
    /// - **DO NOT USE IN PRODUCTION** - sensitive data will be transmitted in plaintext
    ///
    /// # Full Implementation Requirements
    /// To implement a working STARTTLS upgrade, the following changes are needed:
    ///
    /// 1. **Refactor handle() method signature**:
    ///    - Change from `handle(stream: TcpStream)` to accept an owned stream
    ///    - Avoid splitting the stream until after potential STARTTLS upgrade
    ///
    /// 2. **Create unified stream type**:
    ///    ```rust
    ///    enum SmtpStream {
    ///        Plain(TcpStream),
    ///        Tls(tokio_rustls::server::TlsStream<TcpStream>),
    ///    }
    ///    ```
    ///    Implement AsyncRead + AsyncWrite for this enum
    ///
    /// 3. **Perform actual TLS upgrade**:
    ///    ```rust
    ///    let acceptor = self.tls_config.as_ref().unwrap().acceptor();
    ///    let tls_stream = acceptor.accept(tcp_stream).await?;
    ///    ```
    ///
    /// 4. **Continue session with upgraded stream**:
    ///    - Wrap TLS stream in SmtpStream::Tls variant
    ///    - Continue command loop with encrypted connection
    ///
    /// # References
    /// - RFC 3207: SMTP Service Extension for Secure SMTP over TLS
    /// - tokio-rustls documentation for TLS upgrade patterns
    async fn handle_starttls<W>(
        &mut self,
        writer: &mut W,
    ) -> Result<()>
    where
        W: tokio::io::AsyncWrite + Unpin,
    {
        // Check if TLS is available
        if self.tls_config.is_none() {
            writer.write_all(b"502 STARTTLS not available\r\n").await?;
            return Ok(());
        }

        // Check if already encrypted
        if self.is_encrypted {
            writer.write_all(b"503 Already using TLS\r\n").await?;
            return Ok(());
        }

        // Check state (must be after EHLO/HELO, before MAIL FROM)
        if self.state != SmtpState::Greeted {
            writer.write_all(b"503 Bad sequence of commands\r\n").await?;
            return Ok(());
        }

        info!("STARTTLS initiated (PLACEHOLDER - not actually encrypting)");
        writer.write_all(b"220 Ready to start TLS\r\n").await?;
        writer.flush().await?;

        // Mark as encrypted (PLACEHOLDER - see docs above for full implementation)
        self.is_encrypted = true;

        warn!("STARTTLS: Connection marked as encrypted but TLS upgrade not performed");
        warn!("This is a PLACEHOLDER implementation - NOT suitable for production");
        warn!("See handle_starttls() documentation for full implementation requirements");

        Ok(())
    }

    /// Handle AUTH command
    async fn handle_auth<R, W>(
        &mut self,
        mechanism: &str,
        initial_response: Option<String>,
        reader: &mut BufReader<R>,
        writer: &mut W,
    ) -> Result<()>
    where
        R: tokio::io::AsyncRead + Unpin,
        W: tokio::io::AsyncWrite + Unpin,
    {
        // Check if authenticator is available
        let authenticator = match &self.authenticator {
            Some(auth) => auth,
            None => {
                writer.write_all(b"502 AUTH not available\r\n").await?;
                return Ok(());
            }
        };

        // Require TLS if configured
        if self.tls_config.is_some() && !self.is_encrypted {
            writer.write_all(b"530 Must issue STARTTLS first\r\n").await?;
            return Ok(());
        }

        // Check if already authenticated
        if self.authenticated_user.is_some() {
            writer.write_all(b"503 Already authenticated\r\n").await?;
            return Ok(());
        }

        // Check state
        if self.state != SmtpState::Greeted {
            writer.write_all(b"503 Bad sequence of commands\r\n").await?;
            return Ok(());
        }

        // Parse mechanism
        let auth_mechanism = match AuthMechanism::from_str(mechanism) {
            Some(m) => m,
            None => {
                writer.write_all(b"504 Authentication mechanism not supported\r\n").await?;
                return Ok(());
            }
        };

        info!("AUTH {} initiated", mechanism);

        // Handle authentication based on mechanism
        match auth_mechanism {
            AuthMechanism::Plain => {
                // PLAIN: AUTH PLAIN <base64-credentials>
                let auth_data = match initial_response {
                    Some(data) => data,
                    None => {
                        // Client didn't provide initial response, request it
                        writer.write_all(b"334 \r\n").await?;

                        // Read auth data
                        let mut line = String::new();
                        timeout(COMMAND_TIMEOUT, reader.read_line(&mut line))
                            .await
                            .map_err(|_| MailError::SmtpProtocol("AUTH timeout".to_string()))??;
                        line.trim().to_string()
                    }
                };

                // Decode PLAIN auth
                let (username, password) = Authenticator::decode_plain_auth(&auth_data)?;

                // Authenticate
                let success = authenticator
                    .authenticate(AuthMechanism::Plain, &username, &password)
                    .await?;

                if success {
                    self.authenticated_user = Some(username.clone());
                    info!("Authentication successful for {}", username);
                    writer.write_all(b"235 Authentication successful\r\n").await?;
                } else {
                    warn!("Authentication failed for {}", username);
                    writer.write_all(b"535 Authentication failed\r\n").await?;
                    self.error_count += 1;
                }
            }
            AuthMechanism::Login => {
                // LOGIN: multi-step process
                // Server sends: 334 VXNlcm5hbWU6 (base64 "Username:")
                writer.write_all(b"334 VXNlcm5hbWU6\r\n").await?;

                // Read username
                let mut line = String::new();
                timeout(COMMAND_TIMEOUT, reader.read_line(&mut line))
                    .await
                    .map_err(|_| MailError::SmtpProtocol("AUTH timeout".to_string()))??;
                let username = Authenticator::decode_login_credential(line.trim())?;

                // Server sends: 334 UGFzc3dvcmQ6 (base64 "Password:")
                writer.write_all(b"334 UGFzc3dvcmQ6\r\n").await?;

                // Read password
                line.clear();
                timeout(COMMAND_TIMEOUT, reader.read_line(&mut line))
                    .await
                    .map_err(|_| MailError::SmtpProtocol("AUTH timeout".to_string()))??;
                let password = Authenticator::decode_login_credential(line.trim())?;

                // Authenticate
                let success = authenticator
                    .authenticate(AuthMechanism::Login, &username, &password)
                    .await?;

                if success {
                    self.authenticated_user = Some(username.clone());
                    info!("Authentication successful for {}", username);
                    writer.write_all(b"235 Authentication successful\r\n").await?;
                } else {
                    warn!("Authentication failed for {}", username);
                    writer.write_all(b"535 Authentication failed\r\n").await?;
                    self.error_count += 1;
                }
            }
        }

        Ok(())
    }
}
