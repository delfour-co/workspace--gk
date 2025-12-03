use futures::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::{connect_async, tungstenite::Message};

/// Test the summary generation endpoint
#[tokio::test]
async fn test_generate_summary() {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:8888/api/generate-summary")
        .json(&json!({
            "user_email": "test@example.com",
            "email_id": "test-123",
            "from": "sender@example.com",
            "subject": "Test Email",
            "body": "This is a test email body with some important information about a meeting tomorrow at 3pm."
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success(), "Summary generation should succeed");

    let body: serde_json::Value = response.json().await.expect("Invalid JSON");
    assert!(body["summary"].is_string(), "Should have summary field");

    let summary = body["summary"].as_str().unwrap();
    assert!(!summary.is_empty(), "Summary should not be empty");
}

/// Test WebSocket connection and authentication
#[tokio::test]
async fn test_websocket_authentication() {
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send auth message
    let auth_msg = json!({
        "type": "auth",
        "email": "test-ws@example.com"
    });

    write
        .send(Message::Text(auth_msg.to_string()))
        .await
        .expect("Failed to send auth message");

    // Wait for auth success response
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        read.next()
    ).await;

    assert!(timeout.is_ok(), "Should receive response within timeout");

    if let Some(Ok(Message::Text(text))) = timeout.unwrap() {
        let response: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(response["type"], "auth_success", "Should receive auth_success message");
        assert_eq!(response["email"], "test-ws@example.com", "Should echo back email");
    } else {
        panic!("Expected text message");
    }
}

/// Test chat message handling
#[tokio::test]
async fn test_websocket_chat() {
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Authenticate first
    let auth_msg = json!({
        "type": "auth",
        "email": "test-chat@example.com"
    });

    write.send(Message::Text(auth_msg.to_string())).await.ok();

    // Wait for auth success message
    let auth_response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        read.next()
    ).await;

    assert!(auth_response.is_ok(), "Should receive auth response");

    // Send chat message with a simple question that doesn't require tools
    let chat_msg = json!({
        "type": "chat",
        "message": "Dis moi bonjour"
    });

    write.send(Message::Text(chat_msg.to_string())).await.expect("Failed to send chat");

    // Just verify we get ANY response within reasonable time (LLM can be slow)
    let mut got_response = false;
    let timeout_duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            read.next()
        ).await;

        if let Ok(Some(Ok(Message::Text(_)))) = timeout {
            got_response = true;
            break;
        }
    }

    // We just need to verify the system responds
    // If no response after 120s, the LLM might be unavailable or overloaded
    assert!(got_response, "Should receive at least one response from LLM (LLM must be running)");
}

/// Test unauthenticated chat rejection
#[tokio::test]
async fn test_websocket_unauthenticated_chat() {
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send chat message WITHOUT authentication
    let chat_msg = json!({
        "type": "chat",
        "message": "Test message"
    });

    write.send(Message::Text(chat_msg.to_string())).await.expect("Failed to send");

    // Should receive error message
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        read.next()
    ).await;

    assert!(timeout.is_ok(), "Should receive error response");

    if let Some(Ok(Message::Text(text))) = timeout.unwrap() {
        let response: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");
        assert_eq!(response["type"], "error", "Should receive error message");
        assert!(response["message"].as_str().unwrap().contains("authenticated"),
                "Error should mention authentication");
    }
}

/// Test MCP tool calling through chat
#[tokio::test]
async fn test_websocket_tool_calling() {
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Authenticate
    let auth_msg = json!({
        "type": "auth",
        "email": "admin@delfour.co"
    });

    write.send(Message::Text(auth_msg.to_string())).await.ok();

    // Wait for auth response
    let auth_response = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        read.next()
    ).await;

    assert!(auth_response.is_ok(), "Should receive auth response");

    // Send message that could trigger tool call
    let chat_msg = json!({
        "type": "chat",
        "message": "Liste mes emails"
    });

    write.send(Message::Text(chat_msg.to_string())).await.expect("Failed to send");

    // Just verify we get a response - tool calling depends on LLM behavior
    let mut got_response = false;
    let timeout_duration = std::time::Duration::from_secs(120);
    let start = std::time::Instant::now();

    while start.elapsed() < timeout_duration {
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            read.next()
        ).await;

        if let Ok(Some(Ok(Message::Text(_)))) = timeout {
            got_response = true;
            break;
        }
    }

    // The system should respond in some way (tool call, chunk, or done)
    assert!(got_response, "Should have received a response for email listing request (LLM must be running)");
}

/// Test summary storage and retrieval
#[tokio::test]
async fn test_summary_persistence() {
    let client = reqwest::Client::new();
    let test_email = "test-persist@example.com";

    // Generate a summary
    let response = client
        .post("http://localhost:8888/api/generate-summary")
        .json(&json!({
            "user_email": test_email,
            "email_id": "persist-123",
            "from": "sender@example.com",
            "subject": "Persistence Test",
            "body": "This email tests summary persistence in the database."
        }))
        .send()
        .await
        .expect("Request failed");

    assert!(response.status().is_success());

    // Connect via WebSocket and authenticate
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Authenticate with the same email
    let auth_msg = json!({
        "type": "auth",
        "email": test_email
    });

    write.send(Message::Text(auth_msg.to_string())).await.ok();

    // Should receive auth_success followed by email_summaries
    let mut found_summaries = false;

    for _ in 0..3 {
        let timeout = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            read.next()
        ).await;

        if let Ok(Some(Ok(Message::Text(text)))) = timeout {
            let msg: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON");

            if msg["type"] == "email_summaries" {
                found_summaries = true;
                let count = msg["count"].as_u64().unwrap();
                assert!(count > 0, "Should have at least one summary");
                break;
            }
        }
    }

    assert!(found_summaries, "Should receive stored summaries on authentication");
}

/// Test invalid JSON handling
#[tokio::test]
async fn test_websocket_invalid_json() {
    let (ws_stream, _) = connect_async("ws://localhost:8888/ws")
        .await
        .expect("Failed to connect");

    let (mut write, mut read) = ws_stream.split();

    // Send invalid JSON
    write.send(Message::Text("{invalid json}".to_string())).await.ok();

    // Should receive error message
    let timeout = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        read.next()
    ).await;

    assert!(timeout.is_ok(), "Should receive error response");

    if let Some(Ok(Message::Text(text))) = timeout.unwrap() {
        let response: serde_json::Value = serde_json::from_str(&text).expect("Invalid JSON response");
        assert_eq!(response["type"], "error", "Should receive error message");
    }
}
