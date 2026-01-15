//! API endpoints for Sieve script management

use crate::api::auth::get_session_email;
use crate::sieve::{CreateSieveScriptRequest, SieveManager, SieveScript, ValidateSieveScriptRequest, ValidationResult, SieveLog};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// App state containing Sieve manager
pub struct SieveState {
    pub manager: Arc<SieveManager>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// GET /api/sieve/scripts - List all scripts for current user
pub async fn list_scripts(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<SieveScript>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let scripts = state.manager.list_scripts(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(scripts))
}

/// GET /api/sieve/scripts/:id - Get a specific script
pub async fn get_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<SieveScript>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let script = state
        .manager
        .get_script(&email, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error: "Script not found".to_string(),
                }),
            )
        })?;

    Ok(Json(script))
}

/// GET /api/sieve/scripts/active - Get active script
pub async fn get_active_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
) -> Result<Json<Option<SieveScript>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let script = state.manager.get_active_script(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(Json(script))
}

/// POST /api/sieve/scripts - Create a new script
pub async fn create_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateSieveScriptRequest>,
) -> Result<Json<SieveScript>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let script = state
        .manager
        .create_script(&email, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(script))
}

/// PUT /api/sieve/scripts/:id - Update a script
pub async fn update_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(payload): Json<CreateSieveScriptRequest>,
) -> Result<Json<SieveScript>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let script = state
        .manager
        .update_script(&email, &id, &payload)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(script))
}

/// DELETE /api/sieve/scripts/:id - Delete a script
pub async fn delete_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
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
        .delete_script(&email, &id)
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/sieve/scripts/:id/activate - Activate a script
pub async fn activate_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
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
        .activate_script(&email, &id)
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

/// POST /api/sieve/scripts/:id/deactivate - Deactivate a script
pub async fn deactivate_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Path(id): Path<String>,
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
        .deactivate_script(&email, &id)
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

/// POST /api/sieve/validate - Validate a script without saving
pub async fn validate_script(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    Json(payload): Json<ValidateSieveScriptRequest>,
) -> Result<Json<ValidationResult>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let result = state
        .manager
        .validate_script(&payload.script_content)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(result))
}

/// Query params for logs
#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    100
}

/// GET /api/sieve/logs - Get execution logs
pub async fn get_logs(
    State(state): State<Arc<SieveState>>,
    headers: HeaderMap,
    axum::extract::Query(query): axum::extract::Query<LogsQuery>,
) -> Result<Json<Vec<SieveLog>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let logs = state
        .manager
        .get_logs(&email, query.limit)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(logs))
}

/// DELETE /api/sieve/logs - Clear execution logs
pub async fn clear_logs(
    State(state): State<Arc<SieveState>>,
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

    state.manager.clear_logs(&email).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError {
                error: e.to_string(),
            }),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}
