/// Quota management for users
///
/// This module provides quota enforcement for:
/// - Storage limits per user
/// - Message count limits per day
/// - Message size limits

pub mod manager;
pub mod types;

pub use manager::QuotaManager;
pub use types::{UserQuota, QuotaStatus};
