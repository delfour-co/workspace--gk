//! API endpoints for server monitoring (SMTP/IMAP stats)

use crate::api::auth::get_session_email;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Activity log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityLogEntry {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub service: String,
    pub message: String,
    pub client_ip: Option<String>,
    pub user: Option<String>,
}

/// Server monitoring manager
pub struct MonitoringManager {
    // SMTP stats
    smtp_running: AtomicBool,
    smtp_connections: AtomicU64,
    smtp_messages_received: AtomicU64,
    smtp_messages_sent: AtomicU64,
    smtp_queue_size: AtomicU64,
    smtp_errors: AtomicU64,
    smtp_start_time: RwLock<Option<Instant>>,

    // IMAP stats
    imap_running: AtomicBool,
    imap_connections: AtomicU64,
    imap_commands: AtomicU64,
    imap_errors: AtomicU64,
    imap_start_time: RwLock<Option<Instant>>,

    // Performance metrics
    smtp_avg_latency_ms: AtomicU64,
    imap_avg_latency_ms: AtomicU64,
    smtp_throughput: AtomicU64,
    imap_throughput: AtomicU64,

    // Activity log
    activity_log: Arc<RwLock<VecDeque<ActivityLogEntry>>>,
    max_log_entries: usize,
}

impl MonitoringManager {
    pub fn new() -> Self {
        Self {
            smtp_running: AtomicBool::new(false),
            smtp_connections: AtomicU64::new(0),
            smtp_messages_received: AtomicU64::new(0),
            smtp_messages_sent: AtomicU64::new(0),
            smtp_queue_size: AtomicU64::new(0),
            smtp_errors: AtomicU64::new(0),
            smtp_start_time: RwLock::new(None),

            imap_running: AtomicBool::new(false),
            imap_connections: AtomicU64::new(0),
            imap_commands: AtomicU64::new(0),
            imap_errors: AtomicU64::new(0),
            imap_start_time: RwLock::new(None),

            smtp_avg_latency_ms: AtomicU64::new(0),
            imap_avg_latency_ms: AtomicU64::new(0),
            smtp_throughput: AtomicU64::new(0),
            imap_throughput: AtomicU64::new(0),

            activity_log: Arc::new(RwLock::new(VecDeque::with_capacity(500))),
            max_log_entries: 500,
        }
    }

    // SMTP methods
    pub fn set_smtp_running(&self, running: bool) {
        self.smtp_running.store(running, Ordering::SeqCst);
    }

    pub fn smtp_is_running(&self) -> bool {
        self.smtp_running.load(Ordering::SeqCst)
    }

    pub async fn start_smtp(&self) {
        self.smtp_running.store(true, Ordering::SeqCst);
        *self.smtp_start_time.write().await = Some(Instant::now());
    }

    pub async fn stop_smtp(&self) {
        self.smtp_running.store(false, Ordering::SeqCst);
        *self.smtp_start_time.write().await = None;
    }

