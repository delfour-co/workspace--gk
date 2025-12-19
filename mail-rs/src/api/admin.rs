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

// ========== DNS CONFIGURATION ==========

use crate::admin::dns::DnsConfigGenerator;
use std::net::IpAddr;

/// DNS configuration response
#[derive(Debug, Serialize)]
pub struct DnsConfigResponse {
    pub domain: String,
    pub hostname: String,
    pub ip: String,
    pub records: Vec<DnsRecordResponse>,
    pub instructions: String,
}

#[derive(Debug, Serialize)]
pub struct DnsRecordResponse {
    pub record_type: String,
    pub name: String,
    pub value: String,
    pub ttl: u32,
    pub priority: Option<u16>,
    pub description: String,
}

/// Get DNS configuration
pub async fn get_dns_config(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DnsConfigResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Getting DNS configuration");

    // TODO: Read from actual configuration
    let domain = "example.com".to_string();
    let hostname = "mail.example.com".to_string();
    let ip: IpAddr = "203.0.113.10".parse().unwrap();

    let generator = DnsConfigGenerator::new(
        domain.clone(),
        hostname.clone(),
        ip,
        "default".to_string(),
    );

    let records = generator.generate_records()
        .map_err(|e| {
            error!("Failed to generate DNS records: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to generate DNS records"))
            )
        })?;

    let instructions = generator.generate_instructions()
        .map_err(|e| {
            error!("Failed to generate DNS instructions: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to generate DNS instructions"))
            )
        })?;

    let records_response = records.into_iter().map(|r| DnsRecordResponse {
        record_type: r.record_type.to_string(),
        name: r.name,
        value: r.value,
        ttl: r.ttl,
        priority: r.priority,
        description: r.description,
    }).collect();

    Ok(Json(DnsConfigResponse {
        domain,
        hostname,
        ip: ip.to_string(),
        records: records_response,
        instructions,
    }))
}

// ========== SYSTEM DIAGNOSTICS ==========

use crate::admin::diagnostics::SystemDiagnostics;

/// Diagnostics response
#[derive(Debug, Serialize)]
pub struct DiagnosticsResponse {
    pub overall_status: String,
    pub checks: Vec<DiagnosticCheckResponse>,
    pub report: String,
}

#[derive(Debug, Serialize)]
pub struct DiagnosticCheckResponse {
    pub name: String,
    pub status: String,
    pub message: String,
    pub value: Option<String>,
    pub timestamp: String,
}

/// Get system diagnostics
pub async fn get_diagnostics(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<DiagnosticsResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Running system diagnostics");

    // TODO: Read from actual configuration
    let maildir_path = "/var/mail".to_string();
    let diagnostics = SystemDiagnostics::new(maildir_path);

    let results = diagnostics.run_all_checks().await
        .map_err(|e| {
            error!("Failed to run diagnostics: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to run diagnostics"))
            )
        })?;

    let overall_status = diagnostics.get_health_status().await
        .map_err(|e| {
            error!("Failed to get health status: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to get health status"))
            )
        })?;

    let report = diagnostics.generate_report().await
        .map_err(|e| {
            error!("Failed to generate report: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to generate report"))
            )
        })?;

    let checks = results.into_iter().map(|r| DiagnosticCheckResponse {
        name: r.name,
        status: r.status.to_string(),
        message: r.message,
        value: r.value,
        timestamp: r.timestamp.to_rfc3339(),
    }).collect();

    Ok(Json(DiagnosticsResponse {
        overall_status: overall_status.to_string(),
        checks,
        report,
    }))
}

// ========== BACKUP MANAGEMENT ==========

use crate::admin::backup::BackupManager;

/// Backup response
#[derive(Debug, Serialize)]
pub struct BackupResponse {
    pub filename: String,
    pub created_at: String,
    pub size_bytes: u64,
    pub status: String,
    pub error: Option<String>,
}

/// Backups list response
#[derive(Debug, Serialize)]
pub struct BackupsListResponse {
    pub backups: Vec<BackupResponse>,
    pub total_size: u64,
}

/// Create backup request
#[derive(Debug, Deserialize)]
pub struct CreateBackupRequest {
    // Future: could add options like compress, custom path, etc.
}

