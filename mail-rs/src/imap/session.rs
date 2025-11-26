//! IMAP session management
//!
//! Handles IMAP protocol state machine and command execution

use crate::error::MailError;
use crate::imap::{ImapCommand, Mailbox, SearchCriteria, StoreOperation};
use crate::security::Authenticator;
use std::path::Path;
use tracing::{debug, info};

/// IMAP session states
#[derive(Debug, Clone, PartialEq)]
pub enum SessionState {
    /// Not authenticated
    NotAuthenticated,
    /// Authenticated but no mailbox selected
    Authenticated { username: String },
    /// Mailbox selected
    Selected {
        username: String,
        mailbox: String,
    },
    /// Logout state
    Logout,
}

/// IMAP session
pub struct ImapSession {
    /// Current state
    state: SessionState,
    /// Authenticator for verifying credentials
    authenticator: Authenticator,
    /// Maildir root path
    maildir_root: String,
    /// Currently selected mailbox (if any)
    current_mailbox: Option<Mailbox>,
    /// IDLE mode tag (if in IDLE mode)
    idle_tag: Option<String>,
}

impl ImapSession {
    /// Create a new IMAP session
    pub fn new(authenticator: Authenticator, maildir_root: String) -> Self {
        Self {
            state: SessionState::NotAuthenticated,
            authenticator,
            maildir_root,
            current_mailbox: None,
            idle_tag: None,
        }
    }

    /// Check if session is in IDLE mode
    pub fn is_idle(&self) -> bool {
        self.idle_tag.is_some()
    }

    /// Get current state
    pub fn state(&self) -> &SessionState {
        &self.state
    }

    /// Handle a command and return response
    pub async fn handle_command(
        &mut self,
        tag: String,
        command: ImapCommand,
    ) -> Result<String, MailError> {
        debug!("Handling IMAP command: {:?} in state {:?}", command, self.state);

        match (&self.state, &command) {
            // CAPABILITY - allowed in any state
            (_, ImapCommand::Capability) => Ok(self.handle_capability(tag)),

            // LOGIN - only in NotAuthenticated state
            (SessionState::NotAuthenticated, ImapCommand::Login { username, password }) => {
                self.handle_login(tag, username, password).await
            }

            // SELECT/EXAMINE - only in Authenticated or Selected state
            (SessionState::Authenticated { .. }, ImapCommand::Select { mailbox })
            | (SessionState::Selected { .. }, ImapCommand::Select { mailbox }) => {
                self.handle_select(tag, mailbox).await
            }

            (SessionState::Authenticated { .. }, ImapCommand::Examine { mailbox })
            | (SessionState::Selected { .. }, ImapCommand::Examine { mailbox }) => {
                self.handle_examine(tag, mailbox).await
            }

            // FETCH - only in Selected state
            (SessionState::Selected { .. }, ImapCommand::Fetch { sequence, items }) => {
                self.handle_fetch(tag, sequence, items)
            }

            // SEARCH - only in Selected state
            (SessionState::Selected { .. }, ImapCommand::Search { criteria }) => {
                self.handle_search(tag, criteria)
            }

            // STORE - only in Selected state
            (SessionState::Selected { .. }, ImapCommand::Store { sequence, operation, flags }) => {
                self.handle_store(tag, sequence, operation, flags)
            }

            // EXPUNGE - only in Selected state
            (SessionState::Selected { .. }, ImapCommand::Expunge) => {
                self.handle_expunge(tag)
            }

            // COPY - only in Selected state
            (SessionState::Selected { username, .. }, ImapCommand::Copy { sequence, mailbox }) => {
                self.handle_copy(tag, sequence, mailbox, username)
            }

            // IDLE - only in Selected state
            (SessionState::Selected { .. }, ImapCommand::Idle) => {
                self.handle_idle(tag)
            }

            // DONE - end IDLE mode (handled specially since it has no tag)
            (_, ImapCommand::Done) => {
                self.handle_done()
            }

            // LIST - in Authenticated or Selected state
            (SessionState::Authenticated { .. }, ImapCommand::List { reference, mailbox })
            | (SessionState::Selected { .. }, ImapCommand::List { reference, mailbox }) => {
                Ok(self.handle_list(tag, reference, mailbox))
            }

            // NOOP - allowed in any state except Logout
            (SessionState::Logout, ImapCommand::Noop) => {
                Ok(format!("{} BAD Command not allowed in LOGOUT state\r\n", tag))
            }
            (_, ImapCommand::Noop) => Ok(format!("{} OK NOOP completed\r\n", tag)),

            // LOGOUT - allowed in any state
            (_, ImapCommand::Logout) => Ok(self.handle_logout(tag)),

            // Invalid state/command combinations
            _ => Ok(format!(
                "{} BAD Command not allowed in current state\r\n",
                tag
            )),
        }
    }

