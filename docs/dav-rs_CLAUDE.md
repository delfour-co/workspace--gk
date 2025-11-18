# dav-rs - Serveur CalDAV/CardDAV

## Vue d'ensemble

`dav-rs` est un serveur CalDAV (calendriers) et CardDAV (contacts) écrit en Rust, permettant la synchronisation de calendriers et contacts avec des clients standards (Thunderbird, Apple Calendar/Contacts, etc.).

## Contexte du projet global

Complète la suite de communication avec gestion de calendriers et contacts. Permet synchronisation multi-devices et intégration avec l'assistant AI.

### Interfaces avec les autres composants

- **Expose** : CalDAV/CardDAV (WebDAV) + REST API pour MCP
- **Consommé par** : Clients CalDAV/CardDAV standards, `ai-runtime` (via mcp-dav-server)
- **Derrière** : `proxy-rs` pour exposition HTTPS

## Responsabilités

### Primaires
1. **CalDAV (Calendriers)**
   - CRUD événements/todos
   - Recurring events (RRULE)
   - Invitations (VEVENT)
   - Timezones (VTIMEZONE)
   - Sync protocol

2. **CardDAV (Contacts)**
   - CRUD contacts (vCard)
   - Groups/lists
   - Photos/avatars
   - Sync protocol

3. **WebDAV**
   - PROPFIND, PROPPATCH
   - PUT, GET, DELETE
   - REPORT (calendar-query, addressbook-query)
   - Sync-collection

4. **REST API**
   - Endpoints pour MCP server
   - JSON responses (plus simple que XML WebDAV)

### Secondaires
- Calendar sharing/permissions
- Free/busy info
- Attendees management
- Reminders

## Architecture technique

### Stack Rust

```toml
[dependencies]
# HTTP server
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace"] }

# WebDAV
http = "1"
http-body-util = "0.1"
bytes = "1"

# iCalendar/vCard parsing
icalendar = "0.16"
vcard4 = "0.1"  # Ou parser custom

# XML (pour WebDAV)
quick-xml = { version = "0.31", features = ["serialize"] }

# Database
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls", "uuid", "chrono"] }

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
chrono-tz = "0.8"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Async
tokio = { version = "1", features = ["full"] }

# Auth
jsonwebtoken = "9"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Config
toml = "0.8"
```

### Structure du projet

```
dav-rs/
├── Cargo.toml
├── config.example.toml
├── README.md
├── Dockerfile
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── caldav/
│   │   ├── mod.rs
│   │   ├── server.rs         # CalDAV endpoint handler
│   │   ├── protocol.rs       # CalDAV XML protocol
│   │   ├── calendar.rs       # Calendar operations
│   │   ├── event.rs          # Event CRUD
│   │   ├── rrule.rs          # Recurrence rules
│   │   └── query.rs          # calendar-query REPORT
│   │
│   ├── carddav/
│   │   ├── mod.rs
│   │   ├── server.rs         # CardDAV endpoint handler
│   │   ├── protocol.rs       # CardDAV XML protocol
│   │   ├── addressbook.rs    # Address book operations
│   │   ├── contact.rs        # Contact CRUD
│   │   └── query.rs          # addressbook-query REPORT
│   │
│   ├── webdav/
│   │   ├── mod.rs
│   │   ├── methods/          # WebDAV methods
│   │   │   ├── propfind.rs
│   │   │   ├── proppatch.rs
│   │   │   ├── get.rs
│   │   │   ├── put.rs
│   │   │   ├── delete.rs
│   │   │   └── report.rs
│   │   ├── properties.rs     # WebDAV properties
│   │   └── xml.rs            # XML serialization/parsing
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs         # REST API endpoints
│   │   ├── auth.rs           # JWT authentication
│   │   └── handlers/
│   │       ├── calendars.rs
│   │       ├── events.rs
│   │       ├── addressbooks.rs
│   │       └── contacts.rs
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── calendar.rs
│   │   ├── event.rs
│   │   ├── addressbook.rs
│   │   └── contact.rs
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs             # SQLite operations
│   │   └── migrations/
│   │       ├── 001_init.sql
│   │       ├── 002_calendars.sql
│   │       └── 003_contacts.sql
│   │
│   └── utils/
│       ├── mod.rs
│       ├── ical.rs           # iCalendar helpers
│       └── vcard.rs          # vCard helpers
│
├── tests/
│   ├── caldav_test.rs
│   ├── carddav_test.rs
│   └── fixtures/
│       ├── events.ics
│       └── contacts.vcf
│
└── docs/
    ├── CALDAV.md
    └── CARDDAV.md
```

