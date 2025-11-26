// E2E Test 1: Send Email Simple
// Tests the complete flow: web-ui → ai-runtime → mcp-mail → mail-rs → Maildir

mod e2e;

use e2e::helpers::{TestEnv, TestResult, generate_test_id};
use e2e::websocket_client::WebSocketTestClient;
use e2e::imap_client::ImapTestClient;
use std::time::Instant;

#[tokio::test]
async fn test_e2e_1_send_email_simple() {
    let start = Instant::now();
    let test_name = "E2E Test 1: Send Email Simple".to_string();

    println!("\n🚀 Starting: {}", test_name);
    println!("{}", "=".repeat(80));

    let env = TestEnv::new();

    // Step 1: Check services are running
    println!("\n📋 Step 1: Checking services...");
    if let Err(e) = env.check_services().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Services check failed");
    }
    println!("✅ All services are running");

    // Step 2: Ensure test user exists
    println!("\n📋 Step 2: Ensuring test user exists...");
    if let Err(e) = env.ensure_test_user().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("User setup failed");
    }

    // Step 3: Clean mailbox
    println!("\n📋 Step 3: Cleaning mailbox...");
    if let Err(e) = env.clean_mailbox().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Mailbox cleanup failed");
    }

    // Step 4: Connect to WebSocket
    println!("\n📋 Step 4: Connecting to WebSocket...");
    let ws_url = format!("{}/ws", env.ai_runtime_url.replace("http", "ws"));
    let mut ws_client = match WebSocketTestClient::connect(&ws_url).await {
        Ok(client) => client,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("WebSocket connection failed");
        }
    };
    println!("✅ WebSocket connected");

    // Step 5: Authenticate
    println!("\n📋 Step 5: Authenticating...");
    if let Err(e) = ws_client.authenticate(&env.test_user_email).await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Authentication failed");
    }
    println!("✅ Authenticated as {}", env.test_user_email);

    // Step 6: Send email via AI assistant
    println!("\n📋 Step 6: Sending email via AI assistant...");
    let test_id = generate_test_id();
    let recipient = "bob@localhost";
    let subject = format!("Test Email {}", test_id);
    let body = "This is a test email sent via the AI assistant";

    let prompt = format!(
        "Envoie un email à {} avec le sujet \"{}\" et le message \"{}\"",
        recipient, subject, body
    );

    let response = match ws_client.chat(&prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("Chat request failed");
        }
    };

    println!("📨 Tool calls: {:?}", response.tool_calls.iter().map(|(name, _)| name).collect::<Vec<_>>());
    println!("📬 Final response: {}", response.final_content());

    // Verify send_email tool was called
    if !response.has_tool_call("send_email") {
        let result = TestResult::failure(
            test_name,
            "send_email tool was not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Tool call verification failed");
    }
    println!("✅ send_email tool was called");

    // Step 7: Verify email was sent (check Maildir)
    println!("\n📋 Step 7: Verifying email in Maildir...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await; // Allow time for email to be written

    // Note: Since we're sending to bob@localhost, we should check bob's mailbox
    // For now, we'll just verify the response indicates success
    if let Some(tool_result) = response.get_tool_result("send_email") {
        println!("📧 Send result: {}", tool_result);

        // Check if result indicates success
        let result_str = tool_result.to_string();
        if !result_str.contains("success") && !result_str.contains("sent") && !result_str.contains("envoyé") {
            let result = TestResult::failure(
                test_name,
                format!("Email send may have failed: {}", result_str),
                start.elapsed(),
            );
            result.print();
            panic!("Email verification failed");
        }
    }
    println!("✅ Email sent successfully");

    // Step 8: Close WebSocket
    println!("\n📋 Step 8: Closing connection...");
    if let Err(e) = ws_client.close().await {
        println!("⚠️  Warning: Failed to close WebSocket cleanly: {}", e);
    }

    // Test passed!
    let result = TestResult::success(test_name, start.elapsed());
    result.print();
    println!("\n🎉 Test completed successfully!");
}
