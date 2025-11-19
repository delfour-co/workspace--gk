use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;
use tokio::time::sleep;

/// Helper function to start a test SMTP server
async fn start_test_server() -> SocketAddr {
    use mail_rs::config::Config;
    use mail_rs::smtp::SmtpServer;
    use mail_rs::storage::MaildirStorage;

    let config = Config::default();
    let storage = Arc::new(MaildirStorage::new(config.storage.maildir_path.clone()));
    let server = SmtpServer::new(config, storage);

    let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let local_addr = listener.local_addr().unwrap();

    tokio::spawn(async move {
        loop {
            if let Ok((socket, _)) = listener.accept().await {
                let session = mail_rs::smtp::SmtpSession::new(
                    "test.localhost".to_string(),
                    Arc::new(mail_rs::storage::MaildirStorage::new("/tmp/test-maildir".to_string())),
                    10 * 1024 * 1024,
                );
                tokio::spawn(async move {
                    let _ = session.handle(socket).await;
                });
            }
        }
    });

    // Give the server a moment to start
    sleep(Duration::from_millis(100)).await;

    local_addr
}

/// Helper to read a line from the stream
async fn read_line(reader: &mut BufReader<tokio::net::tcp::ReadHalf<'_>>) -> String {
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    line
}

/// Helper to write a line to the stream
async fn write_line(
    writer: &mut tokio::net::tcp::WriteHalf<'_>,
    line: &str,
) -> Result<(), std::io::Error> {
    writer.write_all(format!("{}\r\n", line).as_bytes()).await
}

#[tokio::test]
async fn test_smtp_greeting() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, _writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let greeting = read_line(&mut reader).await;
    assert!(greeting.starts_with("220"), "Expected 220 greeting");
}

#[tokio::test]
async fn test_smtp_ehlo() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // Send EHLO
    write_line(&mut writer, "EHLO test.client").await.unwrap();

    // Read response
    let response = read_line(&mut reader).await;
    assert!(response.starts_with("250"), "Expected 250 response");
}

#[tokio::test]
async fn test_smtp_helo() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // Send HELO
    write_line(&mut writer, "HELO test.client").await.unwrap();

    // Read response
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("250"),
        "Expected 250 response, got: {}",
        response
    );
}

#[tokio::test]
async fn test_smtp_quit() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // Send QUIT
    write_line(&mut writer, "QUIT").await.unwrap();

    // Read response
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("221"),
        "Expected 221 response, got: {}",
        response
    );
}

#[tokio::test]
async fn test_smtp_invalid_sequence() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // Try to send MAIL FROM without HELO/EHLO
    write_line(&mut writer, "MAIL FROM:<test@example.com>")
        .await
        .unwrap();

    // Read response - should be error
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("503") || response.starts_with("5"),
        "Expected error response for invalid sequence, got: {}",
        response
    );
}

#[tokio::test]
async fn test_smtp_complete_transaction() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // EHLO
    write_line(&mut writer, "EHLO test.client").await.unwrap();
    let _response = read_line(&mut reader).await;

    // MAIL FROM
    write_line(&mut writer, "MAIL FROM:<sender@example.com>")
        .await
        .unwrap();
    let response = read_line(&mut reader).await;
    assert!(response.starts_with("250"), "MAIL FROM failed: {}", response);

    // RCPT TO
    write_line(&mut writer, "RCPT TO:<recipient@localhost>")
        .await
        .unwrap();
    let response = read_line(&mut reader).await;
    assert!(response.starts_with("250"), "RCPT TO failed: {}", response);

    // DATA
    write_line(&mut writer, "DATA").await.unwrap();
    let response = read_line(&mut reader).await;
    assert!(response.starts_with("354"), "DATA failed: {}", response);

    // Send email content
    write_line(&mut writer, "Subject: Test Email").await.unwrap();
    write_line(&mut writer, "").await.unwrap();
    write_line(&mut writer, "This is a test email.").await.unwrap();
    write_line(&mut writer, ".").await.unwrap();

    // Read response
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("250"),
        "Message acceptance failed: {}",
        response
    );

    // QUIT
    write_line(&mut writer, "QUIT").await.unwrap();
    let _response = read_line(&mut reader).await;
}

#[tokio::test]
async fn test_smtp_invalid_email_addresses() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // EHLO
    write_line(&mut writer, "EHLO test.client").await.unwrap();
    let _response = read_line(&mut reader).await;

    // Try invalid email in MAIL FROM
    write_line(&mut writer, "MAIL FROM:<invalid-email>")
        .await
        .unwrap();
    let response = read_line(&mut reader).await;
    assert!(
        response.starts_with("4") || response.starts_with("5"),
        "Expected error for invalid email, got: {}",
        response
    );
}

#[tokio::test]
async fn test_smtp_too_many_recipients() {
    let addr = start_test_server().await;
    let stream = TcpStream::connect(addr).await.unwrap();
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let _greeting = read_line(&mut reader).await;

    // EHLO
    write_line(&mut writer, "EHLO test.client").await.unwrap();
    let _response = read_line(&mut reader).await;

    // MAIL FROM
    write_line(&mut writer, "MAIL FROM:<sender@example.com>")
        .await
        .unwrap();
    let _response = read_line(&mut reader).await;

    // Add 101 recipients (max is 100)
    for i in 0..101 {
        write_line(&mut writer, &format!("RCPT TO:<user{}@localhost>", i))
            .await
            .unwrap();
        let response = read_line(&mut reader).await;

        if i >= 100 {
            // Should be rejected
            assert!(
                response.starts_with("452") || response.starts_with("4"),
                "Expected rejection after 100 recipients, got: {}",
                response
            );
            break;
        } else {
            assert!(
                response.starts_with("250"),
                "RCPT {} failed: {}",
                i,
                response
            );
        }
    }
}
