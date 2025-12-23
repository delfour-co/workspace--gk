//! Auto-reply API endpoints

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::get_session_email;
use crate::auto_reply::{
    AutoReplyConfig, AutoReplyManager, CreateAutoReplyRequest, UpdateAutoReplyRequest,
};

/// Shared state for auto-reply endpoints
pub struct AutoReplyState {
    pub manager: Arc<AutoReplyManager>,
}

/// API error response
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// GET /api/auto-reply - Get current auto-reply configuration
pub async fn get_auto_reply(
    State(state): State<Arc<AutoReplyState>>,
    headers: HeaderMap,
) -> Result<Json<Option<AutoReplyConfig>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let config = state.manager.get_config(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(config))
}

/// POST /api/auto-reply - Create or update auto-reply configuration
pub async fn create_auto_reply(
    State(state): State<Arc<AutoReplyState>>,
    headers: HeaderMap,
    Json(request): Json<CreateAutoReplyRequest>,
) -> Result<(StatusCode, Json<AutoReplyConfig>), (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let config = state
        .manager
        .set_config(&email, request)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((StatusCode::OK, Json(config)))
}

/// PUT /api/auto-reply - Update auto-reply configuration
pub async fn update_auto_reply(
    State(state): State<Arc<AutoReplyState>>,
    headers: HeaderMap,
    Json(request): Json<UpdateAutoReplyRequest>,
) -> Result<Json<AutoReplyConfig>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let config = state
        .manager
        .update_config(&email, request)
        .await
        .map_err(|e| {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ApiError { error: e.to_string() }))
        })?;

    Ok(Json(config))
}

/// DELETE /api/auto-reply - Delete auto-reply configuration
pub async fn delete_auto_reply(
    State(state): State<Arc<AutoReplyState>>,
    headers: HeaderMap,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state.manager.delete_config(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/auto-reply/toggle - Quick toggle active status
#[derive(Deserialize)]
pub struct ToggleRequest {
    pub is_active: bool,
}

pub async fn toggle_auto_reply(
    State(state): State<Arc<AutoReplyState>>,
    headers: HeaderMap,
    Json(request): Json<ToggleRequest>,
) -> Result<Json<AutoReplyConfig>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let update = UpdateAutoReplyRequest {
        is_active: Some(request.is_active),
        start_date: None,
        end_date: None,
        subject: None,
        body_html: None,
        body_text: None,
        reply_interval_hours: None,
    };

    let config = state
        .manager
        .update_config(&email, update)
        .await
        .map_err(|e| {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };
            (status, Json(ApiError { error: e.to_string() }))
        })?;

    Ok(Json(config))
}
