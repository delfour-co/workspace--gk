//! API endpoints for MFA management

use crate::api::auth::get_session_email;
use crate::mfa::{MfaManager, MfaSetupResponse, MfaStatusResponse, MfaVerifyRequest, MfaVerifyResult};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// App state containing MFA manager
pub struct MfaState {
    pub manager: Arc<MfaManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// Backup codes response
#[derive(Serialize)]
pub struct BackupCodesResponse {
    pub codes: Vec<String>,
    pub message: String,
}

/// GET /api/mfa/status - Get MFA status for current user
pub async fn get_status(
    State(state): State<Arc<MfaState>>,
    headers: HeaderMap,
) -> Result<Json<MfaStatusResponse>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let status = state.manager.get_status(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(status))
}

/// POST /api/mfa/setup - Start MFA setup (generates secret and QR code)
pub async fn start_setup(
    State(state): State<Arc<MfaState>>,
    headers: HeaderMap,
) -> Result<Json<MfaSetupResponse>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let setup = state.manager.start_setup(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(setup))
}

/// POST /api/mfa/verify - Verify TOTP code and enable MFA
pub async fn complete_setup(
    State(state): State<Arc<MfaState>>,
    headers: HeaderMap,
    Json(payload): Json<MfaVerifyRequest>,
) -> Result<Json<BackupCodesResponse>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let codes = state
        .manager
        .complete_setup(&email, &payload.code)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(BackupCodesResponse {
        codes,
        message: "MFA enabled successfully. Save these backup codes in a safe place.".to_string(),
    }))
}

/// POST /api/mfa/disable - Disable MFA (requires current code)
pub async fn disable(
    State(state): State<Arc<MfaState>>,
    headers: HeaderMap,
    Json(payload): Json<MfaVerifyRequest>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    state
        .manager
        .disable(&email, &payload.code)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::OK)
}

/// POST /api/mfa/backup-codes - Generate new backup codes
pub async fn regenerate_backup_codes(
    State(state): State<Arc<MfaState>>,
    headers: HeaderMap,
    Json(payload): Json<MfaVerifyRequest>,
) -> Result<Json<BackupCodesResponse>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    // Verify current code first
    let result = state
        .manager
        .verify(&email, &payload.code)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    if result != MfaVerifyResult::Valid {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: "Invalid TOTP code".to_string(),
            }),
        ));
    }

    let codes = state
        .manager
        .generate_backup_codes(&email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(BackupCodesResponse {
        codes,
        message: "New backup codes generated. Previous codes are now invalid.".to_string(),
    }))
}

/// MFA verification for login flow
#[derive(Debug, Deserialize)]
pub struct MfaLoginRequest {
    pub email: String,
    pub code: String,
}

/// POST /api/mfa/check - Verify MFA code during login
pub async fn verify_login(
    State(state): State<Arc<MfaState>>,
    Json(payload): Json<MfaLoginRequest>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let result = state
        .manager
        .verify(&payload.email, &payload.code)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    match result {
        MfaVerifyResult::Valid => Ok(Json(serde_json::json!({
            "success": true,
            "message": "MFA verification successful"
        }))),
        MfaVerifyResult::NotEnabled => Ok(Json(serde_json::json!({
            "success": true,
            "message": "MFA not enabled for this account"
        }))),
        MfaVerifyResult::Invalid => Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Invalid MFA code".to_string(),
            }),
        )),
        MfaVerifyResult::AlreadyUsed => Err((
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Code already used".to_string(),
            }),
        )),
        MfaVerifyResult::RateLimited => Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(ApiError {
                error: "Too many attempts. Please try again later.".to_string(),
            }),
        )),
    }
}

/// GET /api/mfa/required/:email - Check if MFA is required for login
pub async fn is_mfa_required(
    State(state): State<Arc<MfaState>>,
    axum::extract::Path(email): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    let is_enabled = state.manager.is_enabled(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(serde_json::json!({
        "mfa_required": is_enabled
    })))
}
