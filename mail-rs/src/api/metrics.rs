//! Prometheus metrics for API monitoring

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

/// Simple metrics collector
pub struct Metrics {
    /// Total HTTP requests
    pub http_requests_total: AtomicU64,
    /// Total HTTP errors (4xx, 5xx)
    pub http_errors_total: AtomicU64,
    /// Active connections
    pub active_connections: AtomicU64,
    /// Total emails sent
    pub emails_sent_total: AtomicU64,
    /// Total emails received
    pub emails_received_total: AtomicU64,
    /// Total authentication attempts
    pub auth_attempts_total: AtomicU64,
    /// Failed authentication attempts
    pub auth_failures_total: AtomicU64,
    /// Server start time
    start_time: Instant,
}

impl Metrics {
    /// Create a new metrics collector
    pub fn new() -> Self {
        Self {
            http_requests_total: AtomicU64::new(0),
            http_errors_total: AtomicU64::new(0),
            active_connections: AtomicU64::new(0),
            emails_sent_total: AtomicU64::new(0),
            emails_received_total: AtomicU64::new(0),
            auth_attempts_total: AtomicU64::new(0),
            auth_failures_total: AtomicU64::new(0),
            start_time: Instant::now(),
        }
    }

    /// Increment HTTP requests counter
    pub fn inc_requests(&self) {
        self.http_requests_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment HTTP errors counter
    pub fn inc_errors(&self) {
        self.http_errors_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment active connections
    pub fn inc_connections(&self) {
        self.active_connections.fetch_add(1, Ordering::Relaxed);
    }

    /// Decrement active connections
    pub fn dec_connections(&self) {
        self.active_connections.fetch_sub(1, Ordering::Relaxed);
    }

    /// Increment emails sent
    pub fn inc_emails_sent(&self) {
        self.emails_sent_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment emails received
    pub fn inc_emails_received(&self) {
        self.emails_received_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment auth attempts
    pub fn inc_auth_attempts(&self) {
        self.auth_attempts_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Increment auth failures
    pub fn inc_auth_failures(&self) {
        self.auth_failures_total.fetch_add(1, Ordering::Relaxed);
    }

    /// Get uptime in seconds
    pub fn uptime_seconds(&self) -> u64 {
        self.start_time.elapsed().as_secs()
    }

    /// Format metrics in Prometheus format
    pub fn to_prometheus(&self) -> String {
        format!(
            r#"# HELP mail_rs_http_requests_total Total HTTP requests
# TYPE mail_rs_http_requests_total counter
mail_rs_http_requests_total {}

# HELP mail_rs_http_errors_total Total HTTP errors (4xx, 5xx)
# TYPE mail_rs_http_errors_total counter
mail_rs_http_errors_total {}

# HELP mail_rs_active_connections Current active connections
# TYPE mail_rs_active_connections gauge
mail_rs_active_connections {}

# HELP mail_rs_emails_sent_total Total emails sent
# TYPE mail_rs_emails_sent_total counter
mail_rs_emails_sent_total {}

# HELP mail_rs_emails_received_total Total emails received
# TYPE mail_rs_emails_received_total counter
mail_rs_emails_received_total {}

# HELP mail_rs_auth_attempts_total Total authentication attempts
# TYPE mail_rs_auth_attempts_total counter
mail_rs_auth_attempts_total {}

# HELP mail_rs_auth_failures_total Failed authentication attempts
# TYPE mail_rs_auth_failures_total counter
mail_rs_auth_failures_total {}

# HELP mail_rs_uptime_seconds Server uptime in seconds
# TYPE mail_rs_uptime_seconds gauge
mail_rs_uptime_seconds {}
"#,
            self.http_requests_total.load(Ordering::Relaxed),
            self.http_errors_total.load(Ordering::Relaxed),
            self.active_connections.load(Ordering::Relaxed),
            self.emails_sent_total.load(Ordering::Relaxed),
            self.emails_received_total.load(Ordering::Relaxed),
            self.auth_attempts_total.load(Ordering::Relaxed),
            self.auth_failures_total.load(Ordering::Relaxed),
            self.uptime_seconds(),
        )
    }
}

impl Default for Metrics {
    fn default() -> Self {
        Self::new()
    }
}
