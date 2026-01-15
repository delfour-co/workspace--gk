//! Contact operations
//!
//! Provides vCard (VCF) generation and parsing for contacts.

use anyhow::{anyhow, Result};
use chrono::Utc;
use uuid::Uuid;

use super::types::*;

/// Create vCard data from contact request
pub fn create_vcf(contact: &CreateContactRequest, uid: Option<&str>) -> Result<String> {
    let contact_uid = uid.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now();

    let mut vcf = String::new();
    vcf.push_str("BEGIN:VCARD\r\n");
    vcf.push_str("VERSION:3.0\r\n");
    vcf.push_str(&format!("UID:{}\r\n", contact_uid));
    vcf.push_str(&format!("FN:{}\r\n", contact.full_name));

    // Parse name parts from full name
    let name_parts: Vec<&str> = contact.full_name.split_whitespace().collect();
    if name_parts.len() >= 2 {
        let last = name_parts.last().unwrap_or(&"");
        let first = name_parts.first().unwrap_or(&"");
        vcf.push_str(&format!("N:{};{};;;\r\n", last, first));
    } else if name_parts.len() == 1 {
        vcf.push_str(&format!("N:{};{};;;\r\n", name_parts[0], ""));
    }

    if let Some(ref email) = contact.email {
        vcf.push_str(&format!("EMAIL;TYPE=INTERNET:{}\r\n", email));
    }

    if let Some(ref phone) = contact.phone {
        vcf.push_str(&format!("TEL;TYPE=CELL:{}\r\n", phone));
    }

    vcf.push_str(&format!("REV:{}\r\n", now.format("%Y%m%dT%H%M%SZ")));
    vcf.push_str("END:VCARD\r\n");

    Ok(vcf)
}

/// Parse vCard data and extract contact details
pub fn parse_vcf(vcf: &str, contact_id: &str, addressbook_id: &str) -> Result<Contact> {
    let mut uid = None;
    let mut fn_name = None;
    let mut email = None;
    let mut in_vcard = false;

    for line in vcf.lines() {
        let line = line.trim();

        if line == "BEGIN:VCARD" {
            in_vcard = true;
        } else if line == "END:VCARD" {
            in_vcard = false;
        } else if in_vcard {
            if let Some(value) = line.strip_prefix("UID:") {
                uid = Some(value.to_string());
            } else if let Some(value) = line.strip_prefix("FN:") {
                fn_name = Some(value.to_string());
            } else if line.starts_with("EMAIL") {
                // Handle EMAIL;TYPE=INTERNET: or EMAIL:
                if let Some(pos) = line.find(':') {
                    email = Some(line[pos + 1..].to_string());
                }
            }
        }
    }

    let uid = uid.ok_or_else(|| anyhow!("Missing UID in vCard data"))?;
    let now = Utc::now();

    Ok(Contact {
        id: contact_id.to_string(),
        addressbook_id: addressbook_id.to_string(),
        uid,
        vcf_data: vcf.to_string(),
        fn_name,
        email,
        etag: generate_etag(vcf),
        created_at: now,
        updated_at: now,
    })
}

/// Generate an ETag for vCard content
fn generate_etag(content: &str) -> String {
    use std::hash::{Hash, Hasher};
    use std::collections::hash_map::DefaultHasher;

    let mut hasher = DefaultHasher::new();
    content.hash(&mut hasher);
    let timestamp = Utc::now().timestamp();
    timestamp.hash(&mut hasher);
    format!("\"{}\"", hasher.finish())
}

/// Create a complete vCard with additional fields
pub fn create_full_vcf(contact: &CreateFullContactRequest, uid: Option<&str>) -> Result<String> {
    let contact_uid = uid.map(|s| s.to_string()).unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now();

    let mut vcf = String::new();
    vcf.push_str("BEGIN:VCARD\r\n");
    vcf.push_str("VERSION:3.0\r\n");
    vcf.push_str(&format!("UID:{}\r\n", contact_uid));
    vcf.push_str(&format!("FN:{}\r\n", contact.full_name));

    // Structured name
    if let (Some(ref first), Some(ref last)) = (&contact.first_name, &contact.last_name) {
        vcf.push_str(&format!("N:{};{};;;\r\n", last, first));
    }

    // Emails
    for email in &contact.emails {
        vcf.push_str(&format!("EMAIL;TYPE={}:{}\r\n", email.email_type, email.address));
    }

    // Phone numbers
    for phone in &contact.phones {
        vcf.push_str(&format!("TEL;TYPE={}:{}\r\n", phone.phone_type, phone.number));
    }

    // Address
    if let Some(ref addr) = contact.address {
        vcf.push_str(&format!(
            "ADR;TYPE={}:;;{};{};{};{};{}\r\n",
            addr.addr_type,
            addr.street.as_deref().unwrap_or(""),
            addr.city.as_deref().unwrap_or(""),
            addr.state.as_deref().unwrap_or(""),
            addr.postal_code.as_deref().unwrap_or(""),
            addr.country.as_deref().unwrap_or(""),
        ));
    }

    // Organization
    if let Some(ref org) = contact.organization {
        vcf.push_str(&format!("ORG:{}\r\n", org));
    }

    // Title
    if let Some(ref title) = contact.title {
        vcf.push_str(&format!("TITLE:{}\r\n", title));
    }

    // Note
    if let Some(ref note) = contact.note {
        vcf.push_str(&format!("NOTE:{}\r\n", note));
    }

    // Birthday
    if let Some(ref bday) = contact.birthday {
        vcf.push_str(&format!("BDAY:{}\r\n", bday.format("%Y%m%d")));
    }

    vcf.push_str(&format!("REV:{}\r\n", now.format("%Y%m%dT%H%M%SZ")));
    vcf.push_str("END:VCARD\r\n");

    Ok(vcf)
}

/// Extended create contact request with all vCard fields
#[derive(Debug, Clone)]
pub struct CreateFullContactRequest {
    pub full_name: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub emails: Vec<EmailEntry>,
    pub phones: Vec<PhoneEntry>,
    pub address: Option<AddressEntry>,
    pub organization: Option<String>,
    pub title: Option<String>,
    pub note: Option<String>,
    pub birthday: Option<chrono::NaiveDate>,
}

#[derive(Debug, Clone)]
pub struct EmailEntry {
    pub address: String,
    pub email_type: String, // WORK, HOME, etc.
}

#[derive(Debug, Clone)]
pub struct PhoneEntry {
    pub number: String,
    pub phone_type: String, // CELL, WORK, HOME, etc.
}

#[derive(Debug, Clone)]
pub struct AddressEntry {
    pub addr_type: String, // WORK, HOME
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}
