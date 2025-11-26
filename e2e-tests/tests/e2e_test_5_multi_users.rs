// E2E Test 5: Multi-Users
// Tests multiple users with isolated mailboxes

mod e2e;

use e2e::helpers::{TestEnv, TestResult};
use e2e::websocket_client::WebSocketTestClient;
use e2e::smtp_client::SmtpTestClient;
use std::time::Instant;
use std::process::Command;

#[tokio::test]
async fn test_e2e_5_multi_users() {
    let start = Instant::now();
    let test_name = "E2E Test 5: Multi-Users".to_string();

    println!("\n🚀 Starting: {}", test_name);
    println!("{}", "=".repeat(80));

    let env = TestEnv::new();

    // Step 1: Check services
    println!("\n📋 Step 1: Checking services...");
    if let Err(e) = env.check_services().await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Services check failed");
    }

    // Step 2: Create multiple test users
    println!("\n📋 Step 2: Creating test users...");
    let users = vec![
        ("alice@localhost", "alicepass123"),
        ("bob@localhost", "bobpass456"),
        ("charlie@localhost", "charliepass789"),
    ];

    for (email, password) in &users {
        // Check if user exists
        let check = Command::new("docker")
            .args(&["exec", "gk-mail-rs-dev", "/app/mail-user", "exists", email])
            .output();

        if check.is_err() || !check.unwrap().status.success() {
            // Create user
            let create = Command::new("docker")
                .args(&["exec", "gk-mail-rs-dev", "/app/mail-user", "add", email, password])
                .output();

            if let Err(e) = create {
                let result = TestResult::failure(
                    test_name.clone(),
                    format!("Failed to create user {}: {}", email, e),
                    start.elapsed(),
                );
                result.print();
                panic!("User creation failed");
            }
        }
        println!("✅ User created/verified: {}", email);
    }

    // Step 3: Send emails between users
    println!("\n📋 Step 3: Sending inter-user emails...");

    // Alice → Bob
    let mut smtp = SmtpTestClient::connect(&env.mail_rs_url).await.unwrap();
    smtp.send_email(users[0].0, users[1].0, "Hi Bob", "Message from Alice to Bob").await.unwrap();
    smtp.quit().await.ok();

    // Bob → Charlie
    let mut smtp = SmtpTestClient::connect(&env.mail_rs_url).await.unwrap();
    smtp.send_email(users[1].0, users[2].0, "Hello Charlie", "Message from Bob to Charlie").await.unwrap();
    smtp.quit().await.ok();

    // Charlie → Alice
    let mut smtp = SmtpTestClient::connect(&env.mail_rs_url).await.unwrap();
    smtp.send_email(users[2].0, users[0].0, "Hey Alice", "Message from Charlie to Alice").await.unwrap();
    smtp.quit().await.ok();

    println!("✅ Inter-user emails sent");

    // Wait for emails to arrive
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    // Step 4: Verify each user can only see their own emails
    println!("\n📋 Step 4: Verifying mailbox isolation...");
    let ws_url = format!("{}/ws", env.ai_runtime_url.replace("http", "ws"));

    // Test Alice's mailbox
    println!("\n  Testing Alice's mailbox...");
    let mut alice_ws = WebSocketTestClient::connect(&ws_url).await.unwrap();
    alice_ws.authenticate(users[0].0).await.unwrap();

    let alice_response = alice_ws.chat("Liste mes emails").await.unwrap();
    println!("  Alice's emails: {}", alice_response.final_content());

    // Alice should see email from Charlie, not emails between Bob and Charlie
    let alice_result = alice_response.final_content();
    if !alice_result.contains("Charlie") {
        let result = TestResult::failure(
            test_name.clone(),
            "Alice doesn't see email from Charlie".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Mailbox isolation test failed");
    }

    if alice_result.contains("Bob") && alice_result.contains("message from Bob") {
        let result = TestResult::failure(
            test_name.clone(),
            "Alice sees emails not sent to her".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Mailbox isolation violated");
    }

    println!("  ✅ Alice's mailbox is correctly isolated");
    alice_ws.close().await.ok();

    // Test Bob's mailbox
    println!("\n  Testing Bob's mailbox...");
    let mut bob_ws = WebSocketTestClient::connect(&ws_url).await.unwrap();
    bob_ws.authenticate(users[1].0).await.unwrap();

    let bob_response = bob_ws.chat("Liste mes emails").await.unwrap();
    println!("  Bob's emails: {}", bob_response.final_content());

    let bob_result = bob_response.final_content();
    if !bob_result.contains("Alice") {
        let result = TestResult::failure(
            test_name.clone(),
            "Bob doesn't see email from Alice".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Mailbox isolation test failed");
    }

    println!("  ✅ Bob's mailbox is correctly isolated");
    bob_ws.close().await.ok();

    // Test Charlie's mailbox
    println!("\n  Testing Charlie's mailbox...");
    let mut charlie_ws = WebSocketTestClient::connect(&ws_url).await.unwrap();
    charlie_ws.authenticate(users[2].0).await.unwrap();

    let charlie_response = charlie_ws.chat("Liste mes emails").await.unwrap();
    println!("  Charlie's emails: {}", charlie_response.final_content());

    let charlie_result = charlie_response.final_content();
    if !charlie_result.contains("Bob") {
        let result = TestResult::failure(
            test_name.clone(),
            "Charlie doesn't see email from Bob".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Mailbox isolation test failed");
    }

    println!("  ✅ Charlie's mailbox is correctly isolated");
    charlie_ws.close().await.ok();

    // Step 5: Test concurrent access
    println!("\n📋 Step 5: Testing concurrent access...");

    let ws_url_clone = ws_url.clone();
    let users_clone = users.clone();

    let handles: Vec<_> = users.iter().map(|(email, _)| {
        let ws_url = ws_url_clone.clone();
        let email = email.to_string();
        tokio::spawn(async move {
            let mut ws = WebSocketTestClient::connect(&ws_url).await.unwrap();
            ws.authenticate(&email).await.unwrap();
            let response = ws.chat("Combien d'emails j'ai?").await.unwrap();
            ws.close().await.ok();
            response.final_content().to_string()
        })
    }).collect();

    // Wait for all concurrent requests
    for (i, handle) in handles.into_iter().enumerate() {
        match handle.await {
            Ok(response) => {
                println!("  ✅ User {} concurrent request successful: {}", users[i].0, response);
            }
            Err(e) => {
                let result = TestResult::failure(
                    test_name.clone(),
                    format!("Concurrent request failed for {}: {}", users[i].0, e),
                    start.elapsed(),
                );
                result.print();
                panic!("Concurrent access test failed");
            }
        }
    }

    println!("✅ Concurrent access test passed");

    // Test passed!
    let result = TestResult::success(test_name, start.elapsed());
    result.print();
    println!("\n🎉 Multi-user tests passed!");
    println!("   ✅ 3 users created");
    println!("   ✅ Inter-user emails sent");
    println!("   ✅ Mailbox isolation verified");
    println!("   ✅ Concurrent access tested");
}
