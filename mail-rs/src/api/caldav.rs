//! CalDAV/CardDAV API endpoints
//!
//! REST API for calendar and contacts management.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::caldav::{
    CalDavManager, CalDavStats, Calendar, CalendarEvent, Contact, AddressBook,
    CreateCalendarRequest, CreateEventRequest, CreateContactRequest, CreateAddressBookRequest,
    ImportDataRequest,
};

/// CalDAV API state
pub struct CalDavState {
    pub manager: Arc<CalDavManager>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

// ==================== STATISTICS ====================

/// Get CalDAV/CardDAV statistics
pub async fn get_stats(
    State(state): State<Arc<CalDavState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<CalDavStats>>, StatusCode> {
    let email = params.get("email").map(|s| s.as_str());
    match state.manager.get_stats(email).await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get stats: {}", e)))),
    }
}

// ==================== CALENDAR ENDPOINTS ====================

/// List calendars for a user
pub async fn list_calendars(
    State(state): State<Arc<CalDavState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<Calendar>>>, StatusCode> {
    let email = match params.get("email") {
        Some(e) => e,
        None => return Ok(Json(ApiResponse::error("Missing email parameter"))),
    };

    match state.manager.list_calendars(email).await {
        Ok(calendars) => Ok(Json(ApiResponse::success(calendars))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to list calendars: {}", e)))),
    }
}

/// Get calendar by ID
pub async fn get_calendar(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
) -> Result<Json<ApiResponse<Calendar>>, StatusCode> {
    match state.manager.get_calendar(&calendar_id).await {
        Ok(Some(calendar)) => Ok(Json(ApiResponse::success(calendar))),
        Ok(None) => Ok(Json(ApiResponse::error("Calendar not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get calendar: {}", e)))),
    }
}

/// Create calendar request body
#[derive(Debug, Deserialize)]
pub struct CreateCalendarBody {
    pub email: String,
    pub name: String,
    pub color: Option<String>,
}

/// Create a calendar
pub async fn create_calendar(
    State(state): State<Arc<CalDavState>>,
    Json(body): Json<CreateCalendarBody>,
) -> Result<Json<ApiResponse<Calendar>>, StatusCode> {
    let req = CreateCalendarRequest {
        name: body.name,
        color: body.color,
    };

    match state.manager.create_calendar(&body.email, req).await {
        Ok(calendar) => Ok(Json(ApiResponse::success(calendar))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to create calendar: {}", e)))),
    }
}

/// Update a calendar
pub async fn update_calendar(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
    Json(body): Json<CreateCalendarRequest>,
) -> Result<Json<ApiResponse<Calendar>>, StatusCode> {
    match state.manager.update_calendar(&calendar_id, body).await {
        Ok(Some(calendar)) => Ok(Json(ApiResponse::success(calendar))),
        Ok(None) => Ok(Json(ApiResponse::error("Calendar not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to update calendar: {}", e)))),
    }
}

/// Delete a calendar
pub async fn delete_calendar(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.manager.delete_calendar(&calendar_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(()))),
        Ok(false) => Ok(Json(ApiResponse::error("Calendar not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete calendar: {}", e)))),
    }
}

// ==================== EVENT ENDPOINTS ====================

/// List events in a calendar
pub async fn list_events(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<CalendarEvent>>>, StatusCode> {
    match state.manager.list_events(&calendar_id).await {
        Ok(events) => Ok(Json(ApiResponse::success(events))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to list events: {}", e)))),
    }
}

/// Get event by ID
pub async fn get_event(
    State(state): State<Arc<CalDavState>>,
    Path(event_id): Path<String>,
) -> Result<Json<ApiResponse<CalendarEvent>>, StatusCode> {
    match state.manager.get_event(&event_id).await {
        Ok(Some(event)) => Ok(Json(ApiResponse::success(event))),
        Ok(None) => Ok(Json(ApiResponse::error("Event not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get event: {}", e)))),
    }
}

/// Create an event
pub async fn create_event(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
    Json(body): Json<CreateEventRequest>,
) -> Result<Json<ApiResponse<CalendarEvent>>, StatusCode> {
    match state.manager.create_event(&calendar_id, body).await {
        Ok(event) => Ok(Json(ApiResponse::success(event))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to create event: {}", e)))),
    }
}

/// Update an event
pub async fn update_event(
    State(state): State<Arc<CalDavState>>,
    Path(event_id): Path<String>,
    Json(body): Json<CreateEventRequest>,
) -> Result<Json<ApiResponse<CalendarEvent>>, StatusCode> {
    match state.manager.update_event(&event_id, body).await {
        Ok(Some(event)) => Ok(Json(ApiResponse::success(event))),
        Ok(None) => Ok(Json(ApiResponse::error("Event not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to update event: {}", e)))),
    }
}

/// Delete an event
pub async fn delete_event(
    State(state): State<Arc<CalDavState>>,
    Path(event_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.manager.delete_event(&event_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(()))),
        Ok(false) => Ok(Json(ApiResponse::error("Event not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete event: {}", e)))),
    }
}

