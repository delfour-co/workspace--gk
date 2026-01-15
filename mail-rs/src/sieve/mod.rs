//! Sieve email filtering module (RFC 5228)
//!
//! Provides server-side email filtering rules.

pub mod executor;
pub mod manager;
pub mod parser;
pub mod types;

pub use executor::SieveExecutor;
pub use manager::SieveManager;
pub use parser::{parse_script, validate_script};
pub use types::*;
