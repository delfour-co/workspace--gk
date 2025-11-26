//! IMAP command parsing
//!
//! IMAP commands have the format: `tag COMMAND arguments`
//! Example: A001 LOGIN john password

use crate::error::MailError;

/// Search criteria for IMAP SEARCH command
#[derive(Debug, Clone, PartialEq)]
pub enum SearchCriteria {
    /// ALL - All messages
    All,

    /// SUBJECT string - Messages with string in subject
    Subject(String),

    /// FROM string - Messages from sender
    From(String),

    /// TO string - Messages to recipient
    To(String),

    /// TEXT string - Messages with string in body or headers
    Text(String),
}

/// Store operation type
#[derive(Debug, Clone, PartialEq)]
pub enum StoreOperation {
    /// +FLAGS - Add flags to message
    Add,
    /// -FLAGS - Remove flags from message
    Remove,
    /// FLAGS - Replace all flags
    Replace,
}

/// IMAP command parsed from client
#[derive(Debug, Clone, PartialEq)]
pub enum ImapCommand {
    /// CAPABILITY - List server capabilities
    Capability,

    /// LOGIN username password - Authenticate
    Login { username: String, password: String },

    /// SELECT mailbox - Select a mailbox
    Select { mailbox: String },

    /// EXAMINE mailbox - Select mailbox in read-only mode
    Examine { mailbox: String },

    /// FETCH sequence items - Retrieve message data
    Fetch {
        sequence: String,
        items: Vec<String>,
    },

    /// LIST reference mailbox - List mailboxes
    List {
        reference: String,
        mailbox: String,
    },

    /// SEARCH criteria - Search for messages
    Search { criteria: SearchCriteria },

    /// STORE sequence operation flags - Modify message flags
    Store {
        sequence: String,
        operation: StoreOperation,
        flags: Vec<String>,
    },

    /// EXPUNGE - Permanently remove messages marked \Deleted
    Expunge,

    /// COPY sequence destination - Copy messages to another mailbox
    Copy {
        sequence: String,
        mailbox: String,
    },

    /// IDLE - Wait for server notifications
    Idle,

    /// DONE - End IDLE mode (sent without tag)
    Done,

    /// LOGOUT - Close connection
    Logout,

    /// NOOP - No operation (keepalive)
    Noop,
}

