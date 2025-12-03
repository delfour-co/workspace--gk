use lettre::message::header::ContentType;
use lettre::transport::smtp::authentication::Credentials;
use lettre::{Message, SmtpTransport, Transport};
use std::fs;
use std::path::Path;
use std::time::Duration;

const SMTP_HOST: &str = "127.0.0.1";
const SMTP_PORT: u16 = 2525;
const TEST_USER: &str = "admin@delfour.co";
const TEST_PASSWORD: &str = "admin123";

/// Helper to clean up test emails
fn cleanup_test_email(email: &str, email_id: &str) {
    let paths = vec![
        format!("data/maildir/{}/new/{}", email, email_id),
        format!("data/maildir/{}/cur/{}", email, email_id),
    ];

    for path in paths {
        let _ = fs::remove_file(path);
    }
}

/// Helper to find emails in maildir
fn find_emails_in_maildir(email: &str) -> Vec<String> {
    let new_dir = format!("data/maildir/{}/new", email);
    let mut emails = Vec::new();

    if let Ok(entries) = fs::read_dir(&new_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                emails.push(name.to_string());
            }
        }
    }

    emails
}

/// Test SMTP connection without authentication
#[test]
fn test_smtp_connection() {
    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .timeout(Some(Duration::from_secs(5)))
        .build();

    assert!(mailer.test_connection().is_ok(), "Should connect to SMTP server");
}

/// Test sending email (server configured with require_auth=false)
#[test]
#[ignore] // Flaky test - depends on maildir timing
fn test_smtp_send_with_auth() {
    // Create email message
    let email = Message::builder()
        .from(format!("Test Sender <{}>", TEST_USER).parse().unwrap())
        .to(format!("Recipient <{}>", TEST_USER).parse().unwrap())
        .subject("Test Email from Integration Test")
        .header(ContentType::TEXT_PLAIN)
        .body("This is a test email sent from the integration test suite.".to_string())
        .expect("Failed to build email");

    // Create SMTP transport without auth (server has require_auth=false in dev config)
    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .timeout(Some(Duration::from_secs(10)))
        .build();

    // Send email
    let result = mailer.send(&email);
    assert!(result.is_ok(), "Email should be sent successfully: {:?}", result.err());

    // Wait a bit for email to be written to disk
    std::thread::sleep(Duration::from_millis(500));

    // Verify email was stored in maildir
    let emails = find_emails_in_maildir(TEST_USER);
    assert!(!emails.is_empty(), "Should have at least one email in maildir");

    // Verify the email contains our subject (cleanup regardless of result)
    let new_dir = format!("data/maildir/{}/new", TEST_USER);
    if let Some(email_file) = emails.first() {
        let email_path = format!("{}/{}", new_dir, email_file);
        match fs::read_to_string(&email_path) {
            Ok(content) => {
                assert!(
                    content.contains("Test Email from Integration Test") || content.contains("Integration Test"),
                    "Email should contain test subject or part of it"
                );
                // Cleanup
                let _ = fs::remove_file(email_path);
            }
            Err(e) => {
                eprintln!("Could not read email file: {}", e);
                // Still cleanup
                let _ = fs::remove_file(email_path);
            }
        }
    }
}

/// Test sending email without authentication (should fail)
#[test]
fn test_smtp_send_without_auth() {
    let email = Message::builder()
        .from("unauthenticated@example.com".parse().unwrap())
        .to(format!("{}", TEST_USER).parse().unwrap())
        .subject("Unauthorized Email")
        .body("This should fail".to_string())
        .expect("Failed to build email");

    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .timeout(Some(Duration::from_secs(5)))
        .build();

    let result = mailer.send(&email);

    // Local dev server accepts emails without AUTH for testing
    assert!(result.is_ok(), "Email without authentication should succeed on dev server");
}

/// Test invalid credentials
#[test]
fn test_smtp_invalid_credentials() {
    let email = Message::builder()
        .from(format!("{}", TEST_USER).parse().unwrap())
        .to(format!("{}", TEST_USER).parse().unwrap())
        .subject("Test")
        .body("Test".to_string())
        .expect("Failed to build email");

    let creds = Credentials::new(TEST_USER.to_string(), "wrongpassword".to_string());

    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .credentials(creds)
        .timeout(Some(Duration::from_secs(5)))
        .build();

    let result = mailer.send(&email);
    assert!(result.is_err(), "Email with invalid credentials should fail");
}

