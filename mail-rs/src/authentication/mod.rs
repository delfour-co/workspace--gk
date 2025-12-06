/// Email authentication module (SPF, DKIM, DMARC)
///
/// This module provides email authentication mechanisms to verify
/// sender identity and prevent spam/spoofing.

pub mod spf;
pub mod dkim;
pub mod dmarc;
pub mod types;

pub use spf::{SpfValidator, SpfResult};
pub use dkim::{DkimSigner, DkimValidator, DkimResult};
pub use dmarc::{DmarcValidator, DmarcResult, DmarcPolicy};
pub use types::{AuthenticationResults, AuthenticationStatus};
