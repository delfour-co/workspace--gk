//! IMAP server implementation
//!
//! This module provides a full-featured IMAP server implementation
//! supporting: LOGIN, SELECT, FETCH, SEARCH, STORE, COPY, EXPUNGE, IDLE

pub mod commands;
pub mod idle;
pub mod mailbox;
pub mod server;
pub mod session;

pub use commands::{ImapCommand, SearchCriteria, StoreOperation};
pub use idle::IdleWatcher;
pub use mailbox::Mailbox;
pub use server::ImapServer;
pub use session::{ImapSession, SessionState};
