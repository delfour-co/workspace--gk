use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

pub struct ImapTestClient {
    stream: BufReader<TcpStream>,
    tag_counter: u32,
}

impl ImapTestClient {
    /// Connect to IMAP server
    pub async fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .await
            .map_err(|e| format!("Failed to connect to IMAP: {}", e))?;

        let mut client = Self {
            stream: BufReader::new(stream),
            tag_counter: 0,
        };

        // Read greeting
        let greeting = client.read_response().await?;
        if !greeting.contains("OK") {
            return Err(format!("Unexpected greeting: {}", greeting));
        }

        Ok(client)
    }

    /// Login to IMAP server
    pub async fn login(&mut self, username: &str, password: &str) -> Result<(), String> {
        let command = format!("LOGIN {} {}", username, password);
        let response = self.send_command(&command).await?;

        if !response.contains("OK") {
            return Err(format!("Login failed: {}", response));
        }

        Ok(())
    }

    /// Select a mailbox
    pub async fn select(&mut self, mailbox: &str) -> Result<MailboxInfo, String> {
        let response = self.send_command(&format!("SELECT {}", mailbox)).await?;

        if !response.contains("OK") {
            return Err(format!("SELECT failed: {}", response));
        }

        // Parse EXISTS from response
        let exists = response
            .lines()
            .find(|line| line.contains("EXISTS"))
            .and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<usize>().ok())
            })
            .unwrap_or(0);

        Ok(MailboxInfo { exists })
    }

    /// Fetch email by sequence number
    pub async fn fetch(&mut self, seq: usize, what: &str) -> Result<String, String> {
        let command = format!("FETCH {} {}", seq, what);
        self.send_command(&command).await
    }

    /// Search emails
    pub async fn search(&mut self, criteria: &str) -> Result<Vec<usize>, String> {
        let response = self.send_command(&format!("SEARCH {}", criteria)).await?;

        if !response.contains("OK") {
            return Err(format!("SEARCH failed: {}", response));
        }

        // Parse sequence numbers from * SEARCH response
        let numbers: Vec<usize> = response
            .lines()
            .find(|line| line.starts_with("* SEARCH"))
            .map(|line| {
                line.split_whitespace()
                    .skip(2) // Skip "* SEARCH"
                    .filter_map(|s| s.parse::<usize>().ok())
                    .collect()
            })
            .unwrap_or_default();

        Ok(numbers)
    }

    /// Logout
    pub async fn logout(mut self) -> Result<(), String> {
        self.send_command("LOGOUT").await?;
        Ok(())
    }

    /// Send a command with automatic tagging
    async fn send_command(&mut self, command: &str) -> Result<String, String> {
        self.tag_counter += 1;
        let tag = format!("A{:04}", self.tag_counter);
        let line = format!("{} {}\r\n", tag, command);

        self.stream
            .get_mut()
            .write_all(line.as_bytes())
            .await
            .map_err(|e| format!("Failed to send command: {}", e))?;

        // Read response until we get the tagged response
        let mut response = String::new();
        loop {
            let mut line = String::new();
            self.stream
                .read_line(&mut line)
                .await
                .map_err(|e| format!("Failed to read response: {}", e))?;

            response.push_str(&line);

            // Check if this is the tagged response
            if line.starts_with(&tag) {
                break;
            }
        }

        Ok(response)
    }

    /// Read untagged response (greeting, etc.)
    async fn read_response(&mut self) -> Result<String, String> {
        let mut response = String::new();
        self.stream
            .read_line(&mut response)
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;
        Ok(response.trim().to_string())
    }
}

#[derive(Debug)]
pub struct MailboxInfo {
    pub exists: usize,
}
