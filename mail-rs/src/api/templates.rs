//! API endpoints for email templates management

use crate::api::auth::get_session_email;
use crate::error::MailError;
use crate::templates::{
    EmailTemplate, TemplateCategory, TemplateManager, TemplateRenderer, TemplateVariable,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    Json,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// App state containing template manager
pub struct TemplateState {
    pub template_manager: Arc<TemplateManager>,
}

/// Request to create a new template
#[derive(Debug, Deserialize)]
pub struct CreateTemplateRequest {
    pub name: String,
    pub category: TemplateCategory,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub variables: Vec<TemplateVariable>,
    pub is_signature: bool,
}

/// Request to update a template
#[derive(Debug, Deserialize)]
pub struct UpdateTemplateRequest {
    pub name: Option<String>,
    pub subject: Option<String>,
    pub body_html: Option<String>,
    pub body_text: Option<String>,
    pub variables: Option<Vec<TemplateVariable>>,
    pub is_signature: Option<bool>,
}

/// Response with error details
#[derive(Serialize)]
pub struct ApiError {
    pub error: String,
}

/// GET /api/templates - List all templates for authenticated user
pub async fn list_templates(
    State(state): State<Arc<TemplateState>>,
    headers: HeaderMap,
) -> Result<Json<Vec<EmailTemplate>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiError {
                    error: "Not authenticated".to_string(),
                }),
            )
        })?;

    let templates = state
        .template_manager
        .list_templates(&email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(templates))
}

/// GET /api/templates/category/:category - List templates by category
pub async fn list_templates_by_category(
    State(state): State<Arc<TemplateState>>,
    Path(category_str): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Vec<EmailTemplate>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let category = match category_str.as_str() {
        "signature" => TemplateCategory::Signature,
        "quick_reply" => TemplateCategory::QuickReply,
        "custom" => TemplateCategory::Custom,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ApiError {
                    error: format!("Invalid category: {}", category_str),
                }),
            ))
        }
    };

    let templates = state
        .template_manager
        .list_templates_by_category(&email, category)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(templates))
}

/// GET /api/templates/:id - Get a specific template
pub async fn get_template(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Result<Json<EmailTemplate>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let template = state
        .template_manager
        .get_template(&id)
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
                    error: "Template not found".to_string(),
                }),
            )
        })?;

    Ok(Json(template))
}

/// POST /api/templates - Create a new template
pub async fn create_template(
    State(state): State<Arc<TemplateState>>,
    headers: HeaderMap,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<(StatusCode, Json<EmailTemplate>), (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let request = crate::templates::types::CreateTemplateRequest {
        name: payload.name,
        category: payload.category,
        subject: payload.subject,
        body_html: payload.body_html,
        body_text: payload.body_text,
        variables: payload.variables,
        is_signature: payload.is_signature,
    };

    let template = state
        .template_manager
        .create_template(&email, request)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok((StatusCode::CREATED, Json(template)))
}

/// PUT /api/templates/:id - Update a template
pub async fn update_template(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<UpdateTemplateRequest>,
) -> Result<Json<EmailTemplate>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let request = crate::templates::types::UpdateTemplateRequest {
        name: payload.name,
        subject: payload.subject,
        body_html: payload.body_html,
        body_text: payload.body_text,
        variables: payload.variables,
        is_signature: payload.is_signature,
    };

    let template = state
        .template_manager
        .update_template(&id, &email, request)
        .await
        .map_err(|e| match e {
            MailError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
            MailError::Unauthorized(_) => (
                StatusCode::FORBIDDEN,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
        })?;

    Ok(Json(template))
}

/// DELETE /api/templates/:id - Delete a template
pub async fn delete_template(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
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

    state
        .template_manager
        .delete_template(&id, &email)
        .await
        .map_err(|e| match e {
            MailError::NotFound(_) => (
                StatusCode::NOT_FOUND,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
            MailError::Unauthorized(_) => (
                StatusCode::FORBIDDEN,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            ),
        })?;

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/templates/:id/render - Preview template with variables
#[derive(Deserialize)]
pub struct RenderRequest {
    pub variables: HashMap<String, String>,
}

#[derive(Serialize)]
pub struct RenderResponse {
    pub html: String,
    pub text: String,
    pub subject: String,
}

pub async fn render_template(
    State(state): State<Arc<TemplateState>>,
    Path(id): Path<String>,
    headers: HeaderMap,
    Json(payload): Json<RenderRequest>,
) -> Result<Json<RenderResponse>, (StatusCode, Json<ApiError>)> {
    let _email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let template = state
        .template_manager
        .get_template(&id)
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
                    error: "Template not found".to_string(),
                }),
            )
        })?;

    // Validate required variables
    if let Err(missing) = TemplateRenderer::validate_variables(&template, &payload.variables) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ApiError {
                error: format!("Missing required variables: {}", missing.join(", ")),
            }),
        ));
    }

    let (html, text) = TemplateRenderer::render(&template, &payload.variables);
    let subject = TemplateRenderer::render_subject(&template.subject, &payload.variables);

    Ok(Json(RenderResponse {
        html,
        text,
        subject,
    }))
}

/// GET /api/templates/signature/default - Get default signature
pub async fn get_default_signature(
    State(state): State<Arc<TemplateState>>,
    headers: HeaderMap,
) -> Result<Json<Option<EmailTemplate>>, (StatusCode, Json<ApiError>)> {
    let email = get_session_email(&headers).ok_or_else(|| {
        (
            StatusCode::UNAUTHORIZED,
            Json(ApiError {
                error: "Not authenticated".to_string(),
            }),
        )
    })?;

    let signature = state
        .template_manager
        .get_default_signature(&email)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ApiError {
                    error: e.to_string(),
                }),
            )
        })?;

    Ok(Json(signature))
}
