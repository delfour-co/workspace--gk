//! CalDAV/CardDAV manager for database persistence
//!
//! Provides full management of calendars, events, address books, and contacts.

use anyhow::Result;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, SqlitePool};
use uuid::Uuid;

use super::calendar::{create_ics, parse_ics};
use super::contacts::{create_vcf, parse_vcf};
use super::types::*;

/// CalDAV manager
pub struct CalDavManager {
    db: SqlitePool,
}

#[derive(FromRow)]
struct CalendarRow {
    id: String,
    owner_email: String,
    name: String,
    color: Option<String>,
    sync_token: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct EventRow {
    id: String,
    calendar_id: String,
    uid: String,
    ics_data: String,
    summary: Option<String>,
    dtstart: Option<String>,
    dtend: Option<String>,
    etag: String,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct AddressBookRow {
    id: String,
    owner_email: String,
    name: String,
    sync_token: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(FromRow)]
struct ContactRow {
    id: String,
    addressbook_id: String,
    uid: String,
    vcf_data: String,
    fn_name: Option<String>,
    email: Option<String>,
    etag: String,
    created_at: String,
    updated_at: String,
}

impl CalDavManager {
    /// Create a new CalDAV manager
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Initialize database tables
    pub async fn init_db(&self) -> Result<()> {
        // Calendars table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendars (
                id TEXT PRIMARY KEY,
                owner_email TEXT NOT NULL,
                name TEXT NOT NULL,
                color TEXT,
                sync_token TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Calendar events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS calendar_events (
                id TEXT PRIMARY KEY,
                calendar_id TEXT NOT NULL,
                uid TEXT NOT NULL,
                ics_data TEXT NOT NULL,
                summary TEXT,
                dtstart TEXT,
                dtend TEXT,
                etag TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (calendar_id) REFERENCES calendars(id)
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Address books table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS addressbooks (
                id TEXT PRIMARY KEY,
                owner_email TEXT NOT NULL,
                name TEXT NOT NULL,
                sync_token TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Contacts table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                addressbook_id TEXT NOT NULL,
                uid TEXT NOT NULL,
                vcf_data TEXT NOT NULL,
                fn_name TEXT,
                email TEXT,
                etag TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (addressbook_id) REFERENCES addressbooks(id)
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_calendars_owner ON calendars(owner_email)")
            .execute(&self.db)
            .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_events_calendar ON calendar_events(calendar_id)",
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_addressbooks_owner ON addressbooks(owner_email)",
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_contacts_addressbook ON contacts(addressbook_id)",
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    // ==================== CALENDAR METHODS ====================

    /// List calendars for a user
    pub async fn list_calendars(&self, email: &str) -> Result<Vec<Calendar>> {
        let rows: Vec<CalendarRow> = sqlx::query_as(
            "SELECT * FROM calendars WHERE owner_email = ? ORDER BY name",
        )
        .bind(email)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| row_to_calendar(r)).collect())
    }

    /// Get calendar by ID
    pub async fn get_calendar(&self, id: &str) -> Result<Option<Calendar>> {
        let row: Option<CalendarRow> = sqlx::query_as(
            "SELECT * FROM calendars WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(row_to_calendar))
    }

    /// Create a calendar
    pub async fn create_calendar(&self, email: &str, req: CreateCalendarRequest) -> Result<Calendar> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let sync_token = generate_sync_token();

        sqlx::query(
            "INSERT INTO calendars (id, owner_email, name, color, sync_token, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(&req.name)
        .bind(&req.color)
        .bind(&sync_token)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(Calendar {
            id,
            owner_email: email.to_string(),
            name: req.name,
            color: req.color,
            sync_token: Some(sync_token),
            created_at: now,
            updated_at: now,
        })
    }

    /// Update a calendar
    pub async fn update_calendar(&self, id: &str, req: CreateCalendarRequest) -> Result<Option<Calendar>> {
        let now = Utc::now();
        let sync_token = generate_sync_token();

        let result = sqlx::query(
            "UPDATE calendars SET name = ?, color = ?, sync_token = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&req.name)
        .bind(&req.color)
        .bind(&sync_token)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.db)
        .await?;

        if result.rows_affected() > 0 {
            self.get_calendar(id).await
        } else {
            Ok(None)
        }
    }

    /// Delete a calendar
    pub async fn delete_calendar(&self, id: &str) -> Result<bool> {
        // Delete all events first
        sqlx::query("DELETE FROM calendar_events WHERE calendar_id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        let result = sqlx::query("DELETE FROM calendars WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== EVENT METHODS ====================

    /// List events in a calendar
    pub async fn list_events(&self, calendar_id: &str) -> Result<Vec<CalendarEvent>> {
        let rows: Vec<EventRow> = sqlx::query_as(
            "SELECT * FROM calendar_events WHERE calendar_id = ? ORDER BY dtstart",
        )
        .bind(calendar_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| row_to_event(r)).collect())
    }

    /// Get event by ID
    pub async fn get_event(&self, id: &str) -> Result<Option<CalendarEvent>> {
        let row: Option<EventRow> = sqlx::query_as(
            "SELECT * FROM calendar_events WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(row_to_event))
    }

    /// Create an event
    pub async fn create_event(&self, calendar_id: &str, req: CreateEventRequest) -> Result<CalendarEvent> {
        let id = Uuid::new_v4().to_string();
        let uid = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Generate ICS data
        let ics_data = create_ics(&req, Some(&uid))?;
        let etag = generate_etag(&ics_data);

        sqlx::query(
            "INSERT INTO calendar_events (id, calendar_id, uid, ics_data, summary, dtstart, dtend, etag, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(calendar_id)
        .bind(&uid)
        .bind(&ics_data)
        .bind(&req.summary)
        .bind(req.dtstart.to_rfc3339())
        .bind(req.dtend.to_rfc3339())
        .bind(&etag)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        // Update calendar sync token
        self.update_calendar_sync_token(calendar_id).await?;

        Ok(CalendarEvent {
            id,
            calendar_id: calendar_id.to_string(),
            uid,
            ics_data,
            summary: Some(req.summary),
            dtstart: Some(req.dtstart),
            dtend: Some(req.dtend),
            etag,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update an event
    pub async fn update_event(&self, id: &str, req: CreateEventRequest) -> Result<Option<CalendarEvent>> {
        let existing = self.get_event(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let now = Utc::now();
        let ics_data = create_ics(&req, Some(&existing.uid))?;
        let etag = generate_etag(&ics_data);

        sqlx::query(
            "UPDATE calendar_events SET ics_data = ?, summary = ?, dtstart = ?, dtend = ?, etag = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&ics_data)
        .bind(&req.summary)
        .bind(req.dtstart.to_rfc3339())
        .bind(req.dtend.to_rfc3339())
        .bind(&etag)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.db)
        .await?;

        // Update calendar sync token
        self.update_calendar_sync_token(&existing.calendar_id).await?;

        self.get_event(id).await
    }

    /// Delete an event
    pub async fn delete_event(&self, id: &str) -> Result<bool> {
        // Get calendar_id first for sync token update
        let event = self.get_event(id).await?;

        let result = sqlx::query("DELETE FROM calendar_events WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() > 0 {
            if let Some(e) = event {
                self.update_calendar_sync_token(&e.calendar_id).await?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Import ICS data
    pub async fn import_ics(&self, calendar_id: &str, ics_data: &str) -> Result<CalendarEvent> {
        let id = Uuid::new_v4().to_string();
        let event = parse_ics(ics_data, &id, calendar_id)?;
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO calendar_events (id, calendar_id, uid, ics_data, summary, dtstart, dtend, etag, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(calendar_id)
        .bind(&event.uid)
        .bind(&event.ics_data)
        .bind(&event.summary)
        .bind(event.dtstart.map(|d| d.to_rfc3339()))
        .bind(event.dtend.map(|d| d.to_rfc3339()))
        .bind(&event.etag)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        self.update_calendar_sync_token(calendar_id).await?;

        Ok(event)
    }

    /// Update calendar sync token
    async fn update_calendar_sync_token(&self, calendar_id: &str) -> Result<()> {
        let sync_token = generate_sync_token();
        sqlx::query("UPDATE calendars SET sync_token = ?, updated_at = ? WHERE id = ?")
            .bind(&sync_token)
            .bind(Utc::now().to_rfc3339())
            .bind(calendar_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    // ==================== ADDRESS BOOK METHODS ====================

    /// List address books for a user
    pub async fn list_addressbooks(&self, email: &str) -> Result<Vec<AddressBook>> {
        let rows: Vec<AddressBookRow> = sqlx::query_as(
            "SELECT * FROM addressbooks WHERE owner_email = ? ORDER BY name",
        )
        .bind(email)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| row_to_addressbook(r)).collect())
    }

    /// Get address book by ID
    pub async fn get_addressbook(&self, id: &str) -> Result<Option<AddressBook>> {
        let row: Option<AddressBookRow> = sqlx::query_as(
            "SELECT * FROM addressbooks WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(row_to_addressbook))
    }

    /// Create an address book
    pub async fn create_addressbook(&self, email: &str, name: &str) -> Result<AddressBook> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let sync_token = generate_sync_token();

        sqlx::query(
            "INSERT INTO addressbooks (id, owner_email, name, sync_token, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(name)
        .bind(&sync_token)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(AddressBook {
            id,
            owner_email: email.to_string(),
            name: name.to_string(),
            sync_token: Some(sync_token),
            created_at: now,
            updated_at: now,
        })
    }

    /// Delete an address book
    pub async fn delete_addressbook(&self, id: &str) -> Result<bool> {
        // Delete all contacts first
        sqlx::query("DELETE FROM contacts WHERE addressbook_id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        let result = sqlx::query("DELETE FROM addressbooks WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(result.rows_affected() > 0)
    }

    // ==================== CONTACT METHODS ====================

    /// List contacts in an address book
    pub async fn list_contacts(&self, addressbook_id: &str) -> Result<Vec<Contact>> {
        let rows: Vec<ContactRow> = sqlx::query_as(
            "SELECT * FROM contacts WHERE addressbook_id = ? ORDER BY fn_name",
        )
        .bind(addressbook_id)
        .fetch_all(&self.db)
        .await?;

        Ok(rows.into_iter().map(|r| row_to_contact(r)).collect())
    }

    /// Get contact by ID
    pub async fn get_contact(&self, id: &str) -> Result<Option<Contact>> {
        let row: Option<ContactRow> = sqlx::query_as(
            "SELECT * FROM contacts WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(row_to_contact))
    }

    /// Create a contact
    pub async fn create_contact(&self, addressbook_id: &str, req: CreateContactRequest) -> Result<Contact> {
        let id = Uuid::new_v4().to_string();
        let uid = Uuid::new_v4().to_string();
        let now = Utc::now();

        // Generate VCF data
        let vcf_data = create_vcf(&req, Some(&uid))?;
        let etag = generate_etag(&vcf_data);

        sqlx::query(
            "INSERT INTO contacts (id, addressbook_id, uid, vcf_data, fn_name, email, etag, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(addressbook_id)
        .bind(&uid)
        .bind(&vcf_data)
        .bind(&req.full_name)
        .bind(&req.email)
        .bind(&etag)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        // Update addressbook sync token
        self.update_addressbook_sync_token(addressbook_id).await?;

        Ok(Contact {
            id,
            addressbook_id: addressbook_id.to_string(),
            uid,
            vcf_data,
            fn_name: Some(req.full_name),
            email: req.email,
            etag,
            created_at: now,
            updated_at: now,
        })
    }

    /// Update a contact
    pub async fn update_contact(&self, id: &str, req: CreateContactRequest) -> Result<Option<Contact>> {
        let existing = self.get_contact(id).await?;
        if existing.is_none() {
            return Ok(None);
        }
        let existing = existing.unwrap();

        let now = Utc::now();
        let vcf_data = create_vcf(&req, Some(&existing.uid))?;
        let etag = generate_etag(&vcf_data);

        sqlx::query(
            "UPDATE contacts SET vcf_data = ?, fn_name = ?, email = ?, etag = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&vcf_data)
        .bind(&req.full_name)
        .bind(&req.email)
        .bind(&etag)
        .bind(now.to_rfc3339())
        .bind(id)
        .execute(&self.db)
        .await?;

        // Update addressbook sync token
        self.update_addressbook_sync_token(&existing.addressbook_id).await?;

        self.get_contact(id).await
    }

    /// Delete a contact
    pub async fn delete_contact(&self, id: &str) -> Result<bool> {
        let contact = self.get_contact(id).await?;

        let result = sqlx::query("DELETE FROM contacts WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        if result.rows_affected() > 0 {
            if let Some(c) = contact {
                self.update_addressbook_sync_token(&c.addressbook_id).await?;
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Import VCF data
    pub async fn import_vcf(&self, addressbook_id: &str, vcf_data: &str) -> Result<Contact> {
        let id = Uuid::new_v4().to_string();
        let contact = parse_vcf(vcf_data, &id, addressbook_id)?;
        let now = Utc::now();

        sqlx::query(
            "INSERT INTO contacts (id, addressbook_id, uid, vcf_data, fn_name, email, etag, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&contact.id)
        .bind(addressbook_id)
        .bind(&contact.uid)
        .bind(&contact.vcf_data)
        .bind(&contact.fn_name)
        .bind(&contact.email)
        .bind(&contact.etag)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        self.update_addressbook_sync_token(addressbook_id).await?;

        Ok(contact)
    }

    /// Update addressbook sync token
    async fn update_addressbook_sync_token(&self, addressbook_id: &str) -> Result<()> {
        let sync_token = generate_sync_token();
        sqlx::query("UPDATE addressbooks SET sync_token = ?, updated_at = ? WHERE id = ?")
            .bind(&sync_token)
            .bind(Utc::now().to_rfc3339())
            .bind(addressbook_id)
            .execute(&self.db)
            .await?;
        Ok(())
    }

    // ==================== STATISTICS ====================

    /// Get CalDAV/CardDAV statistics
    pub async fn get_stats(&self, email: Option<&str>) -> Result<CalDavStats> {
        let calendar_count: (i64,) = if let Some(e) = email {
            sqlx::query_as("SELECT COUNT(*) FROM calendars WHERE owner_email = ?")
                .bind(e)
                .fetch_one(&self.db)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM calendars")
                .fetch_one(&self.db)
                .await?
        };

        let event_count: (i64,) = if let Some(e) = email {
            sqlx::query_as(
                "SELECT COUNT(*) FROM calendar_events ce
                 JOIN calendars c ON ce.calendar_id = c.id
                 WHERE c.owner_email = ?",
            )
            .bind(e)
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM calendar_events")
                .fetch_one(&self.db)
                .await?
        };

        let addressbook_count: (i64,) = if let Some(e) = email {
            sqlx::query_as("SELECT COUNT(*) FROM addressbooks WHERE owner_email = ?")
                .bind(e)
                .fetch_one(&self.db)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM addressbooks")
                .fetch_one(&self.db)
                .await?
        };

        let contact_count: (i64,) = if let Some(e) = email {
            sqlx::query_as(
                "SELECT COUNT(*) FROM contacts co
                 JOIN addressbooks a ON co.addressbook_id = a.id
                 WHERE a.owner_email = ?",
            )
            .bind(e)
            .fetch_one(&self.db)
            .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM contacts")
                .fetch_one(&self.db)
                .await?
        };

        Ok(CalDavStats {
            total_calendars: calendar_count.0 as u64,
            total_events: event_count.0 as u64,
            total_addressbooks: addressbook_count.0 as u64,
            total_contacts: contact_count.0 as u64,
        })
    }
}

// ==================== HELPER FUNCTIONS ====================

fn row_to_calendar(row: CalendarRow) -> Calendar {
    Calendar {
        id: row.id,
        owner_email: row.owner_email,
        name: row.name,
        color: row.color,
        sync_token: row.sync_token,
        created_at: parse_datetime(&row.created_at),
        updated_at: parse_datetime(&row.updated_at),
    }
}

fn row_to_event(row: EventRow) -> CalendarEvent {
    CalendarEvent {
        id: row.id,
        calendar_id: row.calendar_id,
        uid: row.uid,
        ics_data: row.ics_data,
        summary: row.summary,
        dtstart: row.dtstart.as_ref().map(|s| parse_datetime(s)),
        dtend: row.dtend.as_ref().map(|s| parse_datetime(s)),
        etag: row.etag,
        created_at: parse_datetime(&row.created_at),
        updated_at: parse_datetime(&row.updated_at),
    }
}

fn row_to_addressbook(row: AddressBookRow) -> AddressBook {
    AddressBook {
        id: row.id,
        owner_email: row.owner_email,
        name: row.name,
        sync_token: row.sync_token,
        created_at: parse_datetime(&row.created_at),
        updated_at: parse_datetime(&row.updated_at),
    }
}

fn row_to_contact(row: ContactRow) -> Contact {
    Contact {
        id: row.id,
        addressbook_id: row.addressbook_id,
        uid: row.uid,
        vcf_data: row.vcf_data,
        fn_name: row.fn_name,
        email: row.email,
        etag: row.etag,
        created_at: parse_datetime(&row.created_at),
        updated_at: parse_datetime(&row.updated_at),
    }
}

fn parse_datetime(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn generate_sync_token() -> String {
    format!("sync-{}", Uuid::new_v4())
}

fn generate_etag(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    Utc::now().timestamp().hash(&mut hasher);
    format!("\"{}\"", hasher.finish())
}
