//! IMAP server implementation
//!
//! This module provides a read-only IMAP server implementation
//! supporting basic commands: LOGIN, SELECT, FETCH, LOGOUT

pub mod commands;
pub mod mailbox;
pub mod server;
pub mod session;

pub use commands::{ImapCommand, SearchCriteria, StoreOperation};
pub use mailbox::Mailbox;
pub use server::ImapServer;
pub use session::{ImapSession, SessionState};
