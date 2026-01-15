//! API endpoints for quota management

use crate::api::auth::get_session_email;
use crate::quota::manager::QuotaManager;
use crate::quota::types::UserQuota;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// App state containing quota manager
pub struct QuotaState {
    pub manager: Arc<QuotaManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Stats response for quota dashboard
#[derive(Serialize)]
pub struct QuotaStatsResponse {
    pub total_users: usize,
    pub total_storage_used: u64,
    pub total_storage_limit: u64,
    pub users_near_limit: usize,
    pub users_over_limit: usize,
}

/// Request to update user quota
#[derive(Deserialize)]
pub struct UpdateQuotaRequest {
    pub storage_limit: Option<u64>,
    pub message_limit_daily: Option<u32>,
    pub max_message_size: Option<u64>,
}

/// Default quotas configuration
#[derive(Serialize, Deserialize)]
pub struct DefaultQuotasRequest {
    pub storage_limit: u64,
    pub message_limit_daily: u32,
    pub max_message_size: u64,
}

/// GET /api/admin/quotas/stats - Get quota statistics
pub async fn get_stats(
    State(state): State<Arc<QuotaState>>,
    headers: HeaderMap,
) -> Result<Json<QuotaStatsResponse>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let quotas = state.manager.list_quotas().await;

    let total_storage_used: u64 = quotas.iter().map(|q| q.storage_used).sum();
    let total_storage_limit: u64 = quotas.iter().map(|q| q.storage_limit).sum();
    let users_near_limit = quotas
        .iter()
        .filter(|q| {
            let percent = q.storage_usage_percent();
            percent >= 80.0 && percent < 100.0
        })
        .count();
    let users_over_limit = quotas
        .iter()
        .filter(|q| q.storage_usage_percent() >= 100.0)
        .count();

    Ok(Json(QuotaStatsResponse {
        total_users: quotas.len(),
        total_storage_used,
        total_storage_limit,
        users_near_limit,
        users_over_limit,
    }))
}

/// GET /api/admin/quotas - List all user quotas
pub async fn list_quotas(
    State(state): State<Arc<QuotaState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<UserQuota>>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let quotas = state.manager.list_quotas().await;
    Ok(Json(quotas))
}

/// GET /api/admin/quotas/:email - Get specific user quota
pub async fn get_quota(
    State(state): State<Arc<QuotaState>>,
    Path(user_email): Path<String>,
    headers: HeaderMap,
) -> Result<Json<UserQuota>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let quota = state.manager.get_quota(&user_email).await;
    Ok(Json(quota))
}

/// PUT /api/admin/quotas/:email - Update user quota
pub async fn update_quota(
    State(state): State<Arc<QuotaState>>,
    Path(user_email): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateQuotaRequest>,
) -> Result<Json<UserQuota>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    // Get existing quota
    let mut quota = state.manager.get_quota(&user_email).await;

    // Apply updates
    if let Some(storage_limit) = payload.storage_limit {
        quota.storage_limit = storage_limit;
    }
    if let Some(message_limit) = payload.message_limit_daily {
        quota.message_limit_daily = message_limit;
    }
    if let Some(max_size) = payload.max_message_size {
        quota.max_message_size = max_size;
    }

    // Save
    state.manager.set_quota(quota.clone()).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(quota))
}

/// GET /api/admin/quotas/defaults - Get default quota settings
pub async fn get_defaults(
    State(_state): State<Arc<QuotaState>>,
    headers: HeaderMap,
) -> Result<Json<DefaultQuotasRequest>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    // Return default quota values
    let defaults = UserQuota::default();
    Ok(Json(DefaultQuotasRequest {
        storage_limit: defaults.storage_limit,
        message_limit_daily: defaults.message_limit_daily,
        max_message_size: defaults.max_message_size,
    }))
}

/// PUT /api/admin/quotas/defaults - Update default quota settings
pub async fn update_defaults(
    State(_state): State<Arc<QuotaState>>,
    headers: HeaderMap,
    Json(_payload): Json<DefaultQuotasRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    // TODO: Persist default quota settings to configuration
    // For now, just acknowledge the request
    // In a full implementation, this would update the QuotaManager's default_quota
    // and persist to a config file or database

    Ok(StatusCode::OK)
}

/// POST /api/admin/quotas/reset-daily - Reset daily message counts for all users
pub async fn reset_daily_counts(
    State(state): State<Arc<QuotaState>>,
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

    state.manager.reset_daily_counts().await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(StatusCode::OK)
}