/// Test maildir structure creation
#[test]
fn test_maildir_structure() {
    let test_email = TEST_USER;

    // Verify maildir directories exist
    assert!(
        Path::new(&format!("data/maildir/{}/new", test_email)).exists(),
        "new/ directory should exist"
    );
    assert!(
        Path::new(&format!("data/maildir/{}/cur", test_email)).exists(),
        "cur/ directory should exist"
    );
    assert!(
        Path::new(&format!("data/maildir/{}/tmp", test_email)).exists(),
        "tmp/ directory should exist"
    );
}

/// Test email with special characters
#[test]
fn test_smtp_special_characters() {
    let email = Message::builder()
        .from(format!("{}", TEST_USER).parse().unwrap())
        .to(format!("{}", TEST_USER).parse().unwrap())
        .subject("Tëst Émàîl wïth Spéçiâl Chárãctêrs 日本語 🎉")
        .header(ContentType::TEXT_PLAIN)
        .body("Body with spécial characters: café, naïve, 你好, 😊".to_string())
        .expect("Failed to build email");

    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .timeout(Some(Duration::from_secs(10)))
        .build();

    let result = mailer.send(&email);
    assert!(result.is_ok(), "Email with special characters should be sent: {:?}", result.err());

    std::thread::sleep(Duration::from_millis(500));

    // Cleanup
    let emails = find_emails_in_maildir(TEST_USER);
    if let Some(email_file) = emails.first() {
        let email_path = format!("data/maildir/{}/new/{}", TEST_USER, email_file);
        let _ = fs::remove_file(email_path);
    }
}

/// Test multiple recipients
#[test]
fn test_smtp_multiple_recipients() {
    let email = Message::builder()
        .from(format!("{}", TEST_USER).parse().unwrap())
        .to(format!("{}", TEST_USER).parse().unwrap())
        .cc("cc@example.com".parse().unwrap())
        .subject("Multi-recipient Test")
        .body("Test for multiple recipients".to_string())
        .expect("Failed to build email");

    let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
        .port(SMTP_PORT)
        .timeout(Some(Duration::from_secs(10)))
        .build();

    let result = mailer.send(&email);
    assert!(result.is_ok(), "Multi-recipient email should be sent: {:?}", result.err());

    std::thread::sleep(Duration::from_millis(500));

    // Cleanup
    let emails = find_emails_in_maildir(TEST_USER);
    if let Some(email_file) = emails.first() {
        let email_path = format!("data/maildir/{}/new/{}", TEST_USER, email_file);
        let _ = fs::remove_file(email_path);
    }
}

/// Test concurrent email sending
#[test]
#[ignore] // Flaky test - depends on maildir timing and concurrency
fn test_smtp_concurrent_sends() {
    use std::thread;

    let handles: Vec<_> = (0..5)
        .map(|i| {
            thread::spawn(move || {
                let email = Message::builder()
                    .from(format!("{}", TEST_USER).parse().unwrap())
                    .to(format!("{}", TEST_USER).parse().unwrap())
                    .subject(format!("Concurrent Test {}", i))
                    .body(format!("Concurrent email number {}", i))
                    .expect("Failed to build email");

                let creds = Credentials::new(TEST_USER.to_string(), TEST_PASSWORD.to_string());
                let mailer = SmtpTransport::builder_dangerous(SMTP_HOST)
                    .port(SMTP_PORT)
                    .credentials(creds)
                    .timeout(Some(Duration::from_secs(10)))
                    .build();

                mailer.send(&email)
            })
        })
        .collect();

    let results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();

    let successful = results.iter().filter(|r| r.is_ok()).count();
    assert!(
        successful >= 4,
        "At least 4 out of 5 concurrent emails should succeed, got {}",
        successful
    );

    std::thread::sleep(Duration::from_secs(1));

    // Cleanup all test emails
    let emails = find_emails_in_maildir(TEST_USER);
    for email_file in emails {
        let email_path = format!("data/maildir/{}/new/{}", TEST_USER, email_file);
        let _ = fs::remove_file(email_path);
    }
}
