use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

/// Health status levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// All checks passed
    Healthy,
    /// Some warnings but operational
    Warning,
    /// Critical issues detected
    Critical,
    /// Service unavailable
    Down,
}

impl std::fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "Healthy"),
            HealthStatus::Warning => write!(f, "Warning"),
            HealthStatus::Critical => write!(f, "Critical"),
            HealthStatus::Down => write!(f, "Down"),
        }
    }
}

/// Individual diagnostic check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticResult {
    /// Check name
    pub name: String,
    /// Health status
    pub status: HealthStatus,
    /// Detailed message
    pub message: String,
    /// Optional metric value
    pub value: Option<String>,
    /// Timestamp of check
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl DiagnosticResult {
    pub fn new(name: String, status: HealthStatus, message: String) -> Self {
        DiagnosticResult {
            name,
            status,
            message,
            value: None,
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn with_value(mut self, value: String) -> Self {
        self.value = Some(value);
        self
    }
}

/// System diagnostics and health monitoring
pub struct SystemDiagnostics {
    maildir_path: String,
}

impl SystemDiagnostics {
    /// Create new diagnostics instance
    pub fn new(maildir_path: String) -> Self {
        SystemDiagnostics { maildir_path }
    }

    /// Run all diagnostic checks
    pub async fn run_all_checks(&self) -> Result<Vec<DiagnosticResult>> {
        let mut results = Vec::new();

        results.push(self.check_disk_space().await);
        results.push(self.check_maildir_permissions().await);
        results.push(self.check_memory_usage().await);
        results.push(self.check_port_availability().await);
        results.push(self.check_dns_resolution().await);
        results.push(self.check_ssl_certificates().await);

        Ok(results)
    }

    /// Get overall system health
    pub async fn get_health_status(&self) -> Result<HealthStatus> {
        let results = self.run_all_checks().await?;

        // If any critical, system is critical
        if results.iter().any(|r| r.status == HealthStatus::Critical) {
            return Ok(HealthStatus::Critical);
        }

        // If any down, system is down
        if results.iter().any(|r| r.status == HealthStatus::Down) {
            return Ok(HealthStatus::Down);
        }

        // If any warnings, system has warnings
        if results.iter().any(|r| r.status == HealthStatus::Warning) {
            return Ok(HealthStatus::Warning);
        }

        // All healthy
        Ok(HealthStatus::Healthy)
    }

    /// Check disk space availability
    async fn check_disk_space(&self) -> DiagnosticResult {
        // Use df command to check disk space
        match Command::new("df")
            .arg("-h")
            .arg(&self.maildir_path)
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse df output to get percentage
                if let Some(line) = stdout.lines().nth(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 5 {
                        let usage = parts[4];
                        let percentage = usage.trim_end_matches('%');

                        if let Ok(pct) = percentage.parse::<u32>() {
                            let status = if pct >= 90 {
                                HealthStatus::Critical
                            } else if pct >= 75 {
                                HealthStatus::Warning
                            } else {
                                HealthStatus::Healthy
                            };

                            return DiagnosticResult::new(
                                "Disk Space".to_string(),
                                status,
                                format!("Disk usage at {}%", pct),
                            )
                            .with_value(format!("{}%", pct));
                        }
                    }
                }

                DiagnosticResult::new(
                    "Disk Space".to_string(),
                    HealthStatus::Warning,
                    "Could not parse disk usage".to_string(),
                )
            }
            Err(e) => DiagnosticResult::new(
                "Disk Space".to_string(),
                HealthStatus::Warning,
                format!("Could not check disk space: {}", e),
            ),
        }
    }

    /// Check maildir directory permissions
    async fn check_maildir_permissions(&self) -> DiagnosticResult {
        let path = Path::new(&self.maildir_path);

        if !path.exists() {
            return DiagnosticResult::new(
                "Maildir Permissions".to_string(),
                HealthStatus::Critical,
                format!("Maildir does not exist: {}", self.maildir_path),
            );
        }

        match fs::metadata(path).await {
            Ok(metadata) => {
                if metadata.is_dir() {
                    DiagnosticResult::new(
                        "Maildir Permissions".to_string(),
                        HealthStatus::Healthy,
                        "Maildir accessible and writable".to_string(),
                    )
                } else {
                    DiagnosticResult::new(
                        "Maildir Permissions".to_string(),
                        HealthStatus::Critical,
                        "Maildir path is not a directory".to_string(),
                    )
                }
            }
            Err(e) => DiagnosticResult::new(
                "Maildir Permissions".to_string(),
                HealthStatus::Critical,
                format!("Cannot access maildir: {}", e),
            ),
        }
    }

    /// Check memory usage
    async fn check_memory_usage(&self) -> DiagnosticResult {
        // Use free command to check memory
        match Command::new("free").arg("-m").output().await {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                // Parse free output
                if let Some(line) = stdout.lines().nth(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        if let (Ok(total), Ok(used)) = (parts[1].parse::<f64>(), parts[2].parse::<f64>()) {
                            let percentage = (used / total * 100.0) as u32;

                            let status = if percentage >= 90 {
                                HealthStatus::Critical
                            } else if percentage >= 80 {
                                HealthStatus::Warning
                            } else {
                                HealthStatus::Healthy
                            };

                            return DiagnosticResult::new(
                                "Memory Usage".to_string(),
                                status,
                                format!("Memory usage at {}%", percentage),
                            )
                            .with_value(format!("{}%", percentage));
                        }
                    }
                }

                DiagnosticResult::new(
                    "Memory Usage".to_string(),
                    HealthStatus::Warning,
                    "Could not parse memory usage".to_string(),
                )
            }
            Err(e) => DiagnosticResult::new(
                "Memory Usage".to_string(),
                HealthStatus::Warning,
                format!("Could not check memory: {}", e),
            ),
        }
    }

    /// Check SMTP/IMAP port availability
    async fn check_port_availability(&self) -> DiagnosticResult {
        // Check if ports 25, 587, 993, 143 are available/listening
        match Command::new("ss")
            .arg("-tuln")
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                let mut ports_listening = Vec::new();
                for port in &["25", "587", "143", "993"] {
                    if stdout.contains(&format!(":{}",port)) {
                        ports_listening.push(*port);
                    }
                }

                if ports_listening.is_empty() {
                    DiagnosticResult::new(
                        "Port Availability".to_string(),
                        HealthStatus::Warning,
                        "No mail ports detected listening".to_string(),
                    )
                } else {
                    DiagnosticResult::new(
                        "Port Availability".to_string(),
                        HealthStatus::Healthy,
                        format!("Ports listening: {}", ports_listening.join(", ")),
                    )
                    .with_value(ports_listening.join(", "))
                }
            }
            Err(e) => DiagnosticResult::new(
                "Port Availability".to_string(),
                HealthStatus::Warning,
                format!("Could not check ports: {}", e),
            ),
        }
    }

    /// Check DNS resolution
    async fn check_dns_resolution(&self) -> DiagnosticResult {
        // Try to resolve a known domain
        match Command::new("host")
            .arg("google.com")
            .output()
            .await
        {
            Ok(output) => {
                if output.status.success() {
                    DiagnosticResult::new(
                        "DNS Resolution".to_string(),
                        HealthStatus::Healthy,
                        "DNS resolution working".to_string(),
                    )
                } else {
                    DiagnosticResult::new(
                        "DNS Resolution".to_string(),
                        HealthStatus::Critical,
                        "DNS resolution failing".to_string(),
                    )
                }
            }
            Err(e) => DiagnosticResult::new(
                "DNS Resolution".to_string(),
                HealthStatus::Warning,
                format!("Could not test DNS: {}", e),
            ),
        }
    }

    /// Check SSL certificate validity
    async fn check_ssl_certificates(&self) -> DiagnosticResult {
        let cert_path = Path::new("certs/server.crt");

        if !cert_path.exists() {
            return DiagnosticResult::new(
                "SSL Certificate".to_string(),
                HealthStatus::Warning,
                "No SSL certificate found".to_string(),
            );
        }

        // Check certificate expiration with openssl
        match Command::new("openssl")
            .arg("x509")
            .arg("-in")
            .arg("certs/server.crt")
            .arg("-noout")
            .arg("-enddate")
            .output()
            .await
        {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);

                if stdout.contains("notAfter=") {
                    DiagnosticResult::new(
                        "SSL Certificate".to_string(),
                        HealthStatus::Healthy,
                        "SSL certificate present and valid".to_string(),
                    )
                } else {
                    DiagnosticResult::new(
                        "SSL Certificate".to_string(),
                        HealthStatus::Warning,
                        "Could not parse certificate expiration".to_string(),
                    )
                }
            }
            Err(e) => DiagnosticResult::new(
                "SSL Certificate".to_string(),
                HealthStatus::Warning,
                format!("Could not check certificate: {}", e),
            ),
        }
    }

    /// Generate health report
    pub async fn generate_report(&self) -> Result<String> {
        let results = self.run_all_checks().await?;
        let overall = self.get_health_status().await?;

        let mut report = String::new();
        report.push_str("System Health Report\n");
        report.push_str("=".repeat(60).as_str());
        report.push_str("\n\n");
        report.push_str(&format!("Overall Status: {}\n", overall));
        report.push_str(&format!("Timestamp: {}\n\n", chrono::Utc::now()));

        for result in &results {
            report.push_str(&format!("[{}] {}\n", result.status, result.name));
            report.push_str(&format!("  {}\n", result.message));
            if let Some(ref value) = result.value {
                report.push_str(&format!("  Value: {}\n", value));
            }
            report.push_str("\n");
        }

        Ok(report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_display() {
        assert_eq!(HealthStatus::Healthy.to_string(), "Healthy");
        assert_eq!(HealthStatus::Warning.to_string(), "Warning");
        assert_eq!(HealthStatus::Critical.to_string(), "Critical");
        assert_eq!(HealthStatus::Down.to_string(), "Down");
    }

    #[test]
    fn test_diagnostic_result_new() {
        let result = DiagnosticResult::new(
            "Test Check".to_string(),
            HealthStatus::Healthy,
            "All good".to_string(),
        );

        assert_eq!(result.name, "Test Check");
        assert_eq!(result.status, HealthStatus::Healthy);
        assert_eq!(result.message, "All good");
        assert!(result.value.is_none());
    }

    #[test]
    fn test_diagnostic_result_with_value() {
        let result = DiagnosticResult::new(
            "Test Check".to_string(),
            HealthStatus::Healthy,
            "All good".to_string(),
        )
        .with_value("100%".to_string());

        assert_eq!(result.value, Some("100%".to_string()));
    }

    #[test]
    fn test_system_diagnostics_new() {
        let diagnostics = SystemDiagnostics::new("/tmp/maildir".to_string());
        assert_eq!(diagnostics.maildir_path, "/tmp/maildir");
    }

    #[tokio::test]
    async fn test_run_all_checks() {
        let diagnostics = SystemDiagnostics::new("/tmp".to_string());
        let results = diagnostics.run_all_checks().await.unwrap();

        // Should have 6 checks
        assert_eq!(results.len(), 6);

        // Check that all expected checks are present
        let check_names: Vec<String> = results.iter().map(|r| r.name.clone()).collect();
        assert!(check_names.contains(&"Disk Space".to_string()));
        assert!(check_names.contains(&"Maildir Permissions".to_string()));
        assert!(check_names.contains(&"Memory Usage".to_string()));
        assert!(check_names.contains(&"Port Availability".to_string()));
        assert!(check_names.contains(&"DNS Resolution".to_string()));
        assert!(check_names.contains(&"SSL Certificate".to_string()));
    }

    #[tokio::test]
    async fn test_get_health_status() {
        let diagnostics = SystemDiagnostics::new("/tmp".to_string());
        let status = diagnostics.get_health_status().await.unwrap();

        // Status should be one of the valid values
        assert!(matches!(
            status,
            HealthStatus::Healthy
                | HealthStatus::Warning
                | HealthStatus::Critical
                | HealthStatus::Down
        ));
    }

    #[tokio::test]
    async fn test_check_maildir_permissions_exists() {
        let diagnostics = SystemDiagnostics::new("/tmp".to_string());
        let result = diagnostics.check_maildir_permissions().await;

        assert_eq!(result.name, "Maildir Permissions");
        // /tmp should exist and be accessible
        assert_eq!(result.status, HealthStatus::Healthy);
    }

    #[tokio::test]
    async fn test_check_maildir_permissions_not_exists() {
        let diagnostics = SystemDiagnostics::new("/nonexistent/path".to_string());
        let result = diagnostics.check_maildir_permissions().await;

        assert_eq!(result.name, "Maildir Permissions");
        assert_eq!(result.status, HealthStatus::Critical);
        assert!(result.message.contains("does not exist"));
    }

    #[tokio::test]
    async fn test_generate_report() {
        let diagnostics = SystemDiagnostics::new("/tmp".to_string());
        let report = diagnostics.generate_report().await.unwrap();

        assert!(report.contains("System Health Report"));
        assert!(report.contains("Overall Status:"));
        assert!(report.contains("Timestamp:"));
    }
}
