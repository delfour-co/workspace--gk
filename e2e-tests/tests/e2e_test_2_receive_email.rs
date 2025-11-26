// E2E Test 2: Receive Email External
// Tests receiving an email from an external SMTP client

mod e2e;

use e2e::helpers::{TestEnv, TestResult, generate_test_id};
use e2e::smtp_client::SmtpTestClient;
use e2e::imap_client::ImapTestClient;
use e2e::websocket_client::WebSocketTestClient;
use std::time::Instant;

#[tokio::test]
async fn test_e2e_2_receive_email_external() {
    let start = Instant::now();
    let test_name = "E2E Test 2: Receive Email External".to_string();

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

    // Step 4: Get initial email count
    println!("\n📋 Step 4: Getting initial email count...");
    let initial_count = match env.get_email_count().await {
        Ok(count) => count,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("Failed to get email count");
        }
    };
    println!("📬 Initial email count: {}", initial_count);

    // Step 5: Send email via SMTP
    println!("\n📋 Step 5: Sending email via SMTP...");
    let test_id = generate_test_id();
    let from = "external@example.com";
    let to = &env.test_user_email;
    let subject = format!("External Test Email {}", test_id);
    let body = "This email was sent from an external SMTP client";

    let mut smtp_client = match SmtpTestClient::connect(&env.mail_rs_url).await {
        Ok(client) => client,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("SMTP connection failed");
        }
    };

    if let Err(e) = smtp_client.send_email(from, to, &subject, body).await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("Email send failed");
    }

    if let Err(e) = smtp_client.quit().await {
        println!("⚠️  Warning: SMTP quit failed: {}", e);
    }

    println!("✅ Email sent via SMTP");

    // Step 6: Wait for email to arrive
    println!("\n📋 Step 6: Waiting for email to arrive...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Step 7: Verify email count increased
    println!("\n📋 Step 7: Verifying email arrived...");
    let final_count = match env.get_email_count().await {
        Ok(count) => count,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("Failed to get email count");
        }
    };

    println!("📬 Final email count: {}", final_count);

    if final_count <= initial_count {
        let result = TestResult::failure(
            test_name,
            format!(
                "Email count did not increase (before: {}, after: {})",
                initial_count, final_count
            ),
            start.elapsed(),
        );
        result.print();
        panic!("Email verification failed");
    }

    println!("✅ Email received (count increased from {} to {})", initial_count, final_count);

    // Step 8: Verify via IMAP
    println!("\n📋 Step 8: Verifying email via IMAP...");
    let imap_addr = "localhost:1993";
    let mut imap_client = match ImapTestClient::connect(imap_addr).await {
        Ok(client) => client,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("IMAP connection failed");
        }
    };

    if let Err(e) = imap_client.login(&env.test_user_email, &env.test_user_password).await {
        let result = TestResult::failure(test_name, e, start.elapsed());
        result.print();
        panic!("IMAP login failed");
    }

    let mailbox_info = match imap_client.select("INBOX").await {
        Ok(info) => info,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("IMAP SELECT failed");
        }
    };

    println!("📬 IMAP reports {} emails", mailbox_info.exists);

    if mailbox_info.exists == 0 {
        let result = TestResult::failure(
            test_name,
            "IMAP shows 0 emails".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Email verification via IMAP failed");
    }

    // Search for the specific email
    let search_results = match imap_client.search(&format!("SUBJECT \"{}\"", subject)).await {
        Ok(results) => results,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("IMAP SEARCH failed");
        }
    };

    if search_results.is_empty() {
        let result = TestResult::failure(
            test_name,
            format!("Email with subject '{}' not found", subject),
            start.elapsed(),
        );
        result.print();
        panic!("Email search failed");
    }

    println!("✅ Email found via IMAP SEARCH: {:?}", search_results);

    // Step 9: Verify via AI assistant
    println!("\n📋 Step 9: Verifying via AI assistant...");
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

    let response = match ws_client.chat("Liste mes emails").await {
        Ok(resp) => resp,
        Err(e) => {
            let result = TestResult::failure(test_name, e, start.elapsed());
            result.print();
            panic!("List emails failed");
        }
    };

    println!("📨 AI response: {}", response.final_content());

    if !response.has_tool_call("list_emails") {
        let result = TestResult::failure(
            test_name,
            "list_emails tool was not called".to_string(),
            start.elapsed(),
        );
        result.print();
        panic!("Tool call verification failed");
    }

    println!("✅ AI assistant can list emails");

    if let Err(e) = ws_client.close().await {
        println!("⚠️  Warning: Failed to close WebSocket: {}", e);
    }

    if let Err(e) = imap_client.logout().await {
        println!("⚠️  Warning: Failed to logout from IMAP: {}", e);
    }

    // Test passed!
    let result = TestResult::success(test_name, start.elapsed());
    result.print();
    println!("\n🎉 Test completed successfully!");
}
