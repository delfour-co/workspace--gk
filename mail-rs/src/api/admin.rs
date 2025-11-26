//! Admin API Handlers - Simplified version
//!
//! Basic admin endpoints for user and system management

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use super::handlers::{ApiError, AppState};

/// User response
#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub email: String,
    pub created_at: String,
}

/// System statistics response
#[derive(Debug, Serialize)]
pub struct SystemStatsResponse {
    pub total_users: i64,
    pub version: String,
}

/// User creation request
#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub email: String,
    pub password: String,
}

/// List all users (admin only)
pub async fn list_users(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<UserResponse>>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Listing users");

    let db = &state.authenticator.db;

    let users = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, email, created_at FROM users ORDER BY created_at DESC LIMIT 100"
    )
    .fetch_all(&**db)
    .await
    .map_err(|e| {
        error!("Failed to fetch users: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Failed to fetch users"))
        )
    })?;

    let items = users
        .into_iter()
        .map(|(id, email, created_at)| UserResponse {
            id,
            email,
            created_at,
        })
        .collect();

    Ok(Json(items))
}

/// Get user by ID
pub async fn get_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Result<Json<UserResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Getting user {}", user_id);

    let db = &state.authenticator.db;

    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, email, created_at FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_optional(&**db)
    .await
    .map_err(|e| {
        error!("Failed to fetch user: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Failed to fetch user"))
        )
    })?;

    match user {
        Some((id, email, created_at)) => {
            Ok(Json(UserResponse {
                id,
                email,
                created_at,
            }))
        }
        None => Err((
            StatusCode::NOT_FOUND,
            Json(ApiError::new("User not found"))
        )),
    }
}

/// Create a new user
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserResponse>), (StatusCode, Json<ApiError>)> {
    info!("Admin: Creating user {}", req.email);

    if !req.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("Invalid email format"))
        ));
    }

    if req.password.len() < 8 {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError::new("Password must be at least 8 characters"))
        ));
    }

    let password_hash = state
        .authenticator
        .hash_password(&req.password)
        .map_err(|e| {
            error!("Failed to hash password: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to create user"))
            )
        })?;

    let db = &state.authenticator.db;

    let result = sqlx::query(
        "INSERT INTO users (email, password_hash) VALUES (?, ?)"
    )
    .bind(&req.email)
    .bind(&password_hash)
    .execute(&**db)
    .await
    .map_err(|e| {
        error!("Failed to create user: {}", e);
        let msg = if e.to_string().contains("UNIQUE") {
            "User already exists"
        } else {
            "Failed to create user"
        };
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(msg))
        )
    })?;

    let user_id = result.last_insert_rowid();

    let user = sqlx::query_as::<_, (i64, String, String)>(
        "SELECT id, email, created_at FROM users WHERE id = ?"
    )
    .bind(user_id)
    .fetch_one(&**db)
    .await
    .map_err(|e| {
        error!("Failed to fetch created user: {}", e);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Failed to fetch user"))
        )
    })?;

    let (id, email, created_at) = user;

    info!("User {} created successfully", email);

    Ok((
        StatusCode::CREATED,
        Json(UserResponse {
            id,
            email,
            created_at,
        }),
    ))
}

/// Update user (placeholder)
pub async fn update_user(
    Path(_user_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    Ok(StatusCode::NOT_IMPLEMENTED)
}

/// Delete user
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    info!("Admin: Deleting user {}", user_id);

    let db = &state.authenticator.db;

    let result = sqlx::query("DELETE FROM users WHERE id = ?")
        .bind(user_id)
        .execute(&**db)
        .await
        .map_err(|e| {
            error!("Failed to delete user: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to delete user"))
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ApiError::new("User not found"))
        ));
    }

    info!("User {} deleted successfully", user_id);

    Ok(StatusCode::NO_CONTENT)
}

/// Get system statistics
pub async fn get_system_stats(
    State(state): State<Arc<AppState>>,
) -> Result<Json<SystemStatsResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Getting system statistics");

    let db = &state.authenticator.db;

    let total_users = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
        .fetch_one(&**db)
        .await
        .map_err(|e| {
            error!("Failed to count users: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to get statistics"))
            )
        })?;

    Ok(Json(SystemStatsResponse {
        total_users,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// Get server configuration (placeholder)
pub async fn get_config() -> Result<Json<serde_json::Value>, (StatusCode, Json<ApiError>)> {
    Ok(Json(serde_json::json!({
        "version": env!("CARGO_PKG_VERSION"),
        "status": "ok"
    })))
}