## Spécifications fonctionnelles

### 1. CalDAV Protocol

**Endpoint structure**

```
/caldav/
├── {username}/
│   └── {calendar_name}/
│       ├── {event_uid}.ics
│       └── {event_uid2}.ics
```

**Discovery (PROPFIND)**

```http
PROPFIND /caldav/user@example.com/ HTTP/1.1
Depth: 0
Content-Type: application/xml

<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:current-user-principal/>
    <c:calendar-home-set/>
  </d:prop>
</d:propfind>
```

**List calendars**

```http
PROPFIND /caldav/user@example.com/ HTTP/1.1
Depth: 1
```

**Calendar query (events in date range)**

```http
REPORT /caldav/user@example.com/calendar1/ HTTP/1.1
Content-Type: application/xml

<?xml version="1.0"?>
<c:calendar-query xmlns:d="DAV:" xmlns:c="urn:ietf:params:xml:ns:caldav">
  <d:prop>
    <d:getetag/>
    <c:calendar-data/>
  </d:prop>
  <c:filter>
    <c:comp-filter name="VCALENDAR">
      <c:comp-filter name="VEVENT">
        <c:time-range start="20241101T000000Z" end="20241130T235959Z"/>
      </c:comp-filter>
    </c:comp-filter>
  </c:filter>
</c:calendar-query>
```

**Create/update event (PUT)**

```http
PUT /caldav/user@example.com/calendar1/event-123.ics HTTP/1.1
Content-Type: text/calendar

BEGIN:VCALENDAR
VERSION:2.0
PRODID:-//My App//EN
BEGIN:VEVENT
UID:event-123
DTSTAMP:20241118T100000Z
DTSTART:20241120T140000Z
DTEND:20241120T150000Z
SUMMARY:Team Meeting
DESCRIPTION:Weekly sync
LOCATION:Room 301
END:VEVENT
END:VCALENDAR
```

**Get event (GET)**

```http
GET /caldav/user@example.com/calendar1/event-123.ics HTTP/1.1
```

**Delete event (DELETE)**

```http
DELETE /caldav/user@example.com/calendar1/event-123.ics HTTP/1.1
```

### 2. CardDAV Protocol

**Endpoint structure**

```
/carddav/
├── {username}/
│   └── {addressbook_name}/
│       ├── {contact_uid}.vcf
│       └── {contact_uid2}.vcf
```

**Discovery (PROPFIND)**

```http
PROPFIND /carddav/user@example.com/ HTTP/1.1
Depth: 0
Content-Type: application/xml

<?xml version="1.0"?>
<d:propfind xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <card:addressbook-home-set/>
  </d:prop>
</d:propfind>
```

**Create/update contact (PUT)**

```http
PUT /carddav/user@example.com/contacts/alice.vcf HTTP/1.1
Content-Type: text/vcard

BEGIN:VCARD
VERSION:4.0
UID:alice-123
FN:Alice Smith
N:Smith;Alice;;;
EMAIL;TYPE=work:alice@example.com
TEL;TYPE=cell:+1234567890
END:VCARD
```

**AddressBook query**

```http
REPORT /carddav/user@example.com/contacts/ HTTP/1.1
Content-Type: application/xml

<?xml version="1.0"?>
<card:addressbook-query xmlns:d="DAV:" xmlns:card="urn:ietf:params:xml:ns:carddav">
  <d:prop>
    <d:getetag/>
    <card:address-data/>
  </d:prop>
  <card:filter>
    <card:prop-filter name="EMAIL"/>
  </card:filter>
</card:addressbook-query>
```

### 3. REST API (for MCP)

**Calendars**

```yaml
GET /api/calendars
  Response: [{ id, name, color, description }]

POST /api/calendars
  Request: { name, color?, description? }
  Response: { id, name }

GET /api/calendars/:id
DELETE /api/calendars/:id
```

**Events**

