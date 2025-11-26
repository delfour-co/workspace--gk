use std::process::{Child, Command};
use std::time::Duration;
use tokio::time::sleep;

/// Test environment configuration
pub struct TestEnv {
    pub mail_rs_url: String,
    pub ai_runtime_url: String,
    pub mcp_mail_url: String,
    pub web_ui_url: String,
    pub test_user_email: String,
    pub test_user_password: String,
}

impl Default for TestEnv {
    fn default() -> Self {
        Self {
            mail_rs_url: "localhost:2525".to_string(),
            ai_runtime_url: "http://localhost:8888".to_string(),
            mcp_mail_url: "http://localhost:8090".to_string(),
            web_ui_url: "http://localhost:5173".to_string(),
            test_user_email: "test@localhost".to_string(),
            test_user_password: "testpass123".to_string(),
        }
    }
}

impl TestEnv {
    /// Create a new test environment
    pub fn new() -> Self {
        Self::default()
    }

    /// Wait for service to be ready
    pub async fn wait_for_service(&self, url: &str, timeout_secs: u64) -> Result<(), String> {
        let client = reqwest::Client::new();
        let start = std::time::Instant::now();

        loop {
            if start.elapsed().as_secs() > timeout_secs {
                return Err(format!("Timeout waiting for service: {}", url));
            }

            match client.get(url).send().await {
                Ok(_) => return Ok(()),
                Err(_) => {
                    sleep(Duration::from_millis(500)).await;
                }
            }
        }
    }

    /// Check if Docker services are running
    pub async fn check_services(&self) -> Result<(), String> {
        println!("🔍 Checking if services are running...");

        // Check ai-runtime health
        let health_url = format!("{}/health", self.ai_runtime_url);
        self.wait_for_service(&health_url, 30).await
            .map_err(|e| format!("ai-runtime not ready: {}", e))?;
        println!("✅ ai-runtime is ready");

        // Check mcp-mail health
        let mcp_health_url = format!("{}/health", self.mcp_mail_url);
        self.wait_for_service(&mcp_health_url, 30).await
            .map_err(|e| format!("mcp-mail-server not ready: {}", e))?;
        println!("✅ mcp-mail-server is ready");

        Ok(())
    }

    /// Create a test user if it doesn't exist
    pub async fn ensure_test_user(&self) -> Result<(), String> {
        println!("👤 Ensuring test user exists: {}", self.test_user_email);

        // Ensure database file exists (sqlx doesn't create it automatically)
        let _ = Command::new("docker")
            .args(&[
                "exec",
                "gk-mail-rs-dev",
                "sh",
                "-c",
                "touch /data/db/users.db && chmod 666 /data/db/users.db",
            ])
            .output();

        // Check if user exists via docker exec
        let check_output = Command::new("docker")
            .args(&[
                "exec",
                "gk-mail-rs-dev",
                "/app/mail-user",
                "-d",
                "sqlite:///data/db/users.db",
                "exists",
                &self.test_user_email,
            ])
            .output()
            .map_err(|e| format!("Failed to check user: {}", e))?;

        if check_output.status.success() {
            println!("✅ Test user already exists");
            return Ok(());
        }

        // Create user
        let create_output = Command::new("docker")
            .args(&[
                "exec",
                "gk-mail-rs-dev",
                "/app/mail-user",
                "-d",
                "sqlite:///data/db/users.db",
                "add",
                &self.test_user_email,
                &self.test_user_password,
            ])
            .output()
            .map_err(|e| format!("Failed to create user: {}", e))?;

        if !create_output.status.success() {
            return Err(format!(
                "Failed to create test user: {}",
                String::from_utf8_lossy(&create_output.stderr)
            ));
        }

        println!("✅ Test user created");
        Ok(())
    }

    /// Clean test user's mailbox
    pub async fn clean_mailbox(&self) -> Result<(), String> {
        println!("🧹 Cleaning test user mailbox...");

        // Use /tmp/maildir as configured in the container
        let maildir_path = format!("/tmp/maildir/{}", self.test_user_email);

        // Remove all emails from new/ and cur/
        let _ = Command::new("docker")
            .args(&[
                "exec",
                "gk-mail-rs-dev",
                "sh",
                "-c",
                &format!("rm -f {}/new/* {}/cur/* 2>/dev/null || true", maildir_path, maildir_path),
            ])
            .output();

        println!("✅ Mailbox cleaned");
        Ok(())
    }

    /// Get email count in mailbox
    pub async fn get_email_count(&self) -> Result<usize, String> {
        // Use /tmp/maildir as configured in the container
        let maildir_path = format!("/tmp/maildir/{}", self.test_user_email);

        // Count files in new/ and cur/
        let output = Command::new("docker")
            .args(&[
                "exec",
                "gk-mail-rs-dev",
                "sh",
                "-c",
                &format!("ls -1 {}/new {}/cur 2>/dev/null | wc -l", maildir_path, maildir_path),
            ])
            .output()
            .map_err(|e| format!("Failed to count emails: {}", e))?;

        let count_str = String::from_utf8_lossy(&output.stdout);
        let count = count_str
            .trim()
            .parse::<usize>()
            .map_err(|e| format!("Failed to parse count: {}", e))?;

        Ok(count)
    }
}

/// Test result helper
#[derive(Debug)]
pub struct TestResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub duration: Duration,
}

impl TestResult {
    pub fn success(name: String, duration: Duration) -> Self {
        Self {
            name,
            passed: true,
            message: "✅ Test passed".to_string(),
            duration,
        }
    }

    pub fn failure(name: String, message: String, duration: Duration) -> Self {
        Self {
            name,
            passed: false,
            message: format!("❌ Test failed: {}", message),
            duration,
        }
    }

    pub fn print(&self) {
        println!("\n{}", "=".repeat(80));
        println!("📝 Test: {}", self.name);
        println!("⏱️  Duration: {:?}", self.duration);
        println!("{}", self.message);
        println!("{}", "=".repeat(80));
    }
}

/// Generate a unique email ID for testing
pub fn generate_test_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();
    format!("test_{}", timestamp)
}
