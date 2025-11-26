// E2E Test 3: Email Conversation Complete
// Tests: Send → Receive → Reply → Thread

mod e2e;

use e2e::helpers::{TestEnv, TestResult, generate_test_id};
use e2e::websocket_client::WebSocketTestClient;
use e2e::smtp_client::SmtpTestClient;
use std::time::Instant;

#[tokio::test]
async fn test_e2e_3_email_conversation() {
    let start = Instant::now();
    let test_name = "E2E Test 3: Email Conversation".to_string();

    println!("\n🚀 Starting: {}", test_name);
    println!("{}", "=".repeat(80));

    let env = TestEnv::new();

    // Step 1: Setup
    println!("\n📋 Step 1: Setting up environment...");
    if let Err(e) = env.check_services().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Services check failed");
    }

    if let Err(e) = env.ensure_test_user().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("User setup failed");
    }

    if let Err(e) = env.clean_mailbox().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Mailbox cleanup failed");
    }

    // Step 2: Send initial email (external → test user)
    println!("\n📋 Step 2: Sending initial email...");
    let test_id = generate_test_id();
    let alice = "alice@example.com";
    let subject = format!("Question about project {}", test_id);
    let initial_body = "Hi! Can you send me the project details?";

    let mut smtp_client = match SmtpTestClient::connect(&env.mail_rs_url).await {
        Ok(client) => client,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("SMTP connection failed");
        }
    };

    if let Err(e) = smtp_client.send_email(alice, &env.test_user_email, &subject, initial_body).await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Initial email send failed");
    }

    if let Err(e) = smtp_client.quit().await {
        println!("⚠️  Warning: SMTP quit failed: {}", e);
    }

    println!("✅ Initial email sent from {}", alice);

    // Wait for email to arrive
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Step 3: Connect AI assistant and list emails
    println!("\n📋 Step 3: Listing emails via AI...");
    let ws_url = format!("{}/ws", env.ai_runtime_url.replace("http", "ws"));
    let mut ws_client = match WebSocketTestClient::connect(&ws_url).await {
        Ok(client) => client,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("WebSocket connection failed");
        }
    };

    if let Err(e) = ws_client.authenticate(&env.test_user_email).await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Authentication failed");
    }

    let list_response = match ws_client.chat("Liste mes emails").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("List emails failed");
        }
    };

    println!("📬 Emails listed: {}", list_response.final_content());

    // Verify email is present
    if let Some(tool_result) = list_response.get_tool_result("list_emails") {
        let result_str = tool_result.to_string();
        if !result_str.contains(alice) && !result_str.contains(&subject) {
            let result = TestResult::failure(
                test_name,
                format!("Email from {} not found in list", alice),
                start.elapsed(),
            );
            result.print();
            panic!("Email not found in list");
        }
        println!("✅ Email from {} found in list", alice);
    }

    // Step 4: Send reply via AI
    println!("\n📋 Step 4: Sending reply via AI...");
    let reply_prompt = format!(
        "Réponds à l'email de {} avec le sujet '{}'. Dis: 'Voici les détails du projet que tu as demandés.'",
        alice, subject
    );

    let reply_response = match ws_client.chat(&reply_prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("Reply failed");
        }
    };

    println!("📧 Reply sent: {}", reply_response.final_content());

    if !reply_response.has_tool_call("send_email") {
        let result = TestResult::failure(
            test_name,
            "send_email not called for reply".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Reply verification failed");
    }

    println!("✅ Reply email sent");

    // Step 5: Search for emails from Alice
    println!("\n📋 Step 5: Searching emails from Alice...");
    let search_prompt = format!("Recherche les emails de {}", alice);
    let search_response = match ws_client.chat(&search_prompt).await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("Search failed");
        }
    };

    println!("🔍 Search result: {}", search_response.final_content());

    if !search_response.has_tool_call("search_emails") && !search_response.has_tool_call("list_emails") {
        let result = TestResult::failure(
            test_name,
            "search_emails or list_emails not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Search verification failed");
    }

    println!("✅ Search completed successfully");

    // Close connection
    if let Err(e) = ws_client.close().await {
        println!("⚠️  Warning: Failed to close WebSocket: {}", e);
    }

    // Test passed!
    let result = TestResult::success(test_name, start.elapsed());
    result.print();
    println!("\n🎉 Test completed successfully!");
}
