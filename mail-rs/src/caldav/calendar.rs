//! Calendar operations
//!
//! Provides iCalendar (ICS) generation and parsing for calendar events.

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use icalendar::{Component, Event, EventLike, Calendar as ICalendar};
use uuid::Uuid;

use super::types::*;

/// Create iCalendar data from event request
pub fn create_ics(event: &CreateEventRequest, uid: Option<&str>) -> Result<String> {
    let event_uid = uid.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());

    let mut ical_event = Event::new();
    ical_event.uid(&event_uid);
    ical_event.summary(&event.summary);
    ical_event.starts(event.dtstart);
    ical_event.ends(event.dtend);
    ical_event.timestamp(Utc::now());

    if let Some(ref desc) = event.description {
        ical_event.description(desc);
    }

    if let Some(ref loc) = event.location {
        ical_event.location(loc);
    }

    let mut calendar = ICalendar::new();
    calendar.push(ical_event.done());

    Ok(calendar.to_string())
}

/// Parse iCalendar data and extract event details
pub fn parse_ics(ics: &str, event_id: &str, calendar_id: &str) -> Result<CalendarEvent> {
    // Simple ICS parser for VEVENT components
    let mut uid = None;
    let mut summary = None;
    let mut dtstart = None;
    let mut dtend = None;
    let mut in_vevent = false;

    for line in ics.lines() {
        let line = line.trim();

        if line == "BEGIN:VEVENT" {
            in_vevent = true;
        } else if line == "END:VEVENT" {
            in_vevent = false;
        } else if in_vevent {
            if let Some(value) = line.strip_prefix("UID:") {
                uid = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("SUMMARY:") {
                summary = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("DTSTART:") {
                dtstart = parse_ical_datetime(value);
            } else if let Some(value) = line.strip_prefix("DTSTART;") {
                // Handle DTSTART with parameters (e.g., DTSTART;TZID=...)
                if let Some(dt_value) = value.split(':').last() {
                    dtstart = parse_ical_datetime(dt_value);
                }
            } else if let Some(value) = line.strip_prefix("DTEND:") {
                dtend = parse_ical_datetime(value);
            } else if let Some(value) = line.strip_prefix("DTEND;") {
                if let Some(dt_value) = value.split(':').last() {
                    dtend = parse_ical_datetime(dt_value);
                }
            }
        }
    }

    let uid = uid.ok_or_else(|| anyhow!("Missing UID in ICS data"))?;
    let now = Utc::now();

    Ok(CalendarEvent {
        id: event_id.to_string(),
        calendar_id: calendar_id.to_string(),
        uid,
        ics_data: ics.to_string(),
        summary,
        dtstart,
        dtend,
        etag: generate_etag(ics),
        created_at: now,
        updated_at: now,
    })
}

/// Parse iCalendar datetime format
fn parse_ical_datetime(value: &str) -> Option<DateTime<Utc>> {
    // Handle formats like: 20240115T100000Z or 20240115T100000
    let value = value.trim_end_matches('Z');

    if value.len() >= 15 {
        // Basic format: YYYYMMDDTHHmmss
        let year = value[0..4].parse().ok()?;
        let month = value[4..6].parse().ok()?;
        let day = value[6..8].parse().ok()?;
        let hour = value[9..11].parse().ok()?;
        let min = value[11..13].parse().ok()?;
        let sec = value[13..15].parse().ok()?;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(hour, min, sec))
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
    } else if value.len() >= 8 {
        // Date only: YYYYMMDD
        let year = value[0..4].parse().ok()?;
        let month = value[4..6].parse().ok()?;
        let day = value[6..8].parse().ok()?;

        chrono::NaiveDate::from_ymd_opt(year, month, day)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| DateTime::from_naive_utc_and_offset(dt, Utc))
    } else {
        None
    }
}

/// Generate an ETag for ICS content
fn generate_etag(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let timestamp = Utc::now().timestamp();
    timestamp.hash(&mut hasher);
    format!("\"{}\"", hasher.finish())
}

/// Create a simple recurring event
pub fn create_recurring_ics(
    event: &CreateEventRequest,
    uid: Option<&str>,
    rrule: &str,
) -> Result<String> {
    let event_uid = uid.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now();

    let ics = format!(
        "BEGIN:VCALENDAR\r\n\
         VERSION:2.0\r\n\
         PRODID:-//mail-rs//CalDAV//EN\r\n\
         BEGIN:VEVENT\r\n\
         UID:{}\r\n\
         DTSTAMP:{}\r\n\
         DTSTART:{}\r\n\
         DTEND:{}\r\n\
         SUMMARY:{}\r\n\
         {}{}{}\
         RRULE:{}\r\n\
         END:VEVENT\r\n\
         END:VCALENDAR\r\n",
        event_uid,
        format_ical_datetime(&now),
        format_ical_datetime(&event.dtstart),
        format_ical_datetime(&event.dtend),
        event.summary,
        event.description.as_ref().map(|d| format!("DESCRIPTION:{}\r\n", d)).unwrap_or_default(),
        event.location.as_ref().map(|l| format!("LOCATION:{}\r\n", l)).unwrap_or_default(),
        "",
        rrule,
    );

    Ok(ics)
}

/// Format DateTime to iCalendar format
pub fn format_ical_datetime(dt: &DateTime<Utc>) -> String {
    dt.format("%Y%m%dT%H%M%SZ").to_string()
}
