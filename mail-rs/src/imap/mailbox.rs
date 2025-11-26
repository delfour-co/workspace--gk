//! Mailbox management for IMAP
//!
//! Handles reading emails from Maildir storage

use crate::error::MailError;
use crate::imap::{SearchCriteria, StoreOperation};
use std::fs;
use std::path::{Path, PathBuf};

/// Represents an email message in the mailbox
#[derive(Debug, Clone)]
pub struct EmailMessage {
    /// Sequence number (1-indexed)
    pub sequence: usize,
    /// Unique ID
    pub uid: String,
    /// Message flags (e.g., \Seen, \Flagged)
    pub flags: Vec<String>,
    /// RFC822 message content
    pub content: Vec<u8>,
    /// Message size in bytes
    pub size: usize,
}

/// Mailbox containing emails
pub struct Mailbox {
    /// Mailbox name (e.g., "INBOX")
    pub name: String,
    /// Path to maildir
    path: PathBuf,
    /// Messages in this mailbox
    messages: Vec<EmailMessage>,
}

impl Mailbox {
    /// Open a mailbox for a given email address
    ///
    /// # Arguments
    /// * `email` - Email address (e.g., "john@example.com")
    /// * `mailbox_name` - Mailbox name (e.g., "INBOX", "Sent", "Drafts")
    /// * `maildir_root` - Root directory for maildirs
    pub fn open(email: &str, mailbox_name: &str, maildir_root: &Path) -> Result<Self, MailError> {
        let maildir_path = maildir_root.join(email);

        // Map IMAP mailbox names to Maildir paths
        // INBOX -> Maildir/new and Maildir/cur
        // Other folders -> Maildir/.FolderName/new and Maildir/.FolderName/cur
        let (folder_path, is_inbox) = if mailbox_name.to_uppercase() == "INBOX" {
            (maildir_path.clone(), true)
        } else {
            // Maildir convention: subfolders start with a dot
            (maildir_path.join(format!(".{}", mailbox_name)), false)
        };

        // Check if folder exists
        if !folder_path.exists() {
            return Err(MailError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Mailbox '{}' not found", mailbox_name),
            )));
        }

        let new_path = folder_path.join("new");
        let cur_path = folder_path.join("cur");

        // Read messages from both new/ and cur/ directories
        let mut messages = Vec::new();
        let mut sequence = 1;

        // Read from new/ directory (unread messages)
        if new_path.exists() {
            if let Ok(entries) = fs::read_dir(&new_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(content) = fs::read(&path) {
                            let size = content.len();
                            let uid = entry.file_name().to_string_lossy().to_string();

                            messages.push(EmailMessage {
                                sequence,
                                uid,
                                flags: vec![], // No flags for messages in new/
                                content,
                                size,
                            });
                            sequence += 1;
                        }
                    }
                }
            }
        }

        // Read from cur/ directory (read messages with flags)
        if cur_path.exists() {
            if let Ok(entries) = fs::read_dir(&cur_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        if let Ok(content) = fs::read(&path) {
                            let size = content.len();
                            let filename = entry.file_name().to_string_lossy().to_string();

                            // Parse Maildir flags from filename (format: unique:2,FLAGS)
                            let flags = Self::parse_maildir_flags(&filename);

                            messages.push(EmailMessage {
                                sequence,
                                uid: filename.clone(),
                                flags,
                                content,
                                size,
                            });
                            sequence += 1;
                        }
                    }
                }
            }
        }

        Ok(Mailbox {
            name: mailbox_name.to_string(),
            path: folder_path,
            messages,
        })
    }

    /// Parse Maildir flags from filename
    /// Maildir format: unique:2,FLAGS where FLAGS can be:
    /// - D (Draft)
    /// - F (Flagged)
    /// - P (Passed/Forwarded)
    /// - R (Replied)
    /// - S (Seen)
    /// - T (Trashed/Deleted)
    fn parse_maildir_flags(filename: &str) -> Vec<String> {
        let mut flags = Vec::new();

        // Look for :2, prefix indicating flags section
        if let Some(flags_part) = filename.split(":2,").nth(1) {
            for c in flags_part.chars() {
                match c {
                    'D' => flags.push("\\Draft".to_string()),
                    'F' => flags.push("\\Flagged".to_string()),
                    'R' => flags.push("\\Answered".to_string()),
                    'S' => flags.push("\\Seen".to_string()),
                    'T' => flags.push("\\Deleted".to_string()),
                    _ => {}
                }
            }
        }

        flags
    }

    /// List all available mailboxes for a given email address
    ///
    /// Returns a list of mailbox names (INBOX, Sent, Drafts, etc.)
    pub fn list_mailboxes(email: &str, maildir_root: &Path) -> Result<Vec<String>, MailError> {
        let maildir_path = maildir_root.join(email);
        let mut mailboxes = Vec::new();

        // INBOX always exists (if maildir exists)
        if maildir_path.exists() {
            mailboxes.push("INBOX".to_string());
        }

        // List all subdirectories starting with '.'
        if let Ok(entries) = fs::read_dir(&maildir_path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = entry.file_name().to_string_lossy().strip_prefix('.') {
                        // It's a Maildir subfolder
                        // Check if it has new/ and cur/ subdirectories
                        let has_structure = path.join("new").exists() || path.join("cur").exists();
                        if has_structure {
                            mailboxes.push(name.to_string());
                        }
                    }
                }
            }
        }

        // Sort for consistent ordering
        mailboxes.sort();

        Ok(mailboxes)
    }

    /// Get total number of messages
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get all messages
    pub fn messages(&self) -> &[EmailMessage] {
        &self.messages
    }

    /// Get number of recent messages (all in new/)
    pub fn recent_count(&self) -> usize {
        self.messages.len()
    }

    /// Get number of unseen messages
    pub fn unseen_count(&self) -> usize {
        self.messages
            .iter()
            .filter(|m| !m.flags.contains(&"\\Seen".to_string()))
            .count()
    }

    /// Get first unseen sequence number
    pub fn first_unseen(&self) -> Option<usize> {
        self.messages
            .iter()
            .find(|m| !m.flags.contains(&"\\Seen".to_string()))
            .map(|m| m.sequence)
    }

    /// Get message by sequence number
    pub fn get_message(&self, sequence: usize) -> Option<&EmailMessage> {
        self.messages.get(sequence.saturating_sub(1))
    }

    /// Get messages by sequence range (e.g., "1:3", "1:*", "1")
    pub fn get_messages(&self, sequence_set: &str) -> Vec<&EmailMessage> {
        let mut result = Vec::new();

        for part in sequence_set.split(',') {
            if part.contains(':') {
                // Range: "1:3" or "1:*"
                let parts: Vec<&str> = part.split(':').collect();
                if parts.len() == 2 {
                    let start = parts[0].parse::<usize>().unwrap_or(1);
                    let end = if parts[1] == "*" {
                        self.messages.len()
                    } else {
                        parts[1].parse::<usize>().unwrap_or(self.messages.len())
                    };

                    for seq in start..=end.min(self.messages.len()) {
                        if let Some(msg) = self.get_message(seq) {
                            result.push(msg);
                        }
                    }
                }
            } else {
                // Single sequence: "1"
                if let Ok(seq) = part.parse::<usize>() {
                    if let Some(msg) = self.get_message(seq) {
                        result.push(msg);
                    }
                }
            }
        }

        result
    }

    /// Get UID validity (constant for this implementation)
    pub fn uid_validity(&self) -> u32 {
        1 // Simple constant for now
    }

    /// Get next UID
    pub fn uid_next(&self) -> u32 {
        (self.messages.len() + 1) as u32
    }

    /// Search messages by criteria
    ///
    /// Returns sequence numbers of matching messages
    pub fn search(&self, criteria: &SearchCriteria) -> Result<Vec<usize>, MailError> {
        let mut matches = Vec::new();

        for msg in &self.messages {
            // Convert message content to string for searching
            let content_str = String::from_utf8_lossy(&msg.content);

            // Extract headers (everything before first empty line)
            let headers = if let Some(header_end) = content_str.find("\r\n\r\n") {
                &content_str[..header_end]
            } else {
                &content_str
            };

            // Check if message matches criteria
            let is_match = match criteria {
                SearchCriteria::All => true,

                SearchCriteria::Subject(query) => {
                    Self::extract_header(headers, "Subject:")
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query.to_lowercase())
                }

                SearchCriteria::From(query) => {
                    Self::extract_header(headers, "From:")
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query.to_lowercase())
                }

                SearchCriteria::To(query) => {
                    Self::extract_header(headers, "To:")
                        .unwrap_or_default()
                        .to_lowercase()
                        .contains(&query.to_lowercase())
                }

                SearchCriteria::Text(query) => {
                    // Search in entire message (headers + body)
                    content_str.to_lowercase().contains(&query.to_lowercase())
                }
            };

            if is_match {
                matches.push(msg.sequence);
            }
        }

        Ok(matches)
    }

    /// Store flags on messages
    ///
    /// Modifies message flags according to the operation (Add, Remove, Replace)
    /// Returns the list of modified sequence numbers
    pub fn store_flags(
        &mut self,
        sequence_set: &str,
        operation: &StoreOperation,
        flags: &[String],
    ) -> Result<Vec<usize>, MailError> {
        let mut modified_sequences = Vec::new();

        // Parse sequence set and get message indices
        for part in sequence_set.split(',') {
            let sequences = if part.contains(':') {
                // Range: "1:3" or "1:*"
                let parts: Vec<&str> = part.split(':').collect();
                if parts.len() == 2 {
                    let start = parts[0].parse::<usize>().unwrap_or(1);
                    let end = if parts[1] == "*" {
                        self.messages.len()
                    } else {
                        parts[1].parse::<usize>().unwrap_or(self.messages.len())
                    };
                    (start..=end.min(self.messages.len())).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            } else {
                // Single sequence: "1"
                if let Ok(seq) = part.parse::<usize>() {
                    vec![seq]
                } else {
                    vec![]
                }
            };

            // Modify flags for each message
            for seq in sequences {
                if seq > 0 && seq <= self.messages.len() {
                    let idx = seq - 1; // Convert to 0-indexed
                    let msg = &mut self.messages[idx];

                    match operation {
                        StoreOperation::Add => {
                            // Add flags that don't already exist
                            for flag in flags {
                                if !msg.flags.contains(flag) {
                                    msg.flags.push(flag.clone());
                                }
                            }
                        }
                        StoreOperation::Remove => {
                            // Remove flags
                            msg.flags.retain(|f| !flags.contains(f));
                        }
                        StoreOperation::Replace => {
                            // Replace all flags
                            msg.flags = flags.to_vec();
                        }
                    }

                    modified_sequences.push(seq);
                }
            }
        }

        Ok(modified_sequences)
    }

    /// Expunge messages marked with \Deleted flag
    ///
    /// Permanently removes messages marked as \Deleted from the mailbox
    /// Returns the list of expunged sequence numbers
    pub fn expunge(&mut self) -> Result<Vec<usize>, MailError> {
        let mut expunged_sequences = Vec::new();

        // Find all messages marked as \Deleted
        // Iterate in reverse order to avoid index issues when removing
        let mut idx = self.messages.len();
        while idx > 0 {
            idx -= 1;
            let msg = &self.messages[idx];

            if msg.flags.contains(&"\\Deleted".to_string()) {
                expunged_sequences.push(msg.sequence);
                self.messages.remove(idx);
            }
        }

        // Re-number remaining messages (sequences must be continuous from 1..N)
        for (idx, msg) in self.messages.iter_mut().enumerate() {
            msg.sequence = idx + 1;
        }

        // Reverse to return in ascending order
        expunged_sequences.reverse();

        Ok(expunged_sequences)
    }

    /// Copy messages to another mailbox
    ///
    /// Copies the specified messages to the destination mailbox
    /// Returns the number of messages copied
    pub fn copy_messages(
        &self,
        sequence_set: &str,
        destination: &str,
        email: &str,
        maildir_root: &Path,
    ) -> Result<usize, MailError> {
        let user_maildir = maildir_root.join(email);

        // Determine destination path
        let dest_path = if destination.to_uppercase() == "INBOX" {
            user_maildir.clone()
        } else {
            user_maildir.join(format!(".{}", destination))
        };

        // Ensure destination new/ directory exists
        let dest_new = dest_path.join("new");
        if !dest_new.exists() {
            fs::create_dir_all(&dest_new)?;
        }

        let mut copied_count = 0;

        // Parse sequence set and copy messages
        for part in sequence_set.split(',') {
            let sequences = if part.contains(':') {
                let parts: Vec<&str> = part.split(':').collect();
                if parts.len() == 2 {
                    let start = parts[0].parse::<usize>().unwrap_or(1);
                    let end = if parts[1] == "*" {
                        self.messages.len()
                    } else {
                        parts[1].parse::<usize>().unwrap_or(self.messages.len())
                    };
                    (start..=end.min(self.messages.len())).collect::<Vec<_>>()
                } else {
                    vec![]
                }
            } else {
                if let Ok(seq) = part.parse::<usize>() {
                    vec![seq]
                } else {
                    vec![]
                }
            };

            // Copy each message
            for seq in sequences {
                if seq > 0 && seq <= self.messages.len() {
                    let idx = seq - 1;
                    let msg = &self.messages[idx];

                    // Generate unique filename for copied message
                    let timestamp = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_micros();
                    let filename = format!("{}.{}.copy", timestamp, seq);
                    let dest_file = dest_new.join(&filename);

                    // Write message content to destination
                    fs::write(&dest_file, &msg.content)?;
                    copied_count += 1;
                }
            }
        }

        Ok(copied_count)
    }

    /// Helper: Extract header value from headers string
    fn extract_header(headers: &str, header_name: &str) -> Option<String> {
        for line in headers.lines() {
            if line.to_lowercase().starts_with(&header_name.to_lowercase()) {
                // Extract everything after "Header: "
                if let Some(value) = line.split_once(':') {
                    return Some(value.1.trim().to_string());
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_maildir() -> (TempDir, PathBuf) {
        let temp_dir = TempDir::new().unwrap();
        let maildir = temp_dir.path().join("test@example.com");
        let new_dir = maildir.join("new");

        fs::create_dir_all(&new_dir).unwrap();

        // Create test emails
        fs::write(new_dir.join("1.eml"), b"Subject: Test 1\r\n\r\nBody 1").unwrap();
        fs::write(new_dir.join("2.eml"), b"Subject: Test 2\r\n\r\nBody 2").unwrap();

        let root_path = temp_dir.path().to_path_buf();
        (temp_dir, root_path)
    }

    #[test]
    fn test_open_mailbox() {
        let (_temp, root) = setup_test_maildir();

        let mailbox = Mailbox::open("test@example.com", "INBOX", &root).unwrap();

        assert_eq!(mailbox.name, "INBOX");
        assert_eq!(mailbox.message_count(), 2);
    }

    #[test]
    fn test_get_message() {
        let (_temp, root) = setup_test_maildir();
        let mailbox = Mailbox::open("test@example.com", "INBOX", &root).unwrap();

        let msg = mailbox.get_message(1).unwrap();
        assert_eq!(msg.sequence, 1);
        assert!(msg.content.starts_with(b"Subject: Test"));
    }

    #[test]
    fn test_get_messages_range() {
        let (_temp, root) = setup_test_maildir();
        let mailbox = Mailbox::open("test@example.com", "INBOX", &root).unwrap();

        let messages = mailbox.get_messages("1:2");
        assert_eq!(messages.len(), 2);

        let messages = mailbox.get_messages("1:*");
        assert_eq!(messages.len(), 2);

        let messages = mailbox.get_messages("1");
        assert_eq!(messages.len(), 1);
    }
}
