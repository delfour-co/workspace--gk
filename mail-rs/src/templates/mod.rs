//! Email templates system
//!
//! Provides signature management, quick replies, and custom email templates
//! with variable substitution.

pub mod manager;
pub mod renderer;
pub mod types;

pub use manager::TemplateManager;
pub use renderer::TemplateRenderer;
pub use types::{EmailTemplate, TemplateCategory, TemplateVariable};
