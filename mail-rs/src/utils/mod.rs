pub mod dkim;
pub mod dns;
pub mod email;
pub mod spf;

pub use email::validate_email;
