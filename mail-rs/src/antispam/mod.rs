/// Anti-spam module
///
/// Provides greylisting and whitelist/blacklist management

pub mod greylist;
pub mod types;

pub use greylist::GreylistManager;
pub use types::{GreylistEntry, GreylistStatus, ListEntry};
