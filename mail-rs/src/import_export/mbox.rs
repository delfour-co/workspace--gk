//! MBOX format handling
//!
//! Provides parsing and writing of MBOX format files.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use std::io::{BufRead, BufReader, Read, Write};

/// MBOX writer for exporting messages
pub struct MboxWriter<W: Write> {
    writer: W,
    message_count: u64,
}

impl<W: Write> MboxWriter<W> {
    /// Create a new MBOX writer
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            message_count: 0,
        }
    }

    /// Write a message to the MBOX file
    pub fn write_message(&mut self, from: &str, date: Option<DateTime<Utc>>, raw_message: &[u8]) -> Result<()> {
        // Write From_ line (MBOX separator)
        let date_str = date
            .map(|d| d.format("%a %b %d %H:%M:%S %Y").to_string())
            .unwrap_or_else(|| Utc::now().format("%a %b %d %H:%M:%S %Y").to_string());

        let from_addr = if from.is_empty() { "MAILER-DAEMON" } else { from };

        writeln!(self.writer, "From {} {}", from_addr, date_str)?;

        // Write message content, escaping From_ lines
        let mut in_body = false;
        for line in raw_message.split(|&b| b == b'\n') {
            if line.is_empty() && !in_body {
                in_body = true;
            }

            // Escape lines starting with "From " in the body
            if in_body && line.starts_with(b"From ") {
                self.writer.write_all(b">")?;
            }

            self.writer.write_all(line)?;
            self.writer.write_all(b"\n")?;
        }

        // Add blank line separator
        writeln!(self.writer)?;

        self.message_count += 1;
        Ok(())
    }

    /// Get the number of messages written
    pub fn message_count(&self) -> u64 {
        self.message_count
    }

    /// Finish writing and return the inner writer
    pub fn finish(self) -> W {
        self.writer
    }
}

/// MBOX reader for importing messages
pub struct MboxReader<R: Read> {
    reader: BufReader<R>,
    current_line: String,
    message_count: u64,
    eof: bool,
    /// Flag indicating we already have a From_ line in current_line
    has_pending_from: bool,
}

impl<R: Read> MboxReader<R> {
    /// Create a new MBOX reader
    pub fn new(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
            current_line: String::new(),
            message_count: 0,
            eof: false,
            has_pending_from: false,
        }
    }

    /// Read the next message from the MBOX file
    pub fn read_message(&mut self) -> Result<Option<MboxMessage>> {
        if self.eof {
            return Ok(None);
        }

        // Check if we already have a From_ line from the previous read
        if !self.has_pending_from {
            // Find the From_ line
            loop {
                self.current_line.clear();
                let bytes_read = self.reader.read_line(&mut self.current_line)?;

                if bytes_read == 0 {
                    self.eof = true;
                    return Ok(None);
                }

                if self.current_line.starts_with("From ") {
                    break;
                }
            }
        }

        self.has_pending_from = false;

        // Parse the From_ line
        let from_line = self.current_line.trim_end();
        let (from_addr, date) = parse_from_line(from_line);

        // Read the message content until next From_ line or EOF
        let mut message_content = Vec::new();

        loop {
            self.current_line.clear();
            let bytes_read = self.reader.read_line(&mut self.current_line)?;

            if bytes_read == 0 {
                self.eof = true;
                break;
            }

            // Check for next message
            if self.current_line.starts_with("From ") {
                // We've hit the next message - preserve this line for the next call
                self.has_pending_from = true;
                break;
            }

            // Unescape >From lines
            let line = if self.current_line.starts_with(">From ") {
                &self.current_line[1..]
            } else {
                &self.current_line
            };

            message_content.extend_from_slice(line.as_bytes());
        }

        // Remove trailing blank lines
        while message_content.ends_with(b"\n\n") {
            message_content.pop();
        }

        self.message_count += 1;

        Ok(Some(MboxMessage {
            from: from_addr,
            date,
            content: message_content,
        }))
    }

    /// Get the number of messages read
    pub fn message_count(&self) -> u64 {
        self.message_count
    }
}

/// Parse the From_ line to extract sender and date
fn parse_from_line(line: &str) -> (String, Option<DateTime<Utc>>) {
    // Format: "From sender@example.com Wed Dec 25 12:00:00 2024"
    let parts: Vec<&str> = line.splitn(3, ' ').collect();

    if parts.len() < 2 {
        return (String::new(), None);
    }

    let from_addr = parts[1].to_string();

    let date = if parts.len() >= 3 {
        // Try to parse the date
        parse_mbox_date(parts[2])
    } else {
        None
    };

    (from_addr, date)
}

/// Parse MBOX date format
fn parse_mbox_date(date_str: &str) -> Option<DateTime<Utc>> {
    // Common MBOX date format: "Wed Dec 25 12:00:00 2024"
    chrono::NaiveDateTime::parse_from_str(date_str.trim(), "%a %b %d %H:%M:%S %Y")
        .ok()
        .map(|dt| dt.and_utc())
}

/// A message from an MBOX file
#[derive(Debug)]
pub struct MboxMessage {
    /// Sender from the From_ line
    pub from: String,
    /// Date from the From_ line
    pub date: Option<DateTime<Utc>>,
    /// Raw message content (headers + body)
    pub content: Vec<u8>,
}

impl MboxMessage {
    /// Get the size of the message content
    pub fn size(&self) -> usize {
        self.content.len()
    }
}

/// Count messages in an MBOX file without fully parsing
pub fn count_messages<R: Read>(reader: R) -> Result<u64> {
    let mut buf_reader = BufReader::new(reader);
    let mut count = 0u64;
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = buf_reader.read_line(&mut line)?;

        if bytes_read == 0 {
            break;
        }

        if line.starts_with("From ") {
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_mbox_write_read() {
        let mut buffer = Vec::new();

        // Write messages
        {
            let mut writer = MboxWriter::new(&mut buffer);
            writer.write_message("test@example.com", None, b"Subject: Test\n\nHello World").unwrap();
            writer.write_message("test2@example.com", None, b"Subject: Test 2\n\nHello Again").unwrap();
        }

        // Read them back
        let cursor = Cursor::new(&buffer);
        let mut reader = MboxReader::new(cursor);

        let msg1 = reader.read_message().unwrap().unwrap();
        assert_eq!(msg1.from, "test@example.com");
        assert!(msg1.content.starts_with(b"Subject: Test"));

        let msg2 = reader.read_message().unwrap().unwrap();
        assert_eq!(msg2.from, "test2@example.com");

        assert!(reader.read_message().unwrap().is_none());
    }
}
