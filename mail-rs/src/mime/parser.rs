use anyhow::{anyhow, Result};
use base64::{engine::general_purpose, Engine as _};
use std::collections::HashMap;

use super::types::{MimePart, ParsedEmail};

/// MIME message parser
pub struct MimeParser;

impl MimeParser {
    /// Parse a raw email message into structured parts
    pub fn parse(message: &[u8]) -> Result<ParsedEmail> {
        let message_str = String::from_utf8_lossy(message);

        let (headers_str, body_str) = Self::split_headers_body(&message_str)?;
        let headers = Self::parse_headers(&headers_str);

        let mut parsed = ParsedEmail {
            headers: headers.clone(),
            ..Default::default()
        };

        // Check if this is a multipart message
        if let Some(content_type) = headers.get("content-type") {
            if content_type.to_lowercase().contains("multipart/") {
                // Extract boundary
                if let Some(boundary) = Self::extract_boundary(content_type) {
                    let parts = Self::parse_multipart(&boundary, &body_str);

                    for part in parts {
                        Self::categorize_part(&mut parsed, part);
                    }
                } else {
                    // No boundary found, treat as plain text
                    parsed.text_body = Some(body_str.to_string());
                }
            } else {
                // Single-part message
                parsed.text_body = Some(body_str.to_string());
            }
        } else {
            // No Content-Type, assume plain text
            parsed.text_body = Some(body_str.to_string());
        }

        Ok(parsed)
    }

    /// Split message into headers and body
    fn split_headers_body(message: &str) -> Result<(String, String)> {
        // Headers end with double CRLF or double LF
        if let Some(pos) = message.find("\r\n\r\n") {
            let headers = message[..pos].to_string();
            let body = message[pos + 4..].to_string();
            Ok((headers, body))
        } else if let Some(pos) = message.find("\n\n") {
            let headers = message[..pos].to_string();
            let body = message[pos + 2..].to_string();
            Ok((headers, body))
        } else {
            // No body separator found, treat entire message as headers
            Ok((message.to_string(), String::new()))
        }
    }

    /// Parse email headers into HashMap
    fn parse_headers(headers_str: &str) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        let mut current_header: Option<(String, String)> = None;

        for line in headers_str.lines() {
            if line.starts_with(' ') || line.starts_with('\t') {
                // Continuation of previous header (folded header)
                if let Some((ref name, ref mut value)) = current_header {
                    value.push(' ');
                    value.push_str(line.trim());
                }
            } else if let Some(colon_pos) = line.find(':') {
                // Save previous header if exists
                if let Some((name, value)) = current_header.take() {
                    headers.insert(name.to_lowercase(), value);
                }

                // Start new header
                let name = line[..colon_pos].trim().to_string();
                let value = line[colon_pos + 1..].trim().to_string();
                current_header = Some((name, value));
            }
        }

        // Save last header
        if let Some((name, value)) = current_header {
            headers.insert(name.to_lowercase(), value);
        }

