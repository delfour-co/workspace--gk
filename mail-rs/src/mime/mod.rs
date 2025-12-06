/// MIME message parsing and handling
///
/// This module provides functionality to parse MIME multipart messages
/// and extract attachments.

pub mod parser;
pub mod types;

pub use parser::MimeParser;
pub use types::{MimePart, ParsedEmail};
