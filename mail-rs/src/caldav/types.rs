//! CalDAV/CardDAV types and data structures

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Calendar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Calendar {
    /// Unique ID
    pub id: String,
    /// Owner email
    pub owner_email: String,
    /// Calendar name
    pub name: String,
    /// Display color
    pub color: Option<String>,
    /// CalDAV sync token
    pub sync_token: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Calendar event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalendarEvent {
    /// Unique ID
    pub id: String,
    /// Calendar ID
    pub calendar_id: String,
    /// iCalendar UID
    pub uid: String,
    /// Raw iCalendar data
    pub ics_data: String,
    /// Event summary/title
    pub summary: Option<String>,
    /// Start time
    pub dtstart: Option<DateTime<Utc>>,
    /// End time
    pub dtend: Option<DateTime<Utc>>,
    /// ETag for sync
    pub etag: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Address book
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressBook {
    /// Unique ID
    pub id: String,
    /// Owner email
    pub owner_email: String,
    /// Address book name
    pub name: String,
    /// CardDAV sync token
    pub sync_token: Option<String>,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Contact
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    /// Unique ID
    pub id: String,
    /// Address book ID
    pub addressbook_id: String,
    /// vCard UID
    pub uid: String,
    /// Raw vCard data
    pub vcf_data: String,
    /// Formatted name (FN)
    pub fn_name: Option<String>,
    /// Primary email
    pub email: Option<String>,
    /// ETag for sync
    pub etag: String,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last update timestamp
    pub updated_at: DateTime<Utc>,
}

/// Create calendar request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCalendarRequest {
    /// Calendar name
    pub name: String,
    /// Display color (hex)
    pub color: Option<String>,
}

/// Create event request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateEventRequest {
    /// Event summary/title
    pub summary: String,
    /// Start time
    pub dtstart: DateTime<Utc>,
    /// End time
    pub dtend: DateTime<Utc>,
    /// Description
    pub description: Option<String>,
    /// Location
    pub location: Option<String>,
}

/// Create contact request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateContactRequest {
    /// Full name
    pub full_name: String,
    /// Email address
    pub email: Option<String>,
    /// Phone number
    pub phone: Option<String>,
}

/// Create address book request
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAddressBookRequest {
    /// Address book name
    pub name: String,
}

/// CalDAV/CardDAV statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CalDavStats {
    /// Total calendars
    pub total_calendars: u64,
    /// Total events
    pub total_events: u64,
    /// Total address books
    pub total_addressbooks: u64,
    /// Total contacts
    pub total_contacts: u64,
}

/// Import ICS/VCF request
#[derive(Debug, Clone, Deserialize)]
pub struct ImportDataRequest {
    /// Raw data (ICS or VCF)
    pub data: String,
}