impl ImapCommand {
    /// Parse an IMAP command line
    ///
    /// Format: `tag COMMAND arguments\r\n`
    /// Special case: `DONE` has no tag (used to end IDLE mode)
    pub fn parse(line: &str) -> Result<(String, Self), MailError> {
        let line = line.trim();

        // Special case: DONE has no tag (used to end IDLE mode)
        if line.to_uppercase() == "DONE" {
            return Ok(("".to_string(), ImapCommand::Done));
        }

        // Split into parts: tag command args...
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() < 2 {
            return Err(MailError::ImapProtocol(
                "Invalid IMAP command format".to_string(),
            ));
        }

        let tag = parts[0].to_string();
        let command = parts[1].to_uppercase();

        let cmd = match command.as_str() {
            "CAPABILITY" => ImapCommand::Capability,

            "LOGIN" => {
                if parts.len() < 4 {
                    return Err(MailError::ImapProtocol(
                        "LOGIN requires username and password".to_string(),
                    ));
                }

                // Handle quoted strings with spaces
                // Find username (after tag and LOGIN)
                let after_login = line.split_whitespace().skip(2).collect::<Vec<_>>().join(" ");
                let (username, password) = Self::parse_login_credentials(&after_login)?;

                ImapCommand::Login { username, password }
            }

            "SELECT" => {
                if parts.len() < 3 {
                    return Err(MailError::ImapProtocol(
                        "SELECT requires mailbox name".to_string(),
                    ));
                }
                ImapCommand::Select {
                    mailbox: parts[2].trim_matches('"').to_string(),
                }
            }

            "EXAMINE" => {
                if parts.len() < 3 {
                    return Err(MailError::ImapProtocol(
                        "EXAMINE requires mailbox name".to_string(),
                    ));
                }
                ImapCommand::Examine {
                    mailbox: parts[2].trim_matches('"').to_string(),
                }
            }

            "FETCH" => {
                if parts.len() < 4 {
                    return Err(MailError::ImapProtocol(
                        "FETCH requires sequence and items".to_string(),
                    ));
                }
                let sequence = parts[2].to_string();
                let items = parts[3..].iter().map(|s| s.to_string()).collect();

                ImapCommand::Fetch { sequence, items }
            }

            "LIST" => {
                let reference = if parts.len() > 2 {
                    parts[2].trim_matches('"').to_string()
                } else {
                    String::new()
                };

                let mailbox = if parts.len() > 3 {
                    parts[3].trim_matches('"').to_string()
                } else {
                    "*".to_string()
                };

                ImapCommand::List { reference, mailbox }
            }

            "SEARCH" => {
                if parts.len() < 3 {
                    return Err(MailError::ImapProtocol(
                        "SEARCH requires criteria".to_string(),
                    ));
                }

                let criterion = parts[2].to_uppercase();
                let criteria = match criterion.as_str() {
                    "ALL" => SearchCriteria::All,
                    "SUBJECT" => {
                        if parts.len() < 4 {
                            return Err(MailError::ImapProtocol(
                                "SUBJECT requires a search string".to_string(),
                            ));
                        }
                        let query = parts[3..].join(" ").trim_matches('"').to_string();
                        SearchCriteria::Subject(query)
                    }
                    "FROM" => {
                        if parts.len() < 4 {
                            return Err(MailError::ImapProtocol(
                                "FROM requires a search string".to_string(),
                            ));
                        }
                        let query = parts[3..].join(" ").trim_matches('"').to_string();
                        SearchCriteria::From(query)
                    }
                    "TO" => {
                        if parts.len() < 4 {
                            return Err(MailError::ImapProtocol(
                                "TO requires a search string".to_string(),
                            ));
                        }
                        let query = parts[3..].join(" ").trim_matches('"').to_string();
                        SearchCriteria::To(query)
                    }
                    "TEXT" => {
                        if parts.len() < 4 {
                            return Err(MailError::ImapProtocol(
                                "TEXT requires a search string".to_string(),
                            ));
                        }
                        let query = parts[3..].join(" ").trim_matches('"').to_string();
                        SearchCriteria::Text(query)
                    }
                    _ => {
                        return Err(MailError::ImapProtocol(format!(
                            "Unknown search criterion: {}",
                            criterion
                        )))
                    }
                };

                ImapCommand::Search { criteria }
            }

            "STORE" => {
                if parts.len() < 5 {
                    return Err(MailError::ImapProtocol(
                        "STORE requires sequence, operation, and flags".to_string(),
                    ));
                }

                let sequence = parts[2].to_string();
                let operation_str = parts[3].to_uppercase();

                // Determine operation type
                let operation = match operation_str.as_str() {
                    "+FLAGS" => StoreOperation::Add,
                    "-FLAGS" => StoreOperation::Remove,
                    "FLAGS" => StoreOperation::Replace,
                    _ => {
                        return Err(MailError::ImapProtocol(format!(
                            "Unknown STORE operation: {}",
                            operation_str
                        )))
                    }
                };

                // Parse flags - they're typically in parentheses: (\Deleted \Seen)
                // or sometimes without: \Deleted
                let flags_str = parts[4..].join(" ");
                let flags = Self::parse_flags(&flags_str)?;

                ImapCommand::Store {
                    sequence,
                    operation,
                    flags,
                }
            }

            "EXPUNGE" => ImapCommand::Expunge,

            "COPY" => {
                if parts.len() < 4 {
                    return Err(MailError::ImapProtocol(
                        "COPY requires sequence and destination mailbox".to_string(),
                    ));
                }

                let sequence = parts[2].to_string();
                let mailbox = parts[3].trim_matches('"').to_string();

                ImapCommand::Copy { sequence, mailbox }
            }

            "IDLE" => ImapCommand::Idle,

            "LOGOUT" => ImapCommand::Logout,

            "NOOP" => ImapCommand::Noop,

            _ => {
                return Err(MailError::ImapProtocol(format!(
                    "Unknown IMAP command: {}",
                    command
                )))
            }
        };

        Ok((tag, cmd))
    }

    /// Parse LOGIN credentials handling quoted strings
    fn parse_login_credentials(input: &str) -> Result<(String, String), MailError> {
        let input = input.trim();

        // Try to parse quoted strings first
        if input.starts_with('"') {
            // Find end of first quoted string
            if let Some(first_end) = input[1..].find('"') {
                let username = input[1..first_end + 1].to_string();
                let remaining = input[first_end + 2..].trim();

                // Parse password
                if remaining.starts_with('"') {
                    if let Some(second_end) = remaining[1..].find('"') {
                        let password = remaining[1..second_end + 1].to_string();
                        return Ok((username, password));
                    }
                }
            }
        }

        // Fallback to simple whitespace split for unquoted credentials
        let parts: Vec<&str> = input.split_whitespace().collect();
        if parts.len() >= 2 {
            Ok((parts[0].to_string(), parts[1].to_string()))
        } else {
            Err(MailError::ImapProtocol(
                "Invalid LOGIN credentials format".to_string(),
            ))
        }
    }