        headers
    }

    /// Extract boundary from Content-Type header
    fn extract_boundary(content_type: &str) -> Option<String> {
        for part in content_type.split(';') {
            let part = part.trim();
            if part.starts_with("boundary=") {
                let boundary = &part[9..]; // Skip "boundary="
                                           // Remove quotes if present
                let boundary = boundary.trim_matches('"').trim_matches('\'');
                return Some(boundary.to_string());
            }
        }
        None
    }

    /// Parse multipart message body
    fn parse_multipart(boundary: &str, body: &str) -> Vec<MimePart> {
        let mut parts = Vec::new();
        let boundary_marker = format!("--{}", boundary);
        let end_marker = format!("--{}--", boundary);

        let sections: Vec<&str> = body.split(&boundary_marker).collect();

        for section in sections {
            let section = section.trim();

            // Skip empty sections and end marker
            if section.is_empty() || section.starts_with("--") {
                continue;
            }

            // Each section has headers and body
            if let Ok((part_headers_str, part_body)) = Self::split_headers_body(section) {
                let part_headers = Self::parse_headers(&part_headers_str);

                let mut part = MimePart::default();

                // Extract Content-Type
                if let Some(content_type) = part_headers.get("content-type") {
                    part.content_type = content_type.clone();

                    // Extract filename from Content-Type if present
                    if let Some(name) = Self::extract_parameter(content_type, "name") {
                        part.filename = Some(name);
                    }
                }

                // Extract Content-Disposition
                if let Some(disposition) = part_headers.get("content-disposition") {
                    part.content_disposition = Some(disposition.clone());

                    // Check if attachment
                    if disposition.to_lowercase().contains("attachment") {
                        part.is_attachment = true;
                    }

                    // Extract filename from Content-Disposition
                    if let Some(filename) = Self::extract_parameter(disposition, "filename") {
                        part.filename = Some(filename);
                    }
                }

                // Extract Content-Transfer-Encoding
                if let Some(encoding) = part_headers.get("content-transfer-encoding") {
                    part.encoding = Some(encoding.clone());
                }

                // Store body (may be encoded)
                part.body = part_body.as_bytes().to_vec();

                parts.push(part);
            }
        }

        parts
    }

    /// Extract parameter value from header (e.g., filename="file.txt")
    fn extract_parameter(header: &str, param_name: &str) -> Option<String> {
        for part in header.split(';') {
            let part = part.trim();
            if part.to_lowercase().starts_with(&format!("{}=", param_name)) {
                let value = &part[param_name.len() + 1..]; // Skip "param="
                                                            // Remove quotes
                let value = value.trim_matches('"').trim_matches('\'');
                return Some(value.to_string());
            }
        }
        None
    }

    /// Categorize MIME part into text/HTML/attachment
    fn categorize_part(parsed: &mut ParsedEmail, part: MimePart) {
        if part.is_attachment {
            parsed.attachments.push(part);
        } else if part.content_type.to_lowercase().contains("text/html") {
            // Decode body if needed
            if let Ok(decoded) = Self::decode_body(&part) {
                parsed.html_body = Some(String::from_utf8_lossy(&decoded).to_string());
            }
        } else if part.content_type.to_lowercase().contains("text/plain") {
            // Decode body if needed
            if let Ok(decoded) = Self::decode_body(&part) {
                parsed.text_body = Some(String::from_utf8_lossy(&decoded).to_string());
            }
        } else {
            // Unknown type, treat as attachment
            parsed.attachments.push(part);
        }
    }

    /// Decode message body based on Content-Transfer-Encoding
    fn decode_body(part: &MimePart) -> Result<Vec<u8>> {
        if let Some(ref encoding) = part.encoding {
            let encoding_lower = encoding.to_lowercase();
            if encoding_lower.contains("base64") {
                Self::decode_base64(&part.body)
            } else if encoding_lower.contains("quoted-printable") {
                Ok(Self::decode_quoted_printable(&part.body))
            } else {
                // 7bit, 8bit, binary - no decoding needed
                Ok(part.body.clone())
            }
        } else {
            Ok(part.body.clone())
        }
    }

    /// Decode base64 content
    fn decode_base64(content: &[u8]) -> Result<Vec<u8>> {
        // Remove whitespace and newlines
        let cleaned: Vec<u8> = content
            .iter()
            .filter(|&&b| !b.is_ascii_whitespace())
            .copied()
            .collect();

        general_purpose::STANDARD
            .decode(&cleaned)
            .map_err(|e| anyhow!("Base64 decode error: {}", e))
    }

    /// Decode quoted-printable content
    fn decode_quoted_printable(content: &[u8]) -> Vec<u8> {
        let mut result = Vec::new();
        let text = String::from_utf8_lossy(content);
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '=' {
                // Soft line break
                if chars.peek() == Some(&'\n') || chars.peek() == Some(&'\r') {
                    chars.next();
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                    }
                    continue;
                }

                // Hex-encoded character
                let mut hex = String::new();
                if let Some(c1) = chars.next() {
                    hex.push(c1);
                }
                if let Some(c2) = chars.next() {
                    hex.push(c2);
                }

                if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                    result.push(byte);
                } else {
                    // Invalid encoding, keep as-is
                    result.push(b'=');
                    result.extend(hex.as_bytes());
                }
            } else {
                result.push(ch as u8);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_headers_body_crlf() {
        let message = "From: test@example.com\r\nSubject: Test\r\n\r\nBody content";
        let (headers, body) = MimeParser::split_headers_body(message).unwrap();
        assert!(headers.contains("From:"));
        assert_eq!(body, "Body content");
    }

    #[test]
    fn test_split_headers_body_lf() {
        let message = "From: test@example.com\nSubject: Test\n\nBody content";
        let (headers, body) = MimeParser::split_headers_body(message).unwrap();
        assert!(headers.contains("From:"));
        assert_eq!(body, "Body content");
    }

    #[test]
    fn test_parse_headers() {
        let headers_str = "From: test@example.com\nSubject: Test Email\nContent-Type: text/plain";
        let headers = MimeParser::parse_headers(headers_str);

        assert_eq!(headers.get("from"), Some(&"test@example.com".to_string()));
        assert_eq!(headers.get("subject"), Some(&"Test Email".to_string()));
        assert_eq!(headers.get("content-type"), Some(&"text/plain".to_string()));
    }

    #[test]
    fn test_parse_headers_folded() {
        let headers_str = "Subject: This is a very long subject\n that spans multiple lines";
        let headers = MimeParser::parse_headers(headers_str);

        assert_eq!(
            headers.get("subject"),
            Some(&"This is a very long subject that spans multiple lines".to_string())
        );
    }

    #[test]
    fn test_extract_boundary() {
        let content_type = "multipart/mixed; boundary=\"----=_Part_123\"";
        let boundary = MimeParser::extract_boundary(content_type);
        assert_eq!(boundary, Some("----=_Part_123".to_string()));
    }

    #[test]
    fn test_extract_boundary_no_quotes() {
        let content_type = "multipart/mixed; boundary=simple_boundary";
        let boundary = MimeParser::extract_boundary(content_type);
        assert_eq!(boundary, Some("simple_boundary".to_string()));
    }

    #[test]
    fn test_extract_parameter() {
        let header = "attachment; filename=\"document.pdf\"";
        let filename = MimeParser::extract_parameter(header, "filename");
        assert_eq!(filename, Some("document.pdf".to_string()));
    }

    #[test]
    fn test_decode_base64() {
        let encoded = b"SGVsbG8gV29ybGQ="; // "Hello World"
        let decoded = MimeParser::decode_base64(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_decode_base64_with_whitespace() {
        let encoded = b"SGVs bG8g\nV29y bGQ="; // "Hello World" with whitespace
        let decoded = MimeParser::decode_base64(encoded).unwrap();
        assert_eq!(decoded, b"Hello World");
    }

    #[test]
    fn test_decode_quoted_printable() {
        let encoded = b"Hello=20World=21"; // "Hello World!"
        let decoded = MimeParser::decode_quoted_printable(encoded);
        assert_eq!(decoded, b"Hello World!");
    }

    #[test]
    fn test_decode_quoted_printable_soft_linebreak() {
        let encoded = b"Hello=\nWorld"; // "HelloWorld" with soft line break
        let decoded = MimeParser::decode_quoted_printable(encoded);
        assert_eq!(decoded, b"HelloWorld");
    }

    #[test]
    fn test_parse_simple_text_email() {
        let message = b"From: sender@example.com\nTo: recipient@example.com\nSubject: Test\n\nHello World";
        let parsed = MimeParser::parse(message).unwrap();

        assert_eq!(parsed.headers.get("from"), Some(&"sender@example.com".to_string()));
        assert_eq!(parsed.text_body, Some("Hello World".to_string()));
        assert_eq!(parsed.attachment_count(), 0);
    }

    #[test]
    fn test_parse_multipart_email() {
        let message = b"Content-Type: multipart/mixed; boundary=\"boundary123\"\n\n--boundary123\nContent-Type: text/plain\n\nText part\n--boundary123\nContent-Type: text/html\n\n<p>HTML part</p>\n--boundary123--";

        let parsed = MimeParser::parse(message).unwrap();

        assert_eq!(parsed.text_body, Some("Text part".to_string()));
        assert_eq!(parsed.html_body, Some("<p>HTML part</p>".to_string()));
    }

    #[test]
    fn test_parse_email_with_attachment() {
        let message = b"Content-Type: multipart/mixed; boundary=\"bound\"\n\n--bound\nContent-Type: text/plain\n\nBody\n--bound\nContent-Type: application/pdf\nContent-Disposition: attachment; filename=\"file.pdf\"\n\nPDF content\n--bound--";

        let parsed = MimeParser::parse(message).unwrap();

        assert_eq!(parsed.text_body, Some("Body".to_string()));
        assert_eq!(parsed.attachment_count(), 1);
        assert_eq!(parsed.attachments[0].filename, Some("file.pdf".to_string()));
        assert!(parsed.attachments[0].is_attachment);
    }
}
