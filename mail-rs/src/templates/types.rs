//! Template types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Email template with support for variables and categories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailTemplate {
    /// Unique identifier
    pub id: String,
    /// Template name
    pub name: String,
    /// Category (signature, quick reply, custom)
    pub category: TemplateCategory,
    /// Email subject (optional for signatures)
    pub subject: String,
    /// HTML body with template variables
    pub body_html: String,
    /// Plain text body
    pub body_text: String,
    /// Custom variables defined in this template
    pub variables: Vec<TemplateVariable>,
    /// True if this is a signature template
    pub is_signature: bool,
    /// Owner's email address
    pub owner_email: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Template category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TemplateCategory {
    /// Automatic signature appended to emails
    Signature,
    /// Quick reply template
    QuickReply,
    /// Custom user-defined template
    Custom,
}

impl TemplateCategory {
    /// Convert to database string
    pub fn to_db_string(&self) -> &'static str {
        match self {
            TemplateCategory::Signature => "signature",
            TemplateCategory::QuickReply => "quick_reply",
            TemplateCategory::Custom => "custom",
        }
    }

    /// Parse from database string
    pub fn from_db_string(s: &str) -> Option<Self> {
        match s {
            "signature" => Some(TemplateCategory::Signature),
            "quick_reply" => Some(TemplateCategory::QuickReply),
            "custom" => Some(TemplateCategory::Custom),
            _ => None,
        }
    }
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name (without {{ }})
    pub name: String,
    /// Default value if not provided
    pub default_value: Option<String>,
    /// Whether this variable must be provided
    pub required: bool,
}

/// Request to create a new template
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub category: TemplateCategory,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub variables: Vec<TemplateVariable>,
    pub is_signature: bool,
}

/// Request to update an existing template
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub variables: Option<Vec<TemplateVariable>>,
    pub is_signature: Option<bool>,
}