    pub fn increment_smtp_connections(&self) {
        self.smtp_connections.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_smtp_connections(&self) {
        self.smtp_connections.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn increment_smtp_messages_received(&self) {
        self.smtp_messages_received.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_smtp_messages_sent(&self) {
        self.smtp_messages_sent.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set_smtp_queue_size(&self, size: u64) {
        self.smtp_queue_size.store(size, Ordering::SeqCst);
    }

    pub fn increment_smtp_errors(&self) {
        self.smtp_errors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set_smtp_latency(&self, latency_ms: u64) {
        self.smtp_avg_latency_ms.store(latency_ms, Ordering::SeqCst);
    }

    pub fn set_smtp_throughput(&self, throughput: u64) {
        self.smtp_throughput.store(throughput, Ordering::SeqCst);
    }

    // IMAP methods
    pub fn set_imap_running(&self, running: bool) {
        self.imap_running.store(running, Ordering::SeqCst);
    }

    pub fn imap_is_running(&self) -> bool {
        self.imap_running.load(Ordering::SeqCst)
    }

    pub async fn start_imap(&self) {
        self.imap_running.store(true, Ordering::SeqCst);
        *self.imap_start_time.write().await = Some(Instant::now());
    }

    pub async fn stop_imap(&self) {
        self.imap_running.store(false, Ordering::SeqCst);
        *self.imap_start_time.write().await = None;
    }

    pub fn increment_imap_connections(&self) {
        self.imap_connections.fetch_add(1, Ordering::SeqCst);
    }

    pub fn decrement_imap_connections(&self) {
        self.imap_connections.fetch_sub(1, Ordering::SeqCst);
    }

    pub fn increment_imap_commands(&self) {
        self.imap_commands.fetch_add(1, Ordering::SeqCst);
    }

    pub fn increment_imap_errors(&self) {
        self.imap_errors.fetch_add(1, Ordering::SeqCst);
    }

    pub fn set_imap_latency(&self, latency_ms: u64) {
        self.imap_avg_latency_ms.store(latency_ms, Ordering::SeqCst);
    }

    pub fn set_imap_throughput(&self, throughput: u64) {
        self.imap_throughput.store(throughput, Ordering::SeqCst);
    }

    // Activity log methods
    pub async fn log_activity(&self, entry: ActivityLogEntry) {
        let mut log = self.activity_log.write().await;
        if log.len() >= self.max_log_entries {
            log.pop_front();
        }
        log.push_back(entry);
    }

    pub async fn get_recent_activity(&self, limit: usize) -> Vec<ActivityLogEntry> {
        let log = self.activity_log.read().await;
        log.iter().rev().take(limit).cloned().collect()
    }

    // Get all stats
    pub async fn get_stats(&self) -> MonitoringStats {
        let smtp_uptime = if let Some(start) = *self.smtp_start_time.read().await {
            Some(start.elapsed().as_secs())
        } else {
            None
        };

        let imap_uptime = if let Some(start) = *self.imap_start_time.read().await {
            Some(start.elapsed().as_secs())
        } else {
            None
        };

        MonitoringStats {
            smtp: SmtpStats {
                running: self.smtp_running.load(Ordering::SeqCst),
                active_connections: self.smtp_connections.load(Ordering::SeqCst),
                messages_received: self.smtp_messages_received.load(Ordering::SeqCst),
                messages_sent: self.smtp_messages_sent.load(Ordering::SeqCst),
                queue_size: self.smtp_queue_size.load(Ordering::SeqCst),
                errors: self.smtp_errors.load(Ordering::SeqCst),
                uptime_seconds: smtp_uptime,
                avg_latency_ms: self.smtp_avg_latency_ms.load(Ordering::SeqCst),
                throughput: self.smtp_throughput.load(Ordering::SeqCst),
            },
            imap: ImapStats {
                running: self.imap_running.load(Ordering::SeqCst),
                active_connections: self.imap_connections.load(Ordering::SeqCst),
                commands_processed: self.imap_commands.load(Ordering::SeqCst),
                errors: self.imap_errors.load(Ordering::SeqCst),
                uptime_seconds: imap_uptime,
                avg_latency_ms: self.imap_avg_latency_ms.load(Ordering::SeqCst),
                throughput: self.imap_throughput.load(Ordering::SeqCst),
            },
        }
    }
}

impl Default for MonitoringManager {
    fn default() -> Self {
        Self::new()
    }
}

/// App state containing monitoring manager
pub struct MonitoringState {
    pub manager: Arc<MonitoringManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// SMTP server statistics
#[derive(Serialize)]
pub struct SmtpStats {
    pub running: bool,
    pub active_connections: u64,
    pub messages_received: u64,
    pub messages_sent: u64,
    pub queue_size: u64,
    pub errors: u64,
    pub uptime_seconds: Option<u64>,
    pub avg_latency_ms: u64,
    pub throughput: u64,
}

/// IMAP server statistics
#[derive(Serialize)]
pub struct ImapStats {
    pub running: bool,
    pub active_connections: u64,
    pub commands_processed: u64,
    pub errors: u64,
    pub uptime_seconds: Option<u64>,
    pub avg_latency_ms: u64,
    pub throughput: u64,
}

/// Combined monitoring statistics
#[derive(Serialize)]
pub struct MonitoringStats {
    pub smtp: SmtpStats,
    pub imap: ImapStats,
}

/// Activity log response
#[derive(Serialize)]
pub struct ActivityLogResponse {
    pub entries: Vec<ActivityLogEntry>,
    pub total: usize,
}

/// GET /api/admin/monitoring/stats - Get server statistics
pub async fn get_stats(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<Json<MonitoringStats>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let stats = state.manager.get_stats().await;
    Ok(Json(stats))
}

/// GET /api/admin/monitoring/logs - Get activity logs
pub async fn get_logs(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<Json<ActivityLogResponse>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let entries = state.manager.get_recent_activity(100).await;
    let total = entries.len();

    Ok(Json(ActivityLogResponse { entries, total }))
}

/// POST /api/admin/monitoring/smtp/start - Start SMTP server (placeholder)
pub async fn start_smtp(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state.manager.start_smtp().await;

    // Log the action
    state
        .manager
        .log_activity(ActivityLogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            service: "SMTP".to_string(),
            message: "SMTP server started via admin panel".to_string(),
            client_ip: None,
            user: None,
        })
        .await;

    Ok(StatusCode::OK)
}

/// POST /api/admin/monitoring/smtp/stop - Stop SMTP server (placeholder)
pub async fn stop_smtp(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state.manager.stop_smtp().await;

    state
        .manager
        .log_activity(ActivityLogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            service: "SMTP".to_string(),
            message: "SMTP server stopped via admin panel".to_string(),
            client_ip: None,
            user: None,
        })
        .await;

    Ok(StatusCode::OK)
}

/// POST /api/admin/monitoring/imap/start - Start IMAP server (placeholder)
pub async fn start_imap(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state.manager.start_imap().await;

    state
        .manager
        .log_activity(ActivityLogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            service: "IMAP".to_string(),
            message: "IMAP server started via admin panel".to_string(),
            client_ip: None,
            user: None,
        })
        .await;

    Ok(StatusCode::OK)
}

/// POST /api/admin/monitoring/imap/stop - Stop IMAP server (placeholder)
pub async fn stop_imap(
    State(state): State<Arc<MonitoringState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state.manager.stop_imap().await;

    state
        .manager
        .log_activity(ActivityLogEntry {
            timestamp: Utc::now(),
            level: "INFO".to_string(),
            service: "IMAP".to_string(),
            message: "IMAP server stopped via admin panel".to_string(),
            client_ip: None,
            user: None,
        })
        .await;

    Ok(StatusCode::OK)
}
