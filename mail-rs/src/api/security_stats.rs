//! API endpoints for security statistics (SPF/DKIM/DMARC)

use crate::api::auth::get_session_email;
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Security event type
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityEventType {
    SpfPass,
    SpfFail,
    SpfSoftFail,
    DkimPass,
    DkimFail,
    DmarcPass,
    DmarcFail,
    DmarcQuarantine,
    DmarcReject,
}

impl std::fmt::Display for SecurityEventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecurityEventType::SpfPass => write!(f, "SPF Pass"),
            SecurityEventType::SpfFail => write!(f, "SPF Fail"),
            SecurityEventType::SpfSoftFail => write!(f, "SPF SoftFail"),
            SecurityEventType::DkimPass => write!(f, "DKIM Pass"),
            SecurityEventType::DkimFail => write!(f, "DKIM Fail"),
            SecurityEventType::DmarcPass => write!(f, "DMARC Pass"),
            SecurityEventType::DmarcFail => write!(f, "DMARC Fail"),
            SecurityEventType::DmarcQuarantine => write!(f, "DMARC Quarantine"),
            SecurityEventType::DmarcReject => write!(f, "DMARC Reject"),
        }
    }
}

/// Security event record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    pub event_type: SecurityEventType,
    pub timestamp: DateTime<Utc>,
    pub sender: String,
    pub recipient: String,
    pub client_ip: String,
    pub details: Option<String>,
}

/// Security statistics manager
pub struct SecurityStatsManager {
    events: Arc<RwLock<VecDeque<SecurityEvent>>>,
    max_events: usize,
    // SPF counters
    spf_pass: Arc<RwLock<u64>>,
    spf_fail: Arc<RwLock<u64>>,
    spf_softfail: Arc<RwLock<u64>>,
    // DKIM counters
    dkim_pass: Arc<RwLock<u64>>,
    dkim_fail: Arc<RwLock<u64>>,
    // DMARC counters
    dmarc_pass: Arc<RwLock<u64>>,
    dmarc_fail: Arc<RwLock<u64>>,
    dmarc_quarantine: Arc<RwLock<u64>>,
    dmarc_reject: Arc<RwLock<u64>>,
    // Configuration status
    spf_enabled: Arc<RwLock<bool>>,
    dkim_enabled: Arc<RwLock<bool>>,
    dmarc_enabled: Arc<RwLock<bool>>,
}

impl SecurityStatsManager {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(VecDeque::with_capacity(1000))),
            max_events: 1000,
            spf_pass: Arc::new(RwLock::new(0)),
            spf_fail: Arc::new(RwLock::new(0)),
            spf_softfail: Arc::new(RwLock::new(0)),
            dkim_pass: Arc::new(RwLock::new(0)),
            dkim_fail: Arc::new(RwLock::new(0)),
            dmarc_pass: Arc::new(RwLock::new(0)),
            dmarc_fail: Arc::new(RwLock::new(0)),
            dmarc_quarantine: Arc::new(RwLock::new(0)),
            dmarc_reject: Arc::new(RwLock::new(0)),
            spf_enabled: Arc::new(RwLock::new(true)),
            dkim_enabled: Arc::new(RwLock::new(true)),
            dmarc_enabled: Arc::new(RwLock::new(true)),
        }
    }

    /// Record a security event
    pub async fn record_event(&self, event: SecurityEvent) {
        // Update counters
        match event.event_type {
            SecurityEventType::SpfPass => *self.spf_pass.write().await += 1,
            SecurityEventType::SpfFail => *self.spf_fail.write().await += 1,
            SecurityEventType::SpfSoftFail => *self.spf_softfail.write().await += 1,
            SecurityEventType::DkimPass => *self.dkim_pass.write().await += 1,
            SecurityEventType::DkimFail => *self.dkim_fail.write().await += 1,
            SecurityEventType::DmarcPass => *self.dmarc_pass.write().await += 1,
            SecurityEventType::DmarcFail => *self.dmarc_fail.write().await += 1,
            SecurityEventType::DmarcQuarantine => *self.dmarc_quarantine.write().await += 1,
            SecurityEventType::DmarcReject => *self.dmarc_reject.write().await += 1,
        }

        // Add to event log
        let mut events = self.events.write().await;
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Get recent events (last N)
    pub async fn get_recent_events(&self, limit: usize) -> Vec<SecurityEvent> {
        let events = self.events.read().await;
        events.iter().rev().take(limit).cloned().collect()
    }

    /// Get all statistics
    pub async fn get_stats(&self) -> SecurityStats {
        SecurityStats {
            spf: SpfStats {
                passed: *self.spf_pass.read().await,
                failed: *self.spf_fail.read().await,
                softfail: *self.spf_softfail.read().await,
                enabled: *self.spf_enabled.read().await,
            },
            dkim: DkimStats {
                passed: *self.dkim_pass.read().await,
                failed: *self.dkim_fail.read().await,
                enabled: *self.dkim_enabled.read().await,
            },
            dmarc: DmarcStats {
                passed: *self.dmarc_pass.read().await,
                failed: *self.dmarc_fail.read().await,
                quarantine: *self.dmarc_quarantine.read().await,
                reject: *self.dmarc_reject.read().await,
                enabled: *self.dmarc_enabled.read().await,
            },
        }
    }

    /// Set SPF enabled status
    pub async fn set_spf_enabled(&self, enabled: bool) {
        *self.spf_enabled.write().await = enabled;
    }

    /// Set DKIM enabled status
    pub async fn set_dkim_enabled(&self, enabled: bool) {
        *self.dkim_enabled.write().await = enabled;
    }

    /// Set DMARC enabled status
    pub async fn set_dmarc_enabled(&self, enabled: bool) {
        *self.dmarc_enabled.write().await = enabled;
    }
}