/// List all backups
pub async fn list_backups(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<BackupsListResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Listing backups");

    let manager = BackupManager::with_defaults();

    let backups = manager.list_backups().await
        .map_err(|e| {
            error!("Failed to list backups: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to list backups"))
            )
        })?;

    let total_size = manager.get_total_backup_size().await
        .map_err(|e| {
            error!("Failed to get total backup size: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to get backup size"))
            )
        })?;

    let backups_response = backups.into_iter().map(|b| BackupResponse {
        filename: b.filename,
        created_at: b.created_at.to_rfc3339(),
        size_bytes: b.size_bytes,
        status: b.status.to_string(),
        error: b.error,
    }).collect();

    Ok(Json(BackupsListResponse {
        backups: backups_response,
        total_size,
    }))
}

/// Create a new backup
pub async fn create_backup(
    State(_state): State<Arc<AppState>>,
    Json(_req): Json<CreateBackupRequest>,
) -> Result<(StatusCode, Json<BackupResponse>), (StatusCode, Json<ApiError>)> {
    info!("Admin: Creating backup");

    let manager = BackupManager::with_defaults();

    let metadata = manager.create_backup().await
        .map_err(|e| {
            error!("Failed to create backup: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to create backup"))
            )
        })?;

    Ok((
        StatusCode::CREATED,
        Json(BackupResponse {
            filename: metadata.filename,
            created_at: metadata.created_at.to_rfc3339(),
            size_bytes: metadata.size_bytes,
            status: metadata.status.to_string(),
            error: metadata.error,
        }),
    ))
}

/// Delete a backup
pub async fn delete_backup(
    State(_state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    info!("Admin: Deleting backup {}", filename);

    let manager = BackupManager::with_defaults();

    manager.delete_backup(&filename).await
        .map_err(|e| {
            error!("Failed to delete backup: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to delete backup"))
            )
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// Restore from backup
pub async fn restore_backup(
    State(_state): State<Arc<AppState>>,
    Path(filename): Path<String>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    info!("Admin: Restoring backup {}", filename);

    let manager = BackupManager::with_defaults();

    manager.restore_backup(&filename).await
        .map_err(|e| {
            error!("Failed to restore backup: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to restore backup"))
            )
        })?;

    Ok(StatusCode::OK)
}

// ========== SSL CERTIFICATE MANAGEMENT ==========

use crate::admin::ssl::{SslManager, SslConfig};

/// SSL certificate response
#[derive(Debug, Serialize)]
pub struct SslCertificateResponse {
    pub domain: String,
    pub status: String,
    pub expires_at: Option<String>,
    pub days_until_expiry: Option<i64>,
    pub issuer: Option<String>,
}

/// Get SSL certificate status
pub async fn get_ssl_status(
    State(_state): State<Arc<AppState>>,
) -> Result<Json<SslCertificateResponse>, (StatusCode, Json<ApiError>)> {
    info!("Admin: Getting SSL certificate status");

    // TODO: Read from actual configuration
    let config = SslConfig {
        domain: "mail.example.com".to_string(),
        email: "admin@example.com".to_string(),
        ..Default::default()
    };

    let manager = SslManager::new(config.clone());

    let cert_info = manager.get_certificate_info().await
        .map_err(|e| {
            error!("Failed to get certificate info: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to get certificate info"))
            )
        })?;

    Ok(Json(SslCertificateResponse {
        domain: cert_info.domain,
        status: cert_info.status.to_string(),
        expires_at: cert_info.expires_at.map(|d| d.to_rfc3339()),
        days_until_expiry: cert_info.days_until_expiry,
        issuer: cert_info.issuer,
    }))
}

/// Request new SSL certificate
pub async fn request_ssl_certificate(
    State(_state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    info!("Admin: Requesting SSL certificate");

    let config = SslConfig {
        domain: "mail.example.com".to_string(),
        email: "admin@example.com".to_string(),
        ..Default::default()
    };

    let manager = SslManager::new(config);

    manager.request_certificate().await
        .map_err(|e| {
            error!("Failed to request certificate: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to request certificate"))
            )
        })?;

    Ok(StatusCode::OK)
}

/// Renew SSL certificate
pub async fn renew_ssl_certificate(
    State(_state): State<Arc<AppState>>,
) -> Result<StatusCode, (StatusCode, Json<ApiError>)> {
    info!("Admin: Renewing SSL certificate");

    let config = SslConfig {
        domain: "mail.example.com".to_string(),
        email: "admin@example.com".to_string(),
        ..Default::default()
    };

    let manager = SslManager::new(config);

    manager.renew_certificate().await
        .map_err(|e| {
            error!("Failed to renew certificate: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError::new("Failed to renew certificate"))
            )
        })?;

    Ok(StatusCode::OK)
}
