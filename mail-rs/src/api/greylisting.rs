//! API endpoints for greylisting management

use crate::antispam::greylist::GreylistManager;
use crate::antispam::types::{GreylistEntry, GreylistStatus, ListEntry};
use crate::api::auth::get_session_email;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// App state containing greylist manager
pub struct GreylistState {
    pub manager: Arc<GreylistManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Stats response for greylist dashboard
#[derive(Serialize)]
pub struct GreylistStatsResponse {
    pub greylisted_count: usize,
    pub whitelisted_count: usize,
    pub blacklisted_count: usize,
    pub total_entries: usize,
}

/// Request to add to whitelist/blacklist
#[derive(Deserialize)]
pub struct AddListEntryRequest {
    pub pattern: String,
    pub reason: Option<String>,
}

/// Response for cleanup operation
#[derive(Serialize)]
pub struct CleanupResponse {
    pub removed_count: usize,
}

/// GET /api/admin/greylisting/stats - Get greylist statistics
pub async fn get_stats(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
) -> Result<Json<GreylistStatsResponse>, (StatusCode, Json<ApiError>)> {
    // Verify admin session
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let entries = state.manager.get_entries().await;
    let whitelist = state.manager.get_whitelist().await;
    let blacklist = state.manager.get_blacklist().await;

    let greylisted_count = entries
        .iter()
        .filter(|e| e.status == GreylistStatus::Greylisted)
        .count();
    let whitelisted_in_entries = entries
        .iter()
        .filter(|e| e.status == GreylistStatus::Whitelisted)
        .count();

    Ok(Json(GreylistStatsResponse {
        greylisted_count,
        whitelisted_count: whitelist.len() + whitelisted_in_entries,
        blacklisted_count: blacklist.len(),
        total_entries: entries.len(),
    }))
}

/// GET /api/admin/greylisting/entries - List all greylist entries
pub async fn list_entries(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<GreylistEntry>>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let entries = state.manager.get_entries().await;
    Ok(Json(entries))
}

/// GET /api/admin/greylisting/whitelist - List whitelist entries
pub async fn list_whitelist(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ListEntry>>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let whitelist = state.manager.get_whitelist().await;
    Ok(Json(whitelist))
}

/// POST /api/admin/greylisting/whitelist - Add to whitelist
pub async fn add_to_whitelist(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
    Json(payload): Json<AddListEntryRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    if payload.pattern.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "Pattern cannot be empty".to_string(),
            }),
        ));
    }

    state
        .manager
        .add_to_whitelist(payload.pattern, payload.reason)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::CREATED)
}

/// DELETE /api/admin/greylisting/whitelist/:pattern - Remove from whitelist
pub async fn remove_from_whitelist(
    State(state): State<Arc<GreylistState>>,
    Path(pattern): Path<String>,
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

    state
        .manager
        .remove_from_whitelist(&pattern)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/admin/greylisting/blacklist - List blacklist entries
pub async fn list_blacklist(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<ListEntry>>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let blacklist = state.manager.get_blacklist().await;
    Ok(Json(blacklist))
}

/// POST /api/admin/greylisting/blacklist - Add to blacklist
pub async fn add_to_blacklist(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
    Json(payload): Json<AddListEntryRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    if payload.pattern.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "Pattern cannot be empty".to_string(),
            }),
        ));
    }

    state
        .manager
        .add_to_blacklist(payload.pattern, payload.reason)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::CREATED)
}

/// DELETE /api/admin/greylisting/blacklist/:pattern - Remove from blacklist
pub async fn remove_from_blacklist(
    State(state): State<Arc<GreylistState>>,
    Path(pattern): Path<String>,
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

    state
        .manager
        .remove_from_blacklist(&pattern)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/admin/greylisting/cleanup - Cleanup old entries
pub async fn cleanup_entries(
    State(state): State<Arc<GreylistState>>,
    headers: HeaderMap,
) -> Result<Json<CleanupResponse>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let removed = state.manager.cleanup_old_entries().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(CleanupResponse {
        removed_count: removed,
    }))
}
