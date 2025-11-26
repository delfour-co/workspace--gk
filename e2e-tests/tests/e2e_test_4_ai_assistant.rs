// E2E Test 4: AI Assistant - Various Requests
// Tests different AI assistant capabilities

mod e2e;

use e2e::helpers::{TestEnv, TestResult, generate_test_id};
use e2e::websocket_client::WebSocketTestClient;
use e2e::smtp_client::SmtpTestClient;
use std::time::Instant;

#[tokio::test]
async fn test_e2e_4_ai_assistant_requests() {
    let start = Instant::now();
    let test_name = "E2E Test 4: AI Assistant Requests".to_string();

    println!("\n🚀 Starting: {}", test_name);
    println!("{}", "=".repeat(80));

    let env = TestEnv::new();

    // Setup
    println!("\n📋 Setup: Preparing environment...");
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

    // Seed some test emails
    println!("\n📋 Setup: Seeding test emails...");
    let test_id = generate_test_id();
    let test_emails = vec![
        ("alice@example.com", format!("Project Update {}", test_id), "The project is going well"),
        ("bob@company.com", format!("Meeting Request {}", test_id), "Can we meet tomorrow?"),
        ("charlie@test.com", format!("Bug Report {}", test_id), "I found a critical bug"),
    ];

    for (from, subject, body) in &test_emails {
        let mut smtp = SmtpTestClient::connect(&env.mail_rs_url).await.unwrap();
        smtp.send_email(from, &env.test_user_email, subject, body).await.unwrap();
        smtp.quit().await.ok();
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    println!("✅ {} test emails seeded", test_emails.len());

    // Connect WebSocket
    println!("\n📋 Connecting to AI assistant...");
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

    println!("✅ Connected and authenticated");

    // Test 4.1: List all emails
    println!("\n📋 Test 4.1: List all emails");
    let response = match ws_client.chat("Liste tous mes emails").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("List emails failed");
        }
    };

    if !response.has_tool_call("list_emails") {
        let result = TestResult::failure(
            test_name.clone(),
            "list_emails not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Test 4.1 failed");
    }

    println!("✅ Test 4.1 passed: list_emails called");
    println!("   Response: {}", response.final_content());

    // Test 4.2: Search specific sender
    println!("\n📋 Test 4.2: Search emails from specific sender");
    let response = match ws_client.chat("Recherche les emails d'Alice").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("Search emails failed");
        }
    };

    let has_search = response.has_tool_call("search_emails") || response.has_tool_call("list_emails");
    if !has_search {
        let result = TestResult::failure(
            test_name.clone(),
            "search_emails or list_emails not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Test 4.2 failed");
    }

    println!("✅ Test 4.2 passed: search functionality used");
    println!("   Response: {}", response.final_content());

    // Test 4.3: Search by keyword
    println!("\n📋 Test 4.3: Search by keyword");
    let response = match ws_client.chat("Cherche les emails qui parlent de bug").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("Search by keyword failed");
        }
    };

    let has_search = response.has_tool_call("search_emails") || response.has_tool_call("list_emails");
    if !has_search {
        let result = TestResult::failure(
            test_name.clone(),
            "search not performed".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Test 4.3 failed");
    }

    println!("✅ Test 4.3 passed: keyword search performed");
    println!("   Response: {}", response.final_content());

    // Test 4.4: Compose new email
    println!("\n📋 Test 4.4: Compose and send new email");
    let response = match ws_client.chat(
        "Envoie un email à team@example.com avec le sujet 'Status Update' et dis 'All tasks completed'"
    ).await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("Compose email failed");
        }
    };

    if !response.has_tool_call("send_email") {
        let result = TestResult::failure(
            test_name.clone(),
            "send_email not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Test 4.4 failed");
    }

    println!("✅ Test 4.4 passed: send_email called");
    println!("   Response: {}", response.final_content());

    // Test 4.5: Count emails (natural language)
    println!("\n📋 Test 4.5: Natural language query - count");
    let response = match ws_client.chat("Combien d'emails j'ai reçus?").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("Count query failed");
        }
    };

    if !response.has_tool_call("list_emails") {
        let result = TestResult::failure(
            test_name.clone(),
            "list_emails not called for count".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Test 4.5 failed");
    }

    println!("✅ Test 4.5 passed: count query handled");
    println!("   Response: {}", response.final_content());

    // Test 4.6: Empty result (no emails matching)
    println!("\n📋 Test 4.6: Search with no results");
    let response = match ws_client.chat("Cherche les emails de nonexistent@example.com").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name.clone(), e, start.elapsed());
            result.print();
            panic!("Empty search failed");
        }
    };

    println!("✅ Test 4.6 passed: empty result handled gracefully");
    println!("   Response: {}", response.final_content());

    // Close connection
    if let Err(e) = ws_client.close().await {
        println!("⚠️  Warning: Failed to close WebSocket: {}", e);
    }

    // Test passed!
    let result = TestResult::success(test_name, start.elapsed());
    result.print();
    println!("\n🎉 All AI assistant tests passed!");
    println!("   ✅ Test 4.1: List emails");
    println!("   ✅ Test 4.2: Search by sender");
    println!("   ✅ Test 4.3: Search by keyword");
    println!("   ✅ Test 4.4: Send email");
    println!("   ✅ Test 4.5: Count emails");
    println!("   ✅ Test 4.6: Empty results");
}
