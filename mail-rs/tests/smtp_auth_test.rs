//! Integration tests for SMTP AUTH

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use mail_rs::config::Config;
use mail_rs::security::Authenticator;
use mail_rs::smtp::SmtpServer;
use mail_rs::storage::MaildirStorage;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// Helper function to start SMTP server with AUTH enabled
async fn start_test_server_with_auth(
    port: u16,
) -> Result<(tokio::task::JoinHandle<()>, Arc<Authenticator>), Box<dyn std::error::Error>> {
    let tempdir = tempfile::tempdir()?;
    let maildir_path = tempdir.path().join("maildir");
    std::fs::create_dir_all(&maildir_path)?;

    // Create a temporary database file
    let db_file = tempfile::NamedTempFile::new()?;
    let db_path = db_file.path().to_path_buf();
    let db_url = format!("sqlite:{}", db_path.display());

    // Keep the temp file alive
    std::mem::forget(db_file);

    let mut config = Config::default();
    config.smtp.listen_addr = format!("127.0.0.1:{}", port);
    config.smtp.enable_auth = true;
    config.smtp.auth_database_url = Some(db_url.clone());
    config.smtp.require_auth = true;
    config.storage.maildir_path = maildir_path.to_str().unwrap().to_string();

    // Initialize authenticator and add test user
    let authenticator = Arc::new(Authenticator::new(&db_url).await?);
    authenticator
        .add_user("testuser@example.com", "testpass123")
        .await?;

    let storage = Arc::new(MaildirStorage::new(config.storage.maildir_path.clone()));
    let server = SmtpServer::with_security(config, storage).await?;

    let handle = tokio::spawn(async move {
        let _ = server.run().await;
    });

    // Give server time to start
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    Ok((handle, authenticator))
}

async fn connect_to_server(port: u16) -> Result<TcpStream, Box<dyn std::error::Error>> {
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
    Ok(stream)
}

async fn read_line(reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    line
}

async fn write_line(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    line: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    writer.write_all(format!("{}\r\n", line).as_bytes()).await?;
    Ok(())
}

#[tokio::test]
async fn test_auth_plain_success() {
    let port = 5025;
    let (_handle, _auth) = start_test_server_with_auth(port).await.unwrap();

    let stream = connect_to_server(port).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read greeting
    let greeting = read_line(&mut reader).await;
    assert!(greeting.starts_with("220"));

    // Send EHLO
    write_line(&mut write_half, "EHLO test.client").await.unwrap();

    // Read EHLO response lines
    let mut auth_advertised = false;
    loop {
        let line = read_line(&mut reader).await;
        if line.contains("AUTH") {
            auth_advertised = true;
        }
        if line.starts_with("250 ") {
            break;
        }
    }
    assert!(auth_advertised, "AUTH should be advertised in EHLO");

    // Authenticate with PLAIN mechanism
    // Format: \0username\0password
    let auth_string = format!("\0testuser@example.com\0testpass123");
    let auth_b64 = BASE64.encode(auth_string.as_bytes());

    write_line(&mut write_half, &format!("AUTH PLAIN {}", auth_b64))
        .await
        .unwrap();

    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("235"),
        "Expected 235 Authentication successful, got: {}",
        response
    );

    // Try to send mail (should work after authentication)
    write_line(&mut write_half, "MAIL FROM:<testuser@example.com>")
        .await
        .unwrap();
    let response = read_line(&mut reader).await;
    assert!(response.starts_with("250"), "MAIL FROM should succeed after AUTH");

    // Clean up
    write_line(&mut write_half, "QUIT").await.unwrap();
}

#[tokio::test]
async fn test_auth_plain_failure() {
    let port = 5026;
    let (_handle, _auth) = start_test_server_with_auth(port).await.unwrap();

    let stream = connect_to_server(port).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read greeting
    read_line(&mut reader).await;

    // Send EHLO
    write_line(&mut write_half, "EHLO test.client").await.unwrap();

    // Read EHLO response
    loop {
        let line = read_line(&mut reader).await;
        if line.starts_with("250 ") {
            break;
        }
    }

    // Authenticate with wrong password
    let auth_string = format!("\0testuser@example.com\0wrongpassword");
    let auth_b64 = BASE64.encode(auth_string.as_bytes());

    write_line(&mut write_half, &format!("AUTH PLAIN {}", auth_b64))
        .await
        .unwrap();

    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("535"),
        "Expected 535 Authentication failed, got: {}",
        response
    );

    // Clean up
    write_line(&mut write_half, "QUIT").await.unwrap();
}

#[tokio::test]
async fn test_auth_login_success() {
    let port = 5027;
    let (_handle, _auth) = start_test_server_with_auth(port).await.unwrap();

    let stream = connect_to_server(port).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read greeting
    read_line(&mut reader).await;

    // Send EHLO
    write_line(&mut write_half, "EHLO test.client").await.unwrap();

    // Read EHLO response
    loop {
        let line = read_line(&mut reader).await;
        if line.starts_with("250 ") {
            break;
        }
    }

    // Start LOGIN authentication
    write_line(&mut write_half, "AUTH LOGIN").await.unwrap();

    // Server should ask for username
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("334"),
        "Expected 334 prompt for username, got: {}",
        response
    );

    // Send username (base64 encoded)
    let username_b64 = BASE64.encode(b"testuser@example.com");
    write_line(&mut write_half, &username_b64).await.unwrap();

    // Server should ask for password
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("334"),
        "Expected 334 prompt for password, got: {}",
        response
    );

    // Send password (base64 encoded)
    let password_b64 = BASE64.encode(b"testpass123");
    write_line(&mut write_half, &password_b64).await.unwrap();

    // Check authentication success
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("235"),
        "Expected 235 Authentication successful, got: {}",
        response
    );

    // Clean up
    write_line(&mut write_half, "QUIT").await.unwrap();
}

#[tokio::test]
async fn test_auth_required_for_mail() {
    let port = 5028;
    let (_handle, _auth) = start_test_server_with_auth(port).await.unwrap();

    let stream = connect_to_server(port).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read greeting
    read_line(&mut reader).await;

    // Send EHLO
    write_line(&mut write_half, "EHLO test.client").await.unwrap();

    // Read EHLO response
    loop {
        let line = read_line(&mut reader).await;
        if line.starts_with("250 ") {
            break;
        }
    }

    // Try to send mail WITHOUT authentication (should fail)
    write_line(&mut write_half, "MAIL FROM:<sender@example.com>")
        .await
        .unwrap();

    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("530"),
        "Expected 530 Authentication required, got: {}",
        response
    );

    // Clean up
    write_line(&mut write_half, "QUIT").await.unwrap();
}

#[tokio::test]
async fn test_auth_unknown_mechanism() {
    let port = 5029;
    let (_handle, _auth) = start_test_server_with_auth(port).await.unwrap();

    let stream = connect_to_server(port).await.unwrap();
    let (read_half, mut write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // Read greeting
    read_line(&mut reader).await;

    // Send EHLO
    write_line(&mut write_half, "EHLO test.client").await.unwrap();

    // Read EHLO response
    loop {
        let line = read_line(&mut reader).await;
        if line.starts_with("250 ") {
            break;
        }
    }

    // Try unsupported mechanism
    write_line(&mut write_half, "AUTH CRAM-MD5").await.unwrap();

    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("504"),
        "Expected 504 mechanism not supported, got: {}",
        response
    );

    // Clean up
    write_line(&mut write_half, "QUIT").await.unwrap();
}
