//! Sieve types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Sieve script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveScript {
    /// Unique ID
    pub id: String,
    /// Owner email
    pub owner_email: String,
    /// Script name
    pub name: String,
    /// Script content (raw Sieve syntax)
    pub script_content: String,
    /// Compiled rules
    #[serde(skip)]
    pub rules: Option<Vec<SieveRule>>,
    /// Whether this is the active script
    pub is_active: bool,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// A single Sieve rule (if condition then action)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveRule {
    /// Condition to evaluate
    pub condition: SieveCondition,
    /// Actions to take if condition matches
    pub actions: Vec<SieveAction>,
    /// Optional elsif conditions
    pub elsif_branches: Vec<(SieveCondition, Vec<SieveAction>)>,
    /// Optional else actions
    pub else_actions: Option<Vec<SieveAction>>,
}

/// Sieve condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SieveCondition {
    /// Always true
    True,
    /// Always false
    False,
    /// Logical NOT
    Not(Box<SieveCondition>),
    /// All conditions must match (AND)
    AllOf(Vec<SieveCondition>),
    /// Any condition must match (OR)
    AnyOf(Vec<SieveCondition>),
    /// Header test
    Header(HeaderTest),
    /// Address test
    Address(AddressTest),
    /// Size test
    Size(SizeTest),
    /// Exists test (header exists)
    Exists(Vec<String>),
}

/// Header test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderTest {
    /// Headers to test
    pub headers: Vec<String>,
    /// Values to compare against
    pub values: Vec<String>,
    /// Match type
    pub match_type: MatchType,
    /// Comparator (default: i;ascii-casemap)
    pub comparator: Comparator,
}

/// Address test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressTest {
    /// Headers to test (From, To, Cc, etc.)
    pub headers: Vec<String>,
    /// Values to compare against
    pub values: Vec<String>,
    /// Match type
    pub match_type: MatchType,
    /// Address part to compare
    pub address_part: AddressPart,
    /// Comparator
    pub comparator: Comparator,
}

/// Size test configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SizeTest {
    /// Size comparison operator
    pub comparison: SizeComparison,
    /// Size in bytes
    pub size: u64,
}

/// Size comparison operators
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SizeComparison {
    /// Over the specified size
    Over,
    /// Under the specified size
    Under,
}

/// Match type for tests
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    /// Exact match
    Is,
    /// Contains substring
    Contains,
    /// Wildcard match
    Matches,
    /// Regex match
    Regex,
}

/// Address part to extract
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AddressPart {
    /// Full address
    All,
    /// Local part (before @)
    LocalPart,
    /// Domain part (after @)
    Domain,
}

/// Comparator for string comparison
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Comparator {
    /// ASCII case-insensitive (default)
    AsciiCasemap,
    /// Octet (byte-by-byte)
    Octet,
}

impl Default for Comparator {
    fn default() -> Self {
        Self::AsciiCasemap
    }
}

impl Default for MatchType {
    fn default() -> Self {
        Self::Is
    }
}

impl Default for AddressPart {
    fn default() -> Self {
        Self::All
    }
}

/// Sieve action to take on a message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SieveAction {
    /// Keep the message (default)
    Keep,
    /// Discard the message
    Discard,
    /// File into a specific folder
    FileInto(String),
    /// Redirect to another address
    Redirect(String),
    /// Mark with flags
    Flag(Vec<String>),
    /// Stop processing rules
    Stop,
    /// Send vacation reply
    Vacation(VacationConfig),
}

/// Vacation auto-reply configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VacationConfig {
    /// Subject line
    pub subject: String,
    /// Message body
    pub body: String,
    /// Reply interval in days
    pub days: u32,
    /// Addresses to respond to
    pub addresses: Vec<String>,
}

/// Sieve execution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveResult {
    /// Actions to take
    pub actions: Vec<SieveAction>,
    /// Whether implicit keep is enabled
    pub implicit_keep: bool,
}

impl Default for SieveResult {
    fn default() -> Self {
        Self {
            actions: vec![],
            implicit_keep: true,
        }
    }
}

/// Message context for Sieve evaluation
#[derive(Debug, Clone)]
pub struct MessageContext {
    /// From header
    pub from: String,
    /// To headers
    pub to: Vec<String>,
    /// Cc headers
    pub cc: Vec<String>,
    /// Subject
    pub subject: String,
    /// All headers (name, value pairs)
    pub headers: Vec<(String, String)>,
    /// Message body
    pub body: String,
    /// Message size in bytes
    pub size: u64,
}

/// Sieve execution log
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SieveLog {
    /// Unique ID
    pub id: String,
    /// Owner email
    pub owner_email: String,
    /// Script ID that was executed
    pub script_id: String,
    /// Message ID processed
    pub message_id: String,
    /// Action taken
    pub action_taken: String,
    /// Execution timestamp
    pub executed_at: DateTime<Utc>,
}

/// API request to create/update a Sieve script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSieveScriptRequest {
    /// Script name
    pub name: String,
    /// Script content
    pub script_content: String,
    /// Whether to activate this script
    pub activate: bool,
}

/// API request to validate a Sieve script
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateSieveScriptRequest {
    /// Script content to validate
    pub script_content: String,
}

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Whether the script is valid
    pub valid: bool,
    /// Error message if invalid
    pub error: Option<String>,
    /// Warnings
    pub warnings: Vec<String>,
}
