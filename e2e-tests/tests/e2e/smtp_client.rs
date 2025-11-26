use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub struct SmtpTestClient {
    stream: BufReader<TcpStream>,
}

impl SmtpTestClient {
    /// Connect to SMTP server
    pub async fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("Failed to connect to SMTP: {}", e))?;

        let mut client = Self {
            stream: BufReader::new(stream),
        };

        // Read greeting
        let greeting = client.read_response().await?;
        if !greeting.starts_with("220") {
            return Err(format!("Unexpected greeting: {}", greeting));
        }

        Ok(client)
    }

    /// Send EHLO command
    pub async fn ehlo(&mut self, hostname: &str) -> Result<String, String> {
        self.send_command(&format!("EHLO {}", hostname)).await?;
        self.read_response().await
    }

    /// Send MAIL FROM command
    pub async fn mail_from(&mut self, from: &str) -> Result<String, String> {
        self.send_command(&format!("MAIL FROM:<{}>", from)).await?;
        self.read_response().await
    }

    /// Send RCPT TO command
    pub async fn rcpt_to(&mut self, to: &str) -> Result<String, String> {
        self.send_command(&format!("RCPT TO:<{}>", to)).await?;
        self.read_response().await
    }

    /// Send DATA command and email content
    pub async fn data(&mut self, content: &str) -> Result<String, String> {
        self.send_command("DATA").await?;
        let response = self.read_response().await?;
        if !response.starts_with("354") {
            return Err(format!("DATA command failed: {}", response));
        }

        // Send email content
        self.send_command(content).await?;
        self.send_command(".").await?;
        self.read_response().await
    }

    /// Send QUIT command
    pub async fn quit(mut self) -> Result<String, String> {
        self.send_command("QUIT").await?;
        self.read_response().await
    }

    /// Send a command
    async fn send_command(&mut self, command: &str) -> Result<(), String> {
        let line = format!("{}\r\n", command);
        self.stream
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("Failed to send command: {}", e))?;

        // Flush to ensure data is sent immediately
        self.stream
            .get_mut()
            .flush()
            .await
            .map_err(|e| format!("Failed to flush: {}", e))?;
        Ok(())
    }

    /// Read a response (handles multi-line responses like EHLO)
    async fn read_response(&mut self) -> Result<String, String> {
        let mut full_response = String::new();
        let mut line = String::new();

        loop {
            line.clear();
            self.stream
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            if line.is_empty() {
                break; // Connection closed
            }

            full_response.push_str(&line);

            // Check if this is the last line of a multi-line response
            // Format: "250-..." (continuation) or "250 ..." (last line)
            if line.len() >= 4 {
                let code_and_sep = &line[0..4];
                // If char at index 3 is space (not dash), it's the last line
                if code_and_sep.chars().nth(3) == Some(' ') {
                    break;
                }
            }
        }

        Ok(full_response.trim().to_string())
    }

    /// Send a complete email (convenience method)
    pub async fn send_email(
        &mut self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> Result<(), String> {
        self.ehlo("test-client").await?;
        self.mail_from(from).await?;
        self.rcpt_to(to).await?;

        let email_content = format!(
            "From: {}\r\nTo: {}\r\nSubject: {}\r\n\r\n{}",
            from, to, subject, body
        );

        let response = self.data(&email_content).await?;
        if !response.starts_with("250") {
            return Err(format!("Email rejected: {}", response));
        }

        Ok(())
    }
}