    /// Handle CAPABILITY command
    fn handle_capability(&self, tag: String) -> String {
        format!(
            "* CAPABILITY IMAP4rev1 LOGIN\r\n{} OK CAPABILITY completed\r\n",
            tag
        )
    }

    /// Handle LOGIN command
    async fn handle_login(
        &mut self,
        tag: String,
        username: &str,
        password: &str,
    ) -> Result<String, MailError> {
        info!("LOGIN attempt for user: {}", username);

        // Verify credentials
        match self.authenticator.verify_login(username, password).await {
            Ok(true) => {
                info!("LOGIN successful for: {}", username);
                self.state = SessionState::Authenticated {
                    username: username.to_string(),
                };
                Ok(format!("{} OK LOGIN completed\r\n", tag))
            }
            Ok(false) => {
                info!("LOGIN failed for: {} (invalid credentials)", username);
                Ok(format!(
                    "{} NO LOGIN failed - invalid credentials\r\n",
                    tag
                ))
            }
            Err(e) => {
                info!("LOGIN error for {}: {}", username, e);
                Ok(format!("{} NO LOGIN failed - {}\r\n", tag, e))
            }
        }
    }

    /// Handle SELECT command
    async fn handle_select(&mut self, tag: String, mailbox: &str) -> Result<String, MailError> {
        let username = match &self.state {
            SessionState::Authenticated { username } | SessionState::Selected { username, .. } => {
                username.clone()
            }
            _ => return Ok(format!("{} BAD Not authenticated\r\n", tag)),
        };

        info!("SELECT {} for user {}", mailbox, username);

        // Open mailbox
        match Mailbox::open(&username, mailbox, Path::new(&self.maildir_root)) {
            Ok(mb) => {
                let exists = mb.message_count();
                let recent = mb.recent_count();
                let unseen = mb.first_unseen().unwrap_or(0);
                let uidvalidity = mb.uid_validity();
                let uidnext = mb.uid_next();

                let mut response = String::new();
                response.push_str(&format!("* {} EXISTS\r\n", exists));
                response.push_str(&format!("* {} RECENT\r\n", recent));
                response.push_str("* OK [UIDVALIDITY ");
                response.push_str(&uidvalidity.to_string());
                response.push_str("] UIDs valid\r\n");
                response.push_str("* OK [UIDNEXT ");
                response.push_str(&uidnext.to_string());
                response.push_str("] Predicted next UID\r\n");
                if unseen > 0 {
                    response.push_str(&format!("* OK [UNSEEN {}] First unseen\r\n", unseen));
                }
                response.push_str("* FLAGS (\\Seen \\Answered \\Flagged \\Deleted \\Draft)\r\n");
                response.push_str(&format!("{} OK [READ-WRITE] SELECT completed\r\n", tag));

                self.current_mailbox = Some(mb);
                self.state = SessionState::Selected {
                    username,
                    mailbox: mailbox.to_string(),
                };

                Ok(response)
            }
            Err(e) => Ok(format!("{} NO SELECT failed - {}\r\n", tag, e)),
        }
    }

