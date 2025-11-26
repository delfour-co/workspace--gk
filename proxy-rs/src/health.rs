//! Health checking for backend services
//!
//! Periodically checks backend health and maintains status.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::config::RouteConfig;

/// Health status of a backend
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthStatus {
    /// Backend is healthy and accepting requests
    Healthy,
    /// Backend is unhealthy and should not receive requests
    Unhealthy,
    /// Health status is unknown (never checked)
    Unknown,
}

/// Health check result
#[derive(Debug, Clone)]
pub struct HealthCheckResult {
    /// Current status
    pub status: HealthStatus,
    /// Last check timestamp
    pub last_check: Option<Instant>,
    /// Last successful check timestamp
    pub last_success: Option<Instant>,
    /// Number of consecutive failures
    pub consecutive_failures: u32,
    /// Response time of last successful check (milliseconds)
    pub response_time_ms: Option<u64>,
}

impl Default for HealthCheckResult {
    fn default() -> Self {
        Self {
            status: HealthStatus::Unknown,
            last_check: None,
            last_success: None,
            consecutive_failures: 0,
            response_time_ms: None,
        }
    }
}

/// Health checker for backend services
pub struct HealthChecker {
    /// Health status for each backend (keyed by backend URL)
    status: Arc<RwLock<HashMap<String, HealthCheckResult>>>,
    /// HTTP client for health checks
    client: reqwest::Client,
    /// Check interval
    interval: Duration,
    /// Timeout for health checks
    timeout: Duration,
    /// Number of failures before marking unhealthy
    failure_threshold: u32,
    /// Number of successes before marking healthy again
    success_threshold: u32,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
            client,
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(5),
            failure_threshold: 3,
            success_threshold: 2,
        }
    }

    /// Create with custom configuration
    pub fn with_config(
        interval_secs: u64,
        timeout_secs: u64,
        failure_threshold: u32,
        success_threshold: u32,
    ) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
            client,
            interval: Duration::from_secs(interval_secs),
            timeout: Duration::from_secs(timeout_secs),
            failure_threshold,
            success_threshold,
        }
    }

    /// Get health status for a backend
    pub async fn get_status(&self, backend: &str) -> HealthStatus {
        let status = self.status.read().await;
        status
            .get(backend)
            .map(|r| r.status)
            .unwrap_or(HealthStatus::Unknown)
    }

    /// Get detailed health check result for a backend
    pub async fn get_result(&self, backend: &str) -> Option<HealthCheckResult> {
        let status = self.status.read().await;
        status.get(backend).cloned()
    }

    /// Check if a backend is healthy (or unknown)
    pub async fn is_available(&self, backend: &str) -> bool {
        let status = self.get_status(backend).await;
        matches!(status, HealthStatus::Healthy | HealthStatus::Unknown)
    }

    /// Perform a single health check for a backend
    pub async fn check_backend(&self, backend: &str, health_path: Option<&str>) {
        let url = if let Some(path) = health_path {
            format!("{}{}", backend.trim_end_matches('/'), path)
        } else {
            format!("{}/health", backend.trim_end_matches('/'))
        };

        debug!("Health check: {}", url);
        let start = Instant::now();

        let result = self.client.get(&url).send().await;
        let elapsed = start.elapsed();

        let mut status = self.status.write().await;
        let entry = status.entry(backend.to_string()).or_default();
        entry.last_check = Some(Instant::now());

        match result {
            Ok(response) if response.status().is_success() => {
                entry.consecutive_failures = 0;
                entry.last_success = Some(Instant::now());
                entry.response_time_ms = Some(elapsed.as_millis() as u64);

                if entry.status != HealthStatus::Healthy {
                    info!("Backend {} is now healthy ({}ms)", backend, elapsed.as_millis());
                }
                entry.status = HealthStatus::Healthy;
            }
            Ok(response) => {
                entry.consecutive_failures += 1;
                warn!(
                    "Backend {} health check failed: HTTP {} (failures: {})",
                    backend,
                    response.status(),
                    entry.consecutive_failures
                );

                if entry.consecutive_failures >= self.failure_threshold {
                    if entry.status != HealthStatus::Unhealthy {
                        error!("Backend {} is now unhealthy", backend);
                    }
                    entry.status = HealthStatus::Unhealthy;
                }
            }
            Err(e) => {
                entry.consecutive_failures += 1;
                warn!(
                    "Backend {} health check error: {} (failures: {})",
                    backend, e, entry.consecutive_failures
                );

                if entry.consecutive_failures >= self.failure_threshold {
                    if entry.status != HealthStatus::Unhealthy {
                        error!("Backend {} is now unhealthy", backend);
                    }
                    entry.status = HealthStatus::Unhealthy;
                }
            }
        }
    }

    /// Start background health checking for routes
    pub fn start_background_checks(self: Arc<Self>, routes: Vec<RouteConfig>) {
        let checker = self.clone();
        let interval = self.interval;

        tokio::spawn(async move {
            info!("Starting health check background task");

            loop {
                for route in &routes {
                    if let Some(health_path) = &route.health_check {
                        checker.check_backend(&route.backend, Some(health_path)).await;
                    }
                }

                tokio::time::sleep(interval).await;
            }
        });
    }

    /// Get all backend statuses
    pub async fn all_statuses(&self) -> HashMap<String, HealthCheckResult> {
        self.status.read().await.clone()
    }
}

impl Default for HealthChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_checker_default() {
        let checker = HealthChecker::new();
        let status = checker.get_status("http://localhost:8080").await;
        assert_eq!(status, HealthStatus::Unknown);
    }

    #[tokio::test]
    async fn test_is_available_unknown() {
        let checker = HealthChecker::new();
        // Unknown backends should be considered available (optimistic)
        assert!(checker.is_available("http://localhost:8080").await);
    }

    #[test]
    fn test_health_check_result_default() {
        let result = HealthCheckResult::default();
        assert_eq!(result.status, HealthStatus::Unknown);
        assert_eq!(result.consecutive_failures, 0);
        assert!(result.last_check.is_none());
    }
}