```yaml
GET /api/calendars/:calendar_id/events?start=2024-11-01&end=2024-11-30
  Response: [{
    id, uid, summary, description,
    start, end, location,
    recurrence_rule?, attendees?
  }]

POST /api/calendars/:calendar_id/events
  Request: {
    summary, description?,
    start, end,
    location?, recurrence_rule?,
    attendees?: [{ email, name, status }]
  }
  Response: { id, uid }

GET /api/events/:id
  Response: { id, uid, summary, ... }

PUT /api/events/:id
  Request: { summary?, start?, end?, ... }

DELETE /api/events/:id
```

**Address Books**

```yaml
GET /api/addressbooks
  Response: [{ id, name, description }]

POST /api/addressbooks
  Request: { name, description? }
```

**Contacts**

```yaml
GET /api/addressbooks/:ab_id/contacts
  Response: [{
    id, uid, full_name, emails, phones, organization
  }]

POST /api/addressbooks/:ab_id/contacts
  Request: {
    full_name, emails: [{ type, value }],
    phones: [{ type, value }],
    organization?, birthday?, note?
  }

GET /api/contacts/:id
PUT /api/contacts/:id
DELETE /api/contacts/:id

GET /api/contacts/search?q=alice
  Response: [{ id, full_name, emails }]
```

### 4. Database Schema

```sql
-- Calendars
CREATE TABLE calendars (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    color VARCHAR(7),
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Events
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    calendar_id UUID REFERENCES calendars(id) ON DELETE CASCADE,
    uid VARCHAR(255) UNIQUE NOT NULL,
    summary VARCHAR(500) NOT NULL,
    description TEXT,
    location VARCHAR(500),
    start_time TIMESTAMP WITH TIME ZONE NOT NULL,
    end_time TIMESTAMP WITH TIME ZONE NOT NULL,
    all_day BOOLEAN DEFAULT FALSE,
    recurrence_rule TEXT,  -- RRULE
    ical_data TEXT NOT NULL,  -- Full iCalendar data
    etag VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_events_calendar_id ON events(calendar_id);
CREATE INDEX idx_events_start_time ON events(start_time);

-- Attendees
CREATE TABLE event_attendees (
    event_id UUID REFERENCES events(id) ON DELETE CASCADE,
    email VARCHAR(255) NOT NULL,
    name VARCHAR(255),
    role VARCHAR(50),  -- CHAIR, REQ-PARTICIPANT, OPT-PARTICIPANT
    status VARCHAR(50), -- NEEDS-ACTION, ACCEPTED, DECLINED, TENTATIVE
    PRIMARY KEY (event_id, email)
);

-- Address Books
CREATE TABLE addressbooks (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

-- Contacts
CREATE TABLE contacts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    addressbook_id UUID REFERENCES addressbooks(id) ON DELETE CASCADE,
    uid VARCHAR(255) UNIQUE NOT NULL,
    full_name VARCHAR(255) NOT NULL,
    given_name VARCHAR(255),
    family_name VARCHAR(255),
    organization VARCHAR(255),
    birthday DATE,
    note TEXT,
    photo_url VARCHAR(500),
    vcard_data TEXT NOT NULL,  -- Full vCard data
    etag VARCHAR(64) NOT NULL,
    created_at TIMESTAMP DEFAULT NOW(),
    updated_at TIMESTAMP DEFAULT NOW()
);

CREATE INDEX idx_contacts_addressbook_id ON contacts(addressbook_id);
CREATE INDEX idx_contacts_full_name ON contacts(full_name);

-- Contact emails
CREATE TABLE contact_emails (
    contact_id UUID REFERENCES contacts(id) ON DELETE CASCADE,
    type VARCHAR(50),  -- work, home, other
    email VARCHAR(255) NOT NULL,
    PRIMARY KEY (contact_id, email)
);

-- Contact phones
CREATE TABLE contact_phones (
    contact_id UUID REFERENCES contacts(id) ON DELETE CASCADE,
    type VARCHAR(50),  -- work, home, cell, other
    phone VARCHAR(50) NOT NULL,
    PRIMARY KEY (contact_id, phone)
);
```

### 5. iCalendar Processing