    /// Parse flags from string, handling both "(flag1 flag2)" and "flag1" formats
    fn parse_flags(input: &str) -> Result<Vec<String>, MailError> {
        let input = input.trim();

        // Remove parentheses if present: (\Deleted \Seen) -> \Deleted \Seen
        let flags_str = if input.starts_with('(') && input.ends_with(')') {
            &input[1..input.len() - 1]
        } else {
            input
        };

        // Split by whitespace and collect flags
        let flags: Vec<String> = flags_str
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        if flags.is_empty() {
            return Err(MailError::ImapProtocol(
                "No flags provided".to_string(),
            ));
        }

        Ok(flags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_capability() {
        let (tag, cmd) = ImapCommand::parse("A001 CAPABILITY").unwrap();
        assert_eq!(tag, "A001");
        assert_eq!(cmd, ImapCommand::Capability);
    }

    #[test]
    fn test_parse_login() {
        let (tag, cmd) = ImapCommand::parse("A001 LOGIN john secret").unwrap();
        assert_eq!(tag, "A001");
        assert_eq!(
            cmd,
            ImapCommand::Login {
                username: "john".to_string(),
                password: "secret".to_string()
            }
        );
    }

    #[test]
    fn test_parse_login_quoted() {
        let (tag, cmd) = ImapCommand::parse(r#"A001 LOGIN "john" "my password""#).unwrap();
        assert_eq!(tag, "A001");
        assert_eq!(
            cmd,
            ImapCommand::Login {
                username: "john".to_string(),
                password: "my password".to_string()
            }
        );
    }

    #[test]
    fn test_parse_select() {
        let (tag, cmd) = ImapCommand::parse("A002 SELECT INBOX").unwrap();
        assert_eq!(tag, "A002");
        assert_eq!(
            cmd,
            ImapCommand::Select {
                mailbox: "INBOX".to_string()
            }
        );
    }

    #[test]
    fn test_parse_fetch() {
        let (tag, cmd) = ImapCommand::parse("A003 FETCH 1 BODY[]").unwrap();
        assert_eq!(tag, "A003");
        assert!(matches!(cmd, ImapCommand::Fetch { .. }));
    }

    #[test]
    fn test_parse_logout() {
        let (tag, cmd) = ImapCommand::parse("A004 LOGOUT").unwrap();
        assert_eq!(tag, "A004");
        assert_eq!(cmd, ImapCommand::Logout);
    }

    #[test]
    fn test_parse_search_all() {
        let (tag, cmd) = ImapCommand::parse("A005 SEARCH ALL").unwrap();
        assert_eq!(tag, "A005");
        assert_eq!(
            cmd,
            ImapCommand::Search {
                criteria: SearchCriteria::All
            }
        );
    }

    #[test]
    fn test_parse_search_subject() {
        let (tag, cmd) = ImapCommand::parse("A006 SEARCH SUBJECT hello").unwrap();
        assert_eq!(tag, "A006");
        assert_eq!(
            cmd,
            ImapCommand::Search {
                criteria: SearchCriteria::Subject("hello".to_string())
            }
        );
    }

    #[test]
    fn test_parse_search_subject_quoted() {
        let (tag, cmd) = ImapCommand::parse(r#"A007 SEARCH SUBJECT "test email""#).unwrap();
        assert_eq!(tag, "A007");
        assert_eq!(
            cmd,
            ImapCommand::Search {
                criteria: SearchCriteria::Subject("test email".to_string())
            }
        );
    }

    #[test]
    fn test_parse_search_from() {
        let (tag, cmd) = ImapCommand::parse("A008 SEARCH FROM alice@example.com").unwrap();
        assert_eq!(tag, "A008");
        assert_eq!(
            cmd,
            ImapCommand::Search {
                criteria: SearchCriteria::From("alice@example.com".to_string())
            }
        );
    }

    #[test]
    fn test_parse_search_text() {
        let (tag, cmd) = ImapCommand::parse("A009 SEARCH TEXT meeting").unwrap();
        assert_eq!(tag, "A009");
        assert_eq!(
            cmd,
            ImapCommand::Search {
                criteria: SearchCriteria::Text("meeting".to_string())
            }
        );
    }
}
