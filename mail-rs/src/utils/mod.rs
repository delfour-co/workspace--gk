//! Utility modules
//!
//! Provides various utility functions and email authentication:
//! - [`dkim`]: DKIM signature validation (RFC 6376)
//! - [`dkim_signer`]: DKIM signing for outgoing emails
//! - [`dmarc`]: DMARC policy checking (RFC 7489)
//! - [`dns`]: DNS lookup utilities
//! - [`dns_validator`]: DNS record validation
//! - [`email`]: Email address validation (RFC 5321)
//! - [`spf`]: SPF validation (RFC 7208)

pub mod dkim;
pub mod dkim_signer;
pub mod dmarc;
pub mod dns;
pub mod dns_validator;
pub mod email;
pub mod spf;

pub use email::validate_email;
