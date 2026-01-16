//! Security module
//!
//! Provides authentication, rate limiting, and TLS functionality:
//! - [`auth`]: SMTP authentication mechanisms (LOGIN, PLAIN)
//! - [`rate_limit`]: Connection and request rate limiting
//! - [`tls`]: TLS/STARTTLS configuration and handling

pub mod auth;
pub mod rate_limit;
pub mod tls;

pub use auth::{AuthMechanism, Authenticator};
pub use rate_limit::{RateLimit, RateLimiter};
pub use tls::TlsConfig;
