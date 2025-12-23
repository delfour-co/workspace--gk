//! Auto-reply / Vacation responder module

pub mod manager;
pub mod types;

pub use manager::AutoReplyManager;
pub use types::{
    AutoReplyConfig, AutoReplySent, CreateAutoReplyRequest, UpdateAutoReplyRequest,
};
