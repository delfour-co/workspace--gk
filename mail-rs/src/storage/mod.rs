//! Email storage module
//!
//! Provides email storage backends:
//! - [`maildir`]: Maildir format storage with atomic operations

pub mod maildir;

pub use maildir::MaildirStorage;