impl Default for SecurityStatsManager {
    fn default() -> Self {
        Self::new()
    }
}

/// App state containing security stats manager
pub struct SecurityStatsState {
    pub manager: Arc<SecurityStatsManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// SPF statistics
#[derive(Serialize)]
pub struct SpfStats {
    pub passed: u64,
    pub failed: u64,
    pub softfail: u64,
    pub enabled: bool,
}

/// DKIM statistics
#[derive(Serialize)]
pub struct DkimStats {
    pub passed: u64,
    pub failed: u64,
    pub enabled: bool,
}

/// DMARC statistics
#[derive(Serialize)]
pub struct DmarcStats {
    pub passed: u64,
    pub failed: u64,
    pub quarantine: u64,
    pub reject: u64,
    pub enabled: bool,
}

/// Combined security statistics
#[derive(Serialize)]
pub struct SecurityStats {
    pub spf: SpfStats,
    pub dkim: DkimStats,
    pub dmarc: DmarcStats,
}

/// Security events response
#[derive(Serialize)]
pub struct SecurityEventsResponse {
    pub events: Vec<SecurityEvent>,
    pub total: usize,
}

/// GET /api/admin/security/stats - Get security statistics
pub async fn get_stats(
    State(state): State<Arc<SecurityStatsState>>,
    headers: HeaderMap,
) -> Result<Json<SecurityStats>, (StatusCode, Json<ApiError>)> {
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

/// GET /api/admin/security/events - Get recent security events
pub async fn get_events(
    State(state): State<Arc<SecurityStatsState>>,
    headers: HeaderMap,
) -> Result<Json<SecurityEventsResponse>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let events = state.manager.get_recent_events(100).await;
    let total = events.len();

    Ok(Json(SecurityEventsResponse { events, total }))
}

/// Configuration update request
#[derive(Deserialize)]
pub struct UpdateConfigRequest {
    pub spf_enabled: Option<bool>,
    pub dkim_enabled: Option<bool>,
    pub dmarc_enabled: Option<bool>,
}

/// PUT /api/admin/security/config - Update security configuration
pub async fn update_config(
    State(state): State<Arc<SecurityStatsState>>,
    headers: HeaderMap,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    if let Some(enabled) = payload.spf_enabled {
        state.manager.set_spf_enabled(enabled).await;
    }
    if let Some(enabled) = payload.dkim_enabled {
        state.manager.set_dkim_enabled(enabled).await;
    }
    if let Some(enabled) = payload.dmarc_enabled {
        state.manager.set_dmarc_enabled(enabled).await;
    }

    Ok(StatusCode::OK)
}