```rust
use icalendar::{Calendar, Event as ICalEvent, Component};

pub fn parse_ical(data: &str) -> Result<Vec<Event>> {
    let calendar = Calendar::parse(data)?;
    
    let mut events = Vec::new();
    
    for component in calendar.components {
        if let Component::Event(ical_event) = component {
            let event = Event {
                uid: ical_event.get_uid()?,
                summary: ical_event.get_summary()?,
                description: ical_event.get_description(),
                location: ical_event.get_location(),
                start: ical_event.get_dtstart()?,
                end: ical_event.get_dtend()?,
                recurrence_rule: ical_event.get_rrule(),
                attendees: parse_attendees(&ical_event),
            };
            
            events.push(event);
        }
    }
    
    Ok(events)
}

pub fn generate_ical(event: &Event) -> String {
    let mut calendar = Calendar::new();
    
    let mut ical_event = ICalEvent::new()
        .uid(&event.uid)
        .summary(&event.summary)
        .starts(event.start)
        .ends(event.end)
        .done();
    
    if let Some(desc) = &event.description {
        ical_event = ical_event.description(desc);
    }
    
    if let Some(loc) = &event.location {
        ical_event = ical_event.location(loc);
    }
    
    calendar.push(ical_event);
    calendar.to_string()
}
```

### 6. vCard Processing

```rust
// Simplified vCard parsing
pub fn parse_vcard(data: &str) -> Result<Contact> {
    let lines: Vec<&str> = data.lines().collect();
    
    let mut contact = Contact::default();
    
    for line in lines {
        if line.starts_with("FN:") {
            contact.full_name = line[3..].to_string();
        } else if line.starts_with("EMAIL") {
            let email = extract_email_value(line)?;
            contact.emails.push(email);
        } else if line.starts_with("TEL") {
            let phone = extract_phone_value(line)?;
            contact.phones.push(phone);
        }
        // ... autres champs
    }
    
    Ok(contact)
}

pub fn generate_vcard(contact: &Contact) -> String {
    let mut vcard = String::new();
    vcard.push_str("BEGIN:VCARD\n");
    vcard.push_str("VERSION:4.0\n");
    vcard.push_str(&format!("UID:{}\n", contact.uid));
    vcard.push_str(&format!("FN:{}\n", contact.full_name));
    
    if let Some(org) = &contact.organization {
        vcard.push_str(&format!("ORG:{}\n", org));
    }
    
    for email in &contact.emails {
        vcard.push_str(&format!("EMAIL;TYPE={}:{}\n", email.type_, email.value));
    }
    
    for phone in &contact.phones {
        vcard.push_str(&format!("TEL;TYPE={}:{}\n", phone.type_, phone.value));
    }
    
    vcard.push_str("END:VCARD\n");
    vcard
}
```

### 7. Configuration

```toml
# config.toml

[server]
bind_addr = "0.0.0.0:8082"

[caldav]
base_path = "/caldav"

[carddav]
base_path = "/carddav"

[api]
base_path = "/api"
jwt_secret = "changeme"

[database]
url = "sqlite:///var/dav/dav.db"

[logging]
level = "info"
format = "json"
```

## Tests

```rust
#[tokio::test]
async fn test_create_event() {
    let app = create_test_app().await;
    
    let ical = r#"
BEGIN:VCALENDAR
VERSION:2.0
BEGIN:VEVENT
UID:test-123
SUMMARY:Test Event
DTSTART:20241120T140000Z
DTEND:20241120T150000Z
END:VEVENT
END:VCALENDAR
"#;
    
    let resp = app.put_event("user@test.com", "calendar1", "test-123.ics", ical)
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 201);
}

#[tokio::test]
async fn test_calendar_query() {
    let app = create_test_app().await;
    
    // Create events
    create_test_events(&app).await;
    
    // Query events in November 2024
    let resp = app.calendar_query(
        "user@test.com",
        "calendar1",
        "20241101T000000Z",
        "20241130T235959Z",
    ).await.unwrap();
    
    assert_eq!(resp.events.len(), 3);
}
```

## Déploiement

```dockerfile
FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.19
COPY --from=builder /app/target/release/dav-rs /usr/local/bin/
EXPOSE 8082
CMD ["dav-rs", "--config", "/etc/dav/config.toml"]
```

## Roadmap

### MVP - 3 semaines
- [x] CalDAV core (CRUD events)
- [x] CardDAV core (CRUD contacts)
- [x] WebDAV methods (PROPFIND, PUT, GET, DELETE)
- [x] REST API

### Post-MVP
- [ ] Recurring events (RRULE full support)
- [ ] Calendar sharing
- [ ] Free/busy queries
- [ ] Reminders/alarms
- [ ] Contact groups

## Métriques de succès

- ✅ Compatible Thunderbird/Apple Calendar
- ✅ Sync latency <500ms
- ✅ Support 1000+ events/contacts