/// Import ICS data
pub async fn import_ics(
    State(state): State<Arc<CalDavState>>,
    Path(calendar_id): Path<String>,
    Json(body): Json<ImportDataRequest>,
) -> Result<Json<ApiResponse<CalendarEvent>>, StatusCode> {
    match state.manager.import_ics(&calendar_id, &body.data).await {
        Ok(event) => Ok(Json(ApiResponse::success(event))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to import ICS: {}", e)))),
    }
}

// ==================== ADDRESS BOOK ENDPOINTS ====================

/// List address books for a user
pub async fn list_addressbooks(
    State(state): State<Arc<CalDavState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<AddressBook>>>, StatusCode> {
    let email = match params.get("email") {
        Some(e) => e,
        None => return Ok(Json(ApiResponse::error("Missing email parameter"))),
    };

    match state.manager.list_addressbooks(email).await {
        Ok(addressbooks) => Ok(Json(ApiResponse::success(addressbooks))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to list address books: {}", e)))),
    }
}

/// Get address book by ID
pub async fn get_addressbook(
    State(state): State<Arc<CalDavState>>,
    Path(addressbook_id): Path<String>,
) -> Result<Json<ApiResponse<AddressBook>>, StatusCode> {
    match state.manager.get_addressbook(&addressbook_id).await {
        Ok(Some(addressbook)) => Ok(Json(ApiResponse::success(addressbook))),
        Ok(None) => Ok(Json(ApiResponse::error("Address book not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get address book: {}", e)))),
    }
}

/// Create address book request body
#[derive(Debug, Deserialize)]
pub struct CreateAddressBookBody {
    pub email: String,
    pub name: String,
}

/// Create an address book
pub async fn create_addressbook(
    State(state): State<Arc<CalDavState>>,
    Json(body): Json<CreateAddressBookBody>,
) -> Result<Json<ApiResponse<AddressBook>>, StatusCode> {
    match state.manager.create_addressbook(&body.email, &body.name).await {
        Ok(addressbook) => Ok(Json(ApiResponse::success(addressbook))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to create address book: {}", e)))),
    }
}

/// Delete an address book
pub async fn delete_addressbook(
    State(state): State<Arc<CalDavState>>,
    Path(addressbook_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.manager.delete_addressbook(&addressbook_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(()))),
        Ok(false) => Ok(Json(ApiResponse::error("Address book not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete address book: {}", e)))),
    }
}

// ==================== CONTACT ENDPOINTS ====================

/// List contacts in an address book
pub async fn list_contacts(
    State(state): State<Arc<CalDavState>>,
    Path(addressbook_id): Path<String>,
) -> Result<Json<ApiResponse<Vec<Contact>>>, StatusCode> {
    match state.manager.list_contacts(&addressbook_id).await {
        Ok(contacts) => Ok(Json(ApiResponse::success(contacts))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to list contacts: {}", e)))),
    }
}

/// Get contact by ID
pub async fn get_contact(
    State(state): State<Arc<CalDavState>>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<Contact>>, StatusCode> {
    match state.manager.get_contact(&contact_id).await {
        Ok(Some(contact)) => Ok(Json(ApiResponse::success(contact))),
        Ok(None) => Ok(Json(ApiResponse::error("Contact not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get contact: {}", e)))),
    }
}

/// Create a contact
pub async fn create_contact(
    State(state): State<Arc<CalDavState>>,
    Path(addressbook_id): Path<String>,
    Json(body): Json<CreateContactRequest>,
) -> Result<Json<ApiResponse<Contact>>, StatusCode> {
    match state.manager.create_contact(&addressbook_id, body).await {
        Ok(contact) => Ok(Json(ApiResponse::success(contact))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to create contact: {}", e)))),
    }
}

/// Update a contact
pub async fn update_contact(
    State(state): State<Arc<CalDavState>>,
    Path(contact_id): Path<String>,
    Json(body): Json<CreateContactRequest>,
) -> Result<Json<ApiResponse<Contact>>, StatusCode> {
    match state.manager.update_contact(&contact_id, body).await {
        Ok(Some(contact)) => Ok(Json(ApiResponse::success(contact))),
        Ok(None) => Ok(Json(ApiResponse::error("Contact not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to update contact: {}", e)))),
    }
}

/// Delete a contact
pub async fn delete_contact(
    State(state): State<Arc<CalDavState>>,
    Path(contact_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.manager.delete_contact(&contact_id).await {
        Ok(true) => Ok(Json(ApiResponse::success(()))),
        Ok(false) => Ok(Json(ApiResponse::error("Contact not found"))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete contact: {}", e)))),
    }
}

/// Import VCF data
pub async fn import_vcf(
    State(state): State<Arc<CalDavState>>,
    Path(addressbook_id): Path<String>,
    Json(body): Json<ImportDataRequest>,
) -> Result<Json<ApiResponse<Contact>>, StatusCode> {
    match state.manager.import_vcf(&addressbook_id, &body.data).await {
        Ok(contact) => Ok(Json(ApiResponse::success(contact))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to import VCF: {}", e)))),
    }
}
