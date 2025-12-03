use serde_json::json;
use std::fs;
use std::path::Path;

/// Test helper to create test maildir structure
fn setup_test_maildir(email: &str) -> String {
    let test_dir = format!("mail-rs/data/maildir/{}", email);
    let new_dir = format!("{}/new", test_dir);
    let cur_dir = format!("{}/cur", test_dir);
    let tmp_dir = format!("{}/tmp", test_dir);

    // Create directories
    fs::create_dir_all(&new_dir).ok();
    fs::create_dir_all(&cur_dir).ok();
    fs::create_dir_all(&tmp_dir).ok();

    test_dir
}

/// Test helper to create a test email
fn create_test_email(email: &str, email_id: &str, from: &str, subject: &str, body: &str) {
    let email_path = format!("mail-rs/data/maildir/{}/new/{}", email, email_id);
    let content = format!(
        "From: {}\nTo: {}\nSubject: {}\n\n{}",
        from, email, subject, body
    );
    fs::write(email_path, content).ok();
}

/// Cleanup test maildir
fn cleanup_test_maildir(email: &str) {
    let test_dir = format!("mail-rs/data/maildir/{}", email);
    fs::remove_dir_all(test_dir).ok();
}

#[tokio::test]
async fn test_list_emails_empty() {
    let test_email = "test-empty@example.com";
    setup_test_maildir(test_email);

    // Simulate MCP request
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "list_emails",
                "arguments": {
                    "email": test_email,
                    "limit": 10
                }
            },
            "id": 1
        }))
        .send()
        .await;

    cleanup_test_maildir(test_email);

    assert!(response.is_ok(), "Request should succeed");
    let response = response.unwrap();
    assert!(response.status().is_success(), "Should return 200 OK");
}

#[tokio::test]
async fn test_list_emails_with_messages() {
    let test_email = "test-messages@example.com";
    setup_test_maildir(test_email);

    // Create test emails
    create_test_email(
        test_email,
        "1234567890.001.test",
        "sender1@example.com",
        "Test Email 1",
        "This is the first test email",
    );
    create_test_email(
        test_email,
        "1234567891.002.test",
        "sender2@example.com",
        "Test Email 2",
        "This is the second test email",
    );

    // Test list_emails
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "list_emails",
                "arguments": {
                    "email": test_email,
                    "limit": 10
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let emails = body["result"]["emails"].as_array().expect("No emails array");

    assert_eq!(emails.len(), 2, "Should have 2 emails");

    // Verify both emails are present (order may vary)
    let subjects: Vec<&str> = emails
        .iter()
        .map(|e| e["subject"].as_str().unwrap())
        .collect();
    assert!(subjects.contains(&"Test Email 1"), "Should contain Test Email 1");
    assert!(subjects.contains(&"Test Email 2"), "Should contain Test Email 2");

    cleanup_test_maildir(test_email);
}

#[tokio::test]
async fn test_read_email() {
    let test_email = "test-read@example.com";
    setup_test_maildir(test_email);

    let email_id = "1234567890.003.test";
    create_test_email(
        test_email,
        email_id,
        "sender@example.com",
        "Test Subject",
        "Test body content",
    );

    // Test read_email
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "read_email",
                "arguments": {
                    "email": test_email,
                    "email_id": email_id
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let subject = body["result"]["headers"]["Subject"].as_str().expect("No subject");
    let body_text = body["result"]["body"].as_str().expect("No body");

    assert_eq!(subject, "Test Subject");
    assert!(body_text.contains("Test body content"));

    cleanup_test_maildir(test_email);
}

#[tokio::test]
async fn test_get_email_count() {
    let test_email = "test-count@example.com";
    setup_test_maildir(test_email);

    // Create 3 test emails
    for i in 0..3 {
        create_test_email(
            test_email,
            &format!("test-{}.eml", i),
            "sender@example.com",
            &format!("Email {}", i),
            "Test body",
        );
    }

    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "get_email_count",
                "arguments": {
                    "email": test_email
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let count = body["result"]["count"].as_u64().unwrap();

    assert_eq!(count, 3, "Should have 3 unread emails");

    cleanup_test_maildir(test_email);
}

#[tokio::test]
async fn test_mark_as_read() {
    let test_email = "test-mark@example.com";
    setup_test_maildir(test_email);

    let email_id = "test-mark.eml";
    create_test_email(
        test_email,
        email_id,
        "sender@example.com",
        "Test",
        "Body",
    );

    // Mark as read
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "mark_as_read",
                "arguments": {
                    "email": test_email,
                    "email_id": email_id
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let success = body["result"]["success"].as_bool().unwrap();

    assert!(success, "mark_as_read should succeed");

    // Verify email moved from new/ to cur/
    let new_path = format!("mail-rs/data/maildir/{}/new/{}", test_email, email_id);
    let cur_path = format!("mail-rs/data/maildir/{}/cur/{}", test_email, email_id);

    assert!(!Path::new(&new_path).exists(), "Email should not be in new/");
    assert!(Path::new(&cur_path).exists(), "Email should be in cur/");

    cleanup_test_maildir(test_email);
}

#[tokio::test]
async fn test_delete_email() {
    let test_email = "test-delete@example.com";
    setup_test_maildir(test_email);

    let email_id = "test-delete.eml";
    create_test_email(
        test_email,
        email_id,
        "sender@example.com",
        "Test",
        "Body",
    );

    let email_path = format!("mail-rs/data/maildir/{}/new/{}", test_email, email_id);
    assert!(Path::new(&email_path).exists(), "Email should exist before deletion");

    // Delete email
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "delete_email",
                "arguments": {
                    "email": test_email,
                    "email_id": email_id
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let success = body["result"]["success"].as_bool().unwrap();

    assert!(success, "delete_email should succeed");
    assert!(!Path::new(&email_path).exists(), "Email should be deleted");

    cleanup_test_maildir(test_email);
}

#[tokio::test]
async fn test_search_emails() {
    let test_email = "test-search@example.com";
    setup_test_maildir(test_email);

    create_test_email(
        test_email,
        "search1.eml",
        "sender@example.com",
        "Important Meeting",
        "Please attend the meeting tomorrow",
    );
    create_test_email(
        test_email,
        "search2.eml",
        "other@example.com",
        "Random Subject",
        "This is unrelated content",
    );

    // Search for "meeting"
    let client = reqwest::Client::new();
    let response = client
        .post("http://localhost:8090/mcp")
        .json(&json!({
            "jsonrpc": "2.0",
            "method": "tools/call",
            "params": {
                "name": "search_emails",
                "arguments": {
                    "email": test_email,
                    "query": "meeting"
                }
            },
            "id": 1
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    let emails = body["result"]["emails"].as_array().unwrap();

    assert_eq!(emails.len(), 1, "Should find 1 email with 'meeting'");
    assert!(emails[0]["subject"]
        .as_str()
        .unwrap()
        .contains("Meeting"));

    cleanup_test_maildir(test_email);
}