    /// Handle EXAMINE command (read-only SELECT)
    async fn handle_examine(&mut self, tag: String, mailbox: &str) -> Result<String, MailError> {
        // For now, same as SELECT but with READ-ONLY flag
        let mut response = self.handle_select(tag.clone(), mailbox).await?;
        response = response.replace("READ-WRITE", "READ-ONLY");
        Ok(response)
    }

    /// Handle FETCH command
    fn handle_fetch(&self, tag: String, sequence: &str, items: &[String]) -> Result<String, MailError> {
        let mailbox = match &self.current_mailbox {
            Some(mb) => mb,
            None => return Ok(format!("{} BAD No mailbox selected\r\n", tag)),
        };

        let messages = mailbox.get_messages(sequence);
        let mut response = String::new();

        for msg in messages {
            response.push_str(&format!("* {} FETCH (", msg.sequence));

            // Parse fetch items
            let mut fetch_parts = Vec::new();

            for item in items {
                let item_upper = item.to_uppercase();
                if item_upper.contains("BODY[]") || item_upper == "RFC822" {
                    // Return full message
                    let body = String::from_utf8_lossy(&msg.content);
                    fetch_parts.push(format!("BODY[] {{{}}}\r\n{}", msg.size, body));
                } else if item_upper.contains("BODY[HEADER]") || item_upper == "RFC822.HEADER" {
                    // Return headers only
                    let body = String::from_utf8_lossy(&msg.content);
                    if let Some(header_end) = body.find("\r\n\r\n") {
                        let headers = &body[..header_end + 4];
                        fetch_parts.push(format!("BODY[HEADER] {{{}}}\r\n{}", headers.len(), headers));
                    }
                } else if item_upper == "RFC822.SIZE" {
                    fetch_parts.push(format!("RFC822.SIZE {}", msg.size));
                } else if item_upper == "UID" {
                    fetch_parts.push(format!("UID {}", msg.sequence));
                } else if item_upper == "FLAGS" {
                    let flags = msg.flags.join(" ");
                    fetch_parts.push(format!("FLAGS ({})", flags));
                }
            }

            response.push_str(&fetch_parts.join(" "));
            response.push_str(")\r\n");
        }

        response.push_str(&format!("{} OK FETCH completed\r\n", tag));
        Ok(response)
    }

    /// Handle SEARCH command
    fn handle_search(&self, tag: String, criteria: &SearchCriteria) -> Result<String, MailError> {
        let mailbox = match &self.current_mailbox {
            Some(mb) => mb,
            None => return Ok(format!("{} BAD No mailbox selected\r\n", tag)),
        };

        debug!("Searching with criteria: {:?}", criteria);

        // Get matching message sequence numbers
        let matches = mailbox.search(criteria)?;

        // Format response: "* SEARCH <sequence numbers>\r\n<tag> OK SEARCH completed\r\n"
        let mut response = String::from("* SEARCH");
        for seq in matches {
            response.push(' ');
            response.push_str(&seq.to_string());
        }
        response.push_str("\r\n");
        response.push_str(&format!("{} OK SEARCH completed\r\n", tag));

        Ok(response)
    }

    /// Handle STORE command
    fn handle_store(
        &mut self,
        tag: String,
        sequence: &str,
        operation: &StoreOperation,
        flags: &[String],
    ) -> Result<String, MailError> {
        let mailbox = match &mut self.current_mailbox {
            Some(mb) => mb,
            None => return Ok(format!("{} BAD No mailbox selected\r\n", tag)),
        };

        debug!("Storing flags {:?} on sequence {} with operation {:?}", flags, sequence, operation);

        // Modify flags on messages
        let modified_sequences = mailbox.store_flags(sequence, operation, flags)?;

        // Build response with FLAG updates for each modified message
        let mut response = String::new();
        for seq in &modified_sequences {
            if let Some(msg) = mailbox.get_message(*seq) {
                let flags_str = msg.flags.join(" ");
                response.push_str(&format!("* {} FETCH (FLAGS ({}))\r\n", seq, flags_str));
            }
        }
        response.push_str(&format!("{} OK STORE completed\r\n", tag));

        Ok(response)
    }

