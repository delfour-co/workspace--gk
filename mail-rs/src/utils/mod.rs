pub mod dkim;
pub mod dkim_signer;
pub mod dmarc;
pub mod dns;
pub mod dns_validator;
pub mod email;
pub mod spf;

pub use email::validate_email;
