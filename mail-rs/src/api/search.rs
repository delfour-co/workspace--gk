//! Search API endpoints
//!
//! Provides REST API for full-text email search.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::get_session_email;
use crate::search::{IndexStatus, SearchManager, SearchQuery, SearchResults};

/// Search API state
pub struct SearchState {
    pub search_manager: Arc<SearchManager>,
}

/// Search request query parameters
#[derive(Debug, Deserialize)]
pub struct SearchParams {
    /// Search query string
    pub q: String,
    /// Optional folder filter
    pub folder: Option<String>,
    /// Optional start date (ISO 8601)
    pub from_date: Option<String>,
    /// Optional end date (ISO 8601)
    pub to_date: Option<String>,
    /// Results limit (default 20)
    pub limit: Option<usize>,
    /// Results offset (default 0)
    pub offset: Option<usize>,
}

/// Reindex request
#[derive(Debug, Deserialize)]
pub struct ReindexRequest {
    /// Optional specific user email to reindex (admin only)
    pub email: Option<String>,
}

/// Reindex response
#[derive(Debug, Serialize)]
pub struct ReindexResponse {
    pub success: bool,
    pub indexed_count: u64,
    pub message: String,
}

/// Error response
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

/// Search emails
pub async fn search_emails(
    State(state): State<Arc<SearchState>>,
    headers: HeaderMap,
    Query(params): Query<SearchParams>,
) -> Result<Json<SearchResults>, (StatusCode, Json<ErrorResponse>)> {
    let email = get_session_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Unauthorized".to_string() })))?;

    // Parse dates if provided
    let from_date = params.from_date.as_ref().and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok().map(|dt| dt.with_timezone(&chrono::Utc)));
    let to_date = params.to_date.as_ref().and_then(|d| chrono::DateTime::parse_from_rfc3339(d).ok().map(|dt| dt.with_timezone(&chrono::Utc)));

    let query = SearchQuery {
        query: params.q,
        folder: params.folder,
        from_date,
        to_date,
        limit: params.limit,
        offset: params.offset,
    };

    match state.search_manager.search(&email, query).await {
        Ok(results) => Ok(Json(results)),
        Err(e) => {
            tracing::error!("Search error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

/// Get index status
pub async fn get_index_status(
    State(state): State<Arc<SearchState>>,
    headers: HeaderMap,
) -> Result<Json<IndexStatus>, (StatusCode, Json<ErrorResponse>)> {
    let _email = get_session_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Unauthorized".to_string() })))?;

    match state.search_manager.get_status().await {
        Ok(status) => Ok(Json(status)),
        Err(e) => {
            tracing::error!("Failed to get index status: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

/// Trigger reindexing
pub async fn reindex(
    State(state): State<Arc<SearchState>>,
    headers: HeaderMap,
    Json(request): Json<ReindexRequest>,
) -> Result<Json<ReindexResponse>, (StatusCode, Json<ErrorResponse>)> {
    let email = get_session_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Unauthorized".to_string() })))?;

    // If specific email provided (admin feature), use that, otherwise reindex current user
    let target_email = request.email.unwrap_or(email);

    match state.search_manager.reindex_user(&target_email).await {
        Ok(count) => Ok(Json(ReindexResponse {
            success: true,
            indexed_count: count,
            message: format!("Successfully indexed {} emails for {}", count, target_email),
        })),
        Err(e) => {
            tracing::error!("Reindex error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

/// Reindex all users (admin only)
pub async fn reindex_all(
    State(state): State<Arc<SearchState>>,
    headers: HeaderMap,
) -> Result<Json<ReindexResponse>, (StatusCode, Json<ErrorResponse>)> {
    let _email = get_session_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Unauthorized".to_string() })))?;

    match state.search_manager.reindex_all().await {
        Ok(count) => Ok(Json(ReindexResponse {
            success: true,
            indexed_count: count,
            message: format!("Successfully indexed {} emails across all users", count),
        })),
        Err(e) => {
            tracing::error!("Reindex all error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

/// Clear the search index (admin only)
pub async fn clear_index(
    State(state): State<Arc<SearchState>>,
    headers: HeaderMap,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    let _email = get_session_email(&headers)
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error: "Unauthorized".to_string() })))?;

    match state.search_manager.clear_index().await {
        Ok(_) => Ok(Json(serde_json::json!({
            "success": true,
            "message": "Search index cleared successfully"
        }))),
        Err(e) => {
            tracing::error!("Clear index error: {}", e);
            Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() })))
        }
    }
}