    /// Handle EXPUNGE command
    fn handle_expunge(&mut self, tag: String) -> Result<String, MailError> {
        let mailbox = match &mut self.current_mailbox {
            Some(mb) => mb,
            None => return Ok(format!("{} BAD No mailbox selected\r\n", tag)),
        };

        debug!("Expunging messages marked as \\Deleted");

        // Expunge messages marked as \Deleted
        let expunged_sequences = mailbox.expunge()?;

        // Build response with expunge notifications for each removed message
        let mut response = String::new();
        for seq in &expunged_sequences {
            response.push_str(&format!("* {} EXPUNGE\r\n", seq));
        }
        response.push_str(&format!("{} OK EXPUNGE completed\r\n", tag));

        Ok(response)
    }

    /// Handle COPY command
    fn handle_copy(
        &self,
        tag: String,
        sequence: &str,
        destination: &str,
        username: &str,
    ) -> Result<String, MailError> {
        let source_mailbox = match &self.current_mailbox {
            Some(mb) => mb,
            None => return Ok(format!("{} BAD No mailbox selected\r\n", tag)),
        };

        debug!("Copying messages {} to {}", sequence, destination);

        // Copy messages to destination
        let copied_count = source_mailbox.copy_messages(
            sequence,
            destination,
            username,
            Path::new(&self.maildir_root),
        )?;

        Ok(format!("{} OK COPY completed ({} messages)\r\n", tag, copied_count))
    }

    /// Handle IDLE command
    ///
    /// Puts the session in IDLE mode and returns a continuation response.
    /// The client should send DONE to exit IDLE mode.
    fn handle_idle(&mut self, tag: String) -> Result<String, MailError> {
        debug!("Entering IDLE mode");

        // Store the tag for when DONE is received
        self.idle_tag = Some(tag);

        // Return continuation response
        // Note: In a real implementation, we would start monitoring for
        // new messages and send EXISTS/RECENT notifications
        Ok("+ idling\r\n".to_string())
    }

    /// Handle DONE command (ends IDLE mode)
    fn handle_done(&mut self) -> Result<String, MailError> {
        debug!("Exiting IDLE mode");

        if let Some(tag) = self.idle_tag.take() {
            Ok(format!("{} OK IDLE terminated\r\n", tag))
        } else {
            Ok("* BAD Not in IDLE mode\r\n".to_string())
        }
    }

    /// Handle LIST command
    fn handle_list(&self, tag: String, _reference: &str, pattern: &str) -> String {
        let username = match &self.state {
            SessionState::Authenticated { username } | SessionState::Selected { username, .. } => {
                username.clone()
            }
            _ => return format!("{} BAD Not authenticated\r\n", tag),
        };

        // Get all available mailboxes
        let mailboxes = match Mailbox::list_mailboxes(&username, Path::new(&self.maildir_root)) {
            Ok(mboxes) => mboxes,
            Err(_) => vec!["INBOX".to_string()], // Fallback to INBOX only
        };

        let mut response = String::new();

        // Filter mailboxes based on pattern
        for mailbox in mailboxes {
            // Simple pattern matching: "*" matches all, exact name matches exact
            let matches = pattern == "*"
                || pattern.to_uppercase() == mailbox.to_uppercase()
                || pattern.is_empty();

            if matches {
                // Format: * LIST (flags) "hierarchy_delimiter" "mailbox_name"
                response.push_str(&format!("* LIST () \"/\" \"{}\"\r\n", mailbox));
            }
        }

        response.push_str(&format!("{} OK LIST completed\r\n", tag));
        response
    }

    /// Handle LOGOUT command
    fn handle_logout(&mut self, tag: String) -> String {
        info!("LOGOUT");
        self.state = SessionState::Logout;
        self.current_mailbox = None;
        format!("* BYE IMAP4rev1 Server logging out\r\n{} OK LOGOUT completed\r\n", tag)
    }
}
