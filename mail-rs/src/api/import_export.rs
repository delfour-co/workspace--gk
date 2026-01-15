//! Import/Export API endpoints
//!
//! REST API for mailbox import and export operations.

use axum::{
    body::Bytes,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use crate::import_export::{
    ExportFormat, ExportJob, ExportRequest, ImportExportManager, ImportExportStats,
    ImportFormat, ImportJob, ImportRequest,
};

/// Import/Export API state
pub struct ImportExportState {
    pub manager: Arc<ImportExportManager>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

/// Export request body
#[derive(Debug, Deserialize)]
pub struct ExportRequestBody {
    pub email: String,
    pub folders: Option<Vec<String>>,
    pub format: Option<String>,
    pub include_subfolders: Option<bool>,
}

/// Get statistics
pub async fn get_stats(
    State(state): State<Arc<ImportExportState>>,
) -> Result<Json<ApiResponse<ImportExportStats>>, StatusCode> {
    let stats = state.manager.get_stats().await;
    Ok(Json(ApiResponse::success(stats)))
}

/// Start an export job
pub async fn start_export(
    State(state): State<Arc<ImportExportState>>,
    Json(body): Json<ExportRequestBody>,
) -> Result<Json<ApiResponse<ExportJob>>, StatusCode> {
    let format = match body.format.as_deref() {
        Some("mbox") | None => ExportFormat::Mbox,
        Some("eml") => ExportFormat::Eml,
        Some("zip") | Some("emlzip") => ExportFormat::EmlZip,
        _ => ExportFormat::Mbox,
    };

    let request = ExportRequest {
        email: body.email,
        folders: body.folders,
        format,
        include_subfolders: body.include_subfolders.unwrap_or(true),
        date_from: None,
        date_to: None,
    };

    match state.manager.start_export(request).await {
        Ok(job) => Ok(Json(ApiResponse::success(job))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to start export: {}", e)))),
    }
}

/// Get export job status
pub async fn get_export_job(
    State(state): State<Arc<ImportExportState>>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<ExportJob>>, StatusCode> {
    match state.manager.get_export_job(&job_id).await {
        Some(job) => Ok(Json(ApiResponse::success(job))),
        None => Ok(Json(ApiResponse::error("Export job not found"))),
    }
}

/// List export jobs
pub async fn list_export_jobs(
    State(state): State<Arc<ImportExportState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<ExportJob>>>, StatusCode> {
    let email = params.get("email").map(|s| s.as_str());
    let jobs = state.manager.list_export_jobs(email).await;
    Ok(Json(ApiResponse::success(jobs)))
}

/// Download export file
pub async fn download_export(
    State(state): State<Arc<ImportExportState>>,
    Path(job_id): Path<String>,
) -> Response {
    match state.manager.get_export_file(&job_id).await {
        Ok(Some((filename, content))) => {
            let content_type = if filename.ends_with(".mbox") {
                "application/mbox"
            } else if filename.ends_with(".zip") {
                "application/zip"
            } else {
                "message/rfc822"
            };

            let headers = [
                (
                    axum::http::header::CONTENT_TYPE,
                    content_type.to_string(),
                ),
                (
                    axum::http::header::CONTENT_DISPOSITION,
                    format!("attachment; filename=\"{}\"", filename),
                ),
            ];

            (headers, content).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ApiResponse::<()>::error("Export file not found")),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiResponse::<()>::error(&format!("Failed to get export: {}", e))),
        )
            .into_response(),
    }
}

/// Delete export
pub async fn delete_export(
    State(state): State<Arc<ImportExportState>>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.manager.delete_export(&job_id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete export: {}", e)))),
    }
}

/// Start an import job (multipart upload)
pub async fn start_import(
    State(state): State<Arc<ImportExportState>>,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<ImportJob>>, StatusCode> {
    let mut email = None;
    let mut target_folder = None;
    let mut format = None;
    let mut skip_duplicates = true;
    let mut preserve_dates = true;
    let mut file_data = None;

    while let Ok(Some(field)) = multipart.next_field().await {
        let name = field.name().map(|s| s.to_string());

        match name.as_deref() {
            Some("email") => {
                email = field.text().await.ok();
            }
            Some("target_folder") => {
                target_folder = field.text().await.ok();
            }
            Some("format") => {
                format = field.text().await.ok();
            }
            Some("skip_duplicates") => {
                skip_duplicates = field.text().await.ok().map(|s| s == "true").unwrap_or(true);
            }
            Some("preserve_dates") => {
                preserve_dates = field.text().await.ok().map(|s| s == "true").unwrap_or(true);
            }
            Some("file") => {
                file_data = field.bytes().await.ok().map(|b| b.to_vec());
            }
            _ => {}
        }
    }

    let email = match email {
        Some(e) => e,
        None => return Ok(Json(ApiResponse::error("Missing email field"))),
    };

    let data = match file_data {
        Some(d) => d,
        None => return Ok(Json(ApiResponse::error("Missing file field"))),
    };

    let import_format = match format.as_deref() {
        Some("mbox") | None => ImportFormat::Mbox,
        Some("eml") => ImportFormat::Eml,
        Some("zip") | Some("emlzip") => ImportFormat::EmlZip,
        _ => ImportFormat::Mbox,
    };

    let request = ImportRequest {
        email,
        target_folder,
        format: import_format,
        source_path: String::new(),
        skip_duplicates,
        preserve_dates,
    };

    match state.manager.start_import(request, data).await {
        Ok(job) => Ok(Json(ApiResponse::success(job))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to start import: {}", e)))),
    }
}

/// Get import job status
pub async fn get_import_job(
    State(state): State<Arc<ImportExportState>>,
    Path(job_id): Path<String>,
) -> Result<Json<ApiResponse<ImportJob>>, StatusCode> {
    match state.manager.get_import_job(&job_id).await {
        Some(job) => Ok(Json(ApiResponse::success(job))),
        None => Ok(Json(ApiResponse::error("Import job not found"))),
    }
}

/// List import jobs
pub async fn list_import_jobs(
    State(state): State<Arc<ImportExportState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<ImportJob>>>, StatusCode> {
    let email = params.get("email").map(|s| s.as_str());
    let jobs = state.manager.list_import_jobs(email).await;
    Ok(Json(ApiResponse::success(jobs)))
}

/// Cancel a job
pub async fn cancel_job(
    State(state): State<Arc<ImportExportState>>,
    Path((job_type, job_id)): Path<(String, String)>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    let is_export = job_type == "export";

    match state.manager.cancel_job(&job_id, is_export).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to cancel job: {}", e)))),
    }
}
