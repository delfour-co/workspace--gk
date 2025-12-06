use std::collections::HashMap;

/// A MIME part (can be text, HTML, or attachment)
#[derive(Debug, Clone)]
pub struct MimePart {
    /// Content-Type header value
    pub content_type: String,
    /// Content-Disposition header value (e.g., "attachment")
    pub content_disposition: Option<String>,
    /// Filename from Content-Disposition or Content-Type
    pub filename: Option<String>,
    /// Content-Transfer-Encoding (e.g., "base64", "quoted-printable")
    pub encoding: Option<String>,
    /// Raw body content (may be encoded)
    pub body: Vec<u8>,
    /// Whether this part is an attachment
    pub is_attachment: bool,
}

impl Default for MimePart {
    fn default() -> Self {
        MimePart {
            content_type: "text/plain".to_string(),
            content_disposition: None,
            filename: None,
            encoding: None,
            body: Vec::new(),
            is_attachment: false,
        }
    }
}

/// Parsed email with separated parts
#[derive(Debug, Clone, Default)]
pub struct ParsedEmail {
    /// Email headers (From, To, Subject, etc.)
    pub headers: HashMap<String, String>,
    /// Plain text body (if present)
    pub text_body: Option<String>,
    /// HTML body (if present)
    pub html_body: Option<String>,
    /// List of attachments
    pub attachments: Vec<MimePart>,
}

impl ParsedEmail {
    pub fn new() -> Self {
        ParsedEmail::default()
    }

    /// Get total size of all attachments in bytes
    pub fn total_attachment_size(&self) -> usize {
        self.attachments.iter().map(|a| a.body.len()).sum()
    }

    /// Get number of attachments
    pub fn attachment_count(&self) -> usize {
        self.attachments.len()
    }

    /// Check if email has attachments
    pub fn has_attachments(&self) -> bool {
        !self.attachments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mime_part_default() {
        let part = MimePart::default();
        assert_eq!(part.content_type, "text/plain");
        assert!(part.content_disposition.is_none());
        assert!(part.filename.is_none());
        assert!(!part.is_attachment);
    }

    #[test]
    fn test_parsed_email_default() {
        let email = ParsedEmail::default();
        assert!(email.headers.is_empty());
        assert!(email.text_body.is_none());
        assert!(email.html_body.is_none());
        assert_eq!(email.attachment_count(), 0);
    }

    #[test]
    fn test_parsed_email_new() {
        let email = ParsedEmail::new();
        assert!(!email.has_attachments());
    }

    #[test]
    fn test_attachment_count() {
        let mut email = ParsedEmail::new();
        assert_eq!(email.attachment_count(), 0);

        email.attachments.push(MimePart::default());
        assert_eq!(email.attachment_count(), 1);
        assert!(email.has_attachments());
    }

    #[test]
    fn test_total_attachment_size() {
        let mut email = ParsedEmail::new();

        email.attachments.push(MimePart {
            body: vec![1, 2, 3, 4, 5], // 5 bytes
            ..Default::default()
        });

        email.attachments.push(MimePart {
            body: vec![1, 2, 3], // 3 bytes
            ..Default::default()
        });

        assert_eq!(email.total_attachment_size(), 8);
    }
}
