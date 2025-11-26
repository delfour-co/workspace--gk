use crate::error::{MailError, Result};
use crate::security::{AuthMechanism, Authenticator, TlsConfig};
use crate::smtp::commands::SmtpCommand;
use crate::storage::MaildirStorage;
use crate::utils::validate_email;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadBuf};
use tokio::net::TcpStream;
use tokio::time::timeout;
use tokio_rustls::server::TlsStream;
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

/// Unified stream type for both plain and TLS connections
///
/// This enum allows us to handle both plain TCP and TLS-encrypted connections
/// through the same interface, enabling STARTTLS upgrades mid-session.
enum SmtpStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
    /// Temporary state during STARTTLS upgrade - should never be observable
    Upgrading,
}

impl AsyncRead for SmtpStream {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(stream) => Pin::new(stream).poll_read(cx, buf),
            SmtpStream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
            SmtpStream::Upgrading => {
                panic!("Attempted I/O on SmtpStream during STARTTLS upgrade")
            }
        }
    }
}

impl AsyncWrite for SmtpStream {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match self.get_mut() {
            SmtpStream::Plain(stream) => Pin::new(stream).poll_write(cx, buf),
            SmtpStream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
            SmtpStream::Upgrading => {
                panic!("Attempted I/O on SmtpStream during STARTTLS upgrade")
            }
        }
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(stream) => Pin::new(stream).poll_flush(cx),
            SmtpStream::Tls(stream) => Pin::new(stream).poll_flush(cx),
            SmtpStream::Upgrading => {
                panic!("Attempted I/O on SmtpStream during STARTTLS upgrade")
            }
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        match self.get_mut() {
            SmtpStream::Plain(stream) => Pin::new(stream).poll_shutdown(cx),
            SmtpStream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
            SmtpStream::Upgrading => {
                panic!("Attempted I/O on SmtpStream during STARTTLS upgrade")
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum SmtpState {
    Fresh,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
}

/// Result of processing SMTP commands
enum SessionResult {
    Continue,  // Continue processing (after STARTTLS upgrade)
    Quit,      // Session ended normally
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

    /// Handle SMTP session with comprehensive security checks and STARTTLS support
    pub async fn handle(mut self, stream: TcpStream) -> Result<()> {
        // Wrap in unified stream type (starts as plain)
        let mut smtp_stream = SmtpStream::Plain(stream);

        // Send greeting
        smtp_stream
            .write_all(format!("220 {} ESMTP Service Ready\r\n", self.hostname).as_bytes())
            .await?;

        // Process the session, potentially upgrading to TLS mid-session
        // We use a loop to handle STARTTLS without recursion
        loop {
            match self.process_commands(&mut smtp_stream).await? {
                SessionResult::Continue => continue,
                SessionResult::Quit => break,
            }
        }

        Ok(())
    }

    /// Process SMTP commands on the given stream
    async fn process_commands(&mut self, stream: &mut SmtpStream) -> Result<SessionResult> {
        // Create a BufReader for efficient line reading
        // Note: We need to be careful with borrowing - when STARTTLS happens,
        // we must drop this reader to regain access to the stream
        // Use &mut *stream to create a fresh reborrow that allows later access
        let mut buf_reader = BufReader::new(&mut *stream);
        let mut line = String::new();

        loop {
            // Check error count (security: disconnect abusive clients)
            if self.error_count >= MAX_ERRORS {
                warn!("Too many errors, disconnecting");
                buf_reader
                    .write_all(b"421 Too many errors, closing connection\r\n")
                    .await?;
                return Ok(SessionResult::Quit);
            }

            line.clear();

            // Read line with timeout (security: prevent slowloris)
            let read_result = timeout(COMMAND_TIMEOUT, buf_reader.read_line(&mut line)).await;

            let n = match read_result {
                Ok(Ok(n)) => n,
                Ok(Err(e)) => {
                    error!("IO error reading line: {}", e);
                    return Err(e.into());
                }
                Err(_) => {
                    warn!("Command timeout, disconnecting");
                    buf_reader
                        .write_all(b"421 Timeout, closing connection\r\n")
                        .await?;
                    return Ok(SessionResult::Quit);
                }
            };

            if n == 0 {
                debug!("Client disconnected");
                return Ok(SessionResult::Quit);
            }

            // Check line length (security: prevent buffer overflow)
            if line.len() > MAX_LINE_LENGTH {
                error!("Line too long: {} bytes", line.len());
                buf_reader
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
                        // Drop buf_reader to regain access to stream
                        drop(buf_reader);

                        match self.handle_starttls_upgrade(stream).await {
                            Ok(true) => {
                                // TLS upgrade successful, return Continue to restart processing
                                info!("STARTTLS upgrade completed, restarting session");
                                return Ok(SessionResult::Continue);
                            }
                            Ok(false) => {
                                // STARTTLS not performed, recreate reader and continue
                                buf_reader = BufReader::new(&mut *stream);
                                continue;
                            }
                            Err(e) => {
                                error!("STARTTLS error: {}", e);
                                return Err(e);
                            }
                        }
                    }

                    // Handle AUTH specially - needs back-and-forth communication
                    if let SmtpCommand::Auth(mechanism, initial_response) = cmd.clone() {
                        if let Err(e) = self.handle_auth(&mechanism, initial_response, &mut buf_reader).await {
                            error!("AUTH error: {}", e);
                            buf_reader.write_all(b"535 Authentication failed\r\n").await?;
                            self.error_count += 1;
                        }
                        continue;
                    }

                    match self.handle_command(cmd).await {
                        Ok(response) => {
                            buf_reader.write_all(response.as_bytes()).await?;

                            if response.starts_with("221") {
                                // QUIT command
                                return Ok(SessionResult::Quit);
                            }

                            // Handle DATA mode
                            if self.state == SmtpState::Data {
                                if let Err(e) = self.receive_data(&mut buf_reader).await {
                                    error!("Error receiving data: {}", e);
                                    buf_reader
                                        .write_all(b"451 Error receiving message\r\n")
                                        .await?;
                                    self.error_count += 1;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Error handling command: {}", e);
                            buf_reader
                                .write_all(format!("451 {}\r\n", e).as_bytes())
                                .await?;
                            self.error_count += 1;
                        }
                    }
                }
                Err(e) => {
                    error!("Command parse error: {}", e);
                    buf_reader
                        .write_all(b"500 Syntax error, command unrecognized\r\n")
                        .await?;
                    self.error_count += 1;
                }
            }
        }
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
    async fn receive_data<S>(
        &mut self,
        buf_reader: &mut BufReader<S>,
    ) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        let mut line = String::new();

        loop {
            line.clear();

            // Read with timeout (security: prevent slowloris)
            let read_result = timeout(DATA_TIMEOUT, buf_reader.read_line(&mut line)).await;

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
        buf_reader.write_all(b"250 OK: Message accepted\r\n").await?;

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

    /// Handle STARTTLS command and perform TLS upgrade
    ///
    /// # Implementation
    /// This method performs the actual TLS upgrade by:
    /// 1. Validating preconditions (TLS available, not already encrypted, correct state)
    /// 2. Sending "220 Ready to start TLS" response
    /// 3. Extracting the underlying TcpStream from the SmtpStream
    /// 4. Performing the TLS handshake using tokio_rustls
    /// 5. Replacing the plain stream with the TLS stream in-place
    /// 6. Marking the session as encrypted
    ///
    /// # Security
    /// After successful upgrade, all subsequent communication is encrypted.
    /// The session state is preserved, allowing the client to continue with
    /// authenticated commands over the secure connection.
    ///
    /// # RFC 3207 Compliance
    /// - Requires EHLO/HELO before STARTTLS
    /// - Resets to Fresh state after upgrade (client must EHLO again)
    /// - Prevents nested STARTTLS
    ///
    /// # Returns
    /// - `Ok(true)` - TLS upgrade successful, stream has been replaced
    /// - `Ok(false)` - STARTTLS not performed (already encrypted, not available, etc.)
    /// - `Err(_)` - TLS handshake or I/O error
    async fn handle_starttls_upgrade(
        &mut self,
        stream: &mut SmtpStream,
    ) -> Result<bool> {
        // Check if TLS is available
        let tls_config = match &self.tls_config {
            Some(config) => config.clone(),
            None => {
                stream.write_all(b"502 STARTTLS not available\r\n").await?;
                return Ok(false);
            }
        };

        // Check if already encrypted
        if self.is_encrypted {
            stream.write_all(b"503 Already using TLS\r\n").await?;
            return Ok(false);
        }

        // Check state (must be after EHLO/HELO, before MAIL FROM)
        if self.state != SmtpState::Greeted {
            stream.write_all(b"503 Bad sequence of commands\r\n").await?;
            return Ok(false);
        }

        info!("STARTTLS: Initiating TLS upgrade");
        stream.write_all(b"220 Ready to start TLS\r\n").await?;
        stream.flush().await?;

        // Extract the plain TcpStream - use Upgrading as temporary placeholder
        let tcp_stream = match std::mem::replace(stream, SmtpStream::Upgrading) {
            SmtpStream::Plain(tcp) => tcp,
            SmtpStream::Tls(_) => {
                // This shouldn't happen due to is_encrypted check above
                error!("STARTTLS: Stream already TLS despite is_encrypted=false");
                return Err(MailError::SmtpProtocol(
                    "Internal error: stream state mismatch".to_string(),
                ));
            }
            SmtpStream::Upgrading => {
                // This really shouldn't happen
                error!("STARTTLS: Stream in Upgrading state");
                return Err(MailError::SmtpProtocol(
                    "Internal error: stream already upgrading".to_string(),
                ));
            }
        };

        // Perform TLS handshake
        info!("STARTTLS: Performing TLS handshake");
        let acceptor = tls_config.acceptor();
        let tls_stream = acceptor
            .accept(tcp_stream)
            .await
            .map_err(|e| {
                error!("TLS handshake failed: {}", e);
                MailError::SmtpProtocol(format!("TLS handshake failed: {}", e))
            })?;

        // Replace the stream with the TLS version
        *stream = SmtpStream::Tls(tls_stream);
        self.is_encrypted = true;

        // Reset state - client must send EHLO again after STARTTLS (RFC 3207)
        self.state = SmtpState::Fresh;

        info!("STARTTLS: TLS upgrade completed successfully");
        Ok(true)
    }

    /// Handle AUTH command
    async fn handle_auth<S>(
        &mut self,
        mechanism: &str,
        initial_response: Option<String>,
        buf_reader: &mut BufReader<S>,
    ) -> Result<()>
    where
        S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    {
        // Check if authenticator is available
        let authenticator = match &self.authenticator {
            Some(auth) => auth,
            None => {
                buf_reader.write_all(b"502 AUTH not available\r\n").await?;
                return Ok(());
            }
        };

        // Require TLS if configured
        if self.tls_config.is_some() && !self.is_encrypted {
            buf_reader.write_all(b"530 Must issue STARTTLS first\r\n").await?;
            return Ok(());
        }

        // Check if already authenticated
        if self.authenticated_user.is_some() {
            buf_reader.write_all(b"503 Already authenticated\r\n").await?;
            return Ok(());
        }

        // Check state
        if self.state != SmtpState::Greeted {
            buf_reader.write_all(b"503 Bad sequence of commands\r\n").await?;
            return Ok(());
        }

        // Parse mechanism
        let auth_mechanism = match AuthMechanism::from_str(mechanism) {
            Some(m) => m,
            None => {
                buf_reader.write_all(b"504 Authentication mechanism not supported\r\n").await?;
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
                        buf_reader.write_all(b"334 \r\n").await?;

                        // Read auth data
                        let mut line = String::new();
                        timeout(COMMAND_TIMEOUT, buf_reader.read_line(&mut line))
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
                    buf_reader.write_all(b"235 Authentication successful\r\n").await?;
                } else {
                    warn!("Authentication failed for {}", username);
                    buf_reader.write_all(b"535 Authentication failed\r\n").await?;
                    self.error_count += 1;
                }
            }
            AuthMechanism::Login => {
                // LOGIN: multi-step process
                // Server sends: 334 VXNlcm5hbWU6 (base64 "Username:")
                buf_reader.write_all(b"334 VXNlcm5hbWU6\r\n").await?;

                // Read username
                let mut line = String::new();
                timeout(COMMAND_TIMEOUT, buf_reader.read_line(&mut line))
                    .await
                    .map_err(|_| MailError::SmtpProtocol("AUTH timeout".to_string()))??;
                let username = Authenticator::decode_login_credential(line.trim())?;

                // Server sends: 334 UGFzc3dvcmQ6 (base64 "Password:")
                buf_reader.write_all(b"334 UGFzc3dvcmQ6\r\n").await?;

                // Read password
                line.clear();
                timeout(COMMAND_TIMEOUT, buf_reader.read_line(&mut line))
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
                    buf_reader.write_all(b"235 Authentication successful\r\n").await?;
                } else {
                    warn!("Authentication failed for {}", username);
                    buf_reader.write_all(b"535 Authentication failed\r\n").await?;
                    self.error_count += 1;
                }
            }
        }

        Ok(())
    }
}
