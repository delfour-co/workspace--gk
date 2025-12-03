//! API request handlers

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::api::auth::{Claims, JwtConfig};
use crate::imap::Mailbox;
use crate::security::{AuthMechanism, Authenticator};

/// Shared application state
pub struct AppState {
    pub authenticator: Authenticator,
    pub jwt_config: JwtConfig,
    pub maildir_root: String,
}

/// Login request body
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

/// Login response
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub email: String,
}

/// Email summary for list endpoint
#[derive(Debug, Serialize)]
pub struct EmailSummary {
    pub sequence: usize,
    pub uid: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub date: Option<String>,
    pub size: usize,
    pub flags: Vec<String>,
}

/// Email detail
#[derive(Debug, Serialize)]
pub struct EmailDetail {
    pub sequence: usize,
    pub uid: String,
    pub subject: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub date: Option<String>,
    pub body: String,
    pub flags: Vec<String>,
}

/// Folder info
#[derive(Debug, Serialize)]
pub struct FolderInfo {
    pub name: String,
    pub message_count: usize,
}

/// API error response
#[derive(Debug, Serialize)]
pub struct ApiError {
    pub error: String,
}

impl ApiError {
    pub fn new(msg: &str) -> Self {
        Self {
            error: msg.to_string(),
        }
    }
}

/// POST /api/auth/login - Authenticate and get JWT token
pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    // Verify credentials using PLAIN mechanism (email as username)
    match state.authenticator.authenticate(&req.email, &req.password).await {
        Ok(true) => {
            // Generate JWT token
            match state.jwt_config.create_token(&req.email) {
                Ok(token) => (
                    StatusCode::OK,
                    Json(LoginResponse {
                        token,
                        email: req.email,
                    }),
                )
                    .into_response(),
                Err(_) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(ApiError::new("Failed to create token")),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            Json(ApiError::new("Invalid credentials")),
        )
            .into_response(),
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Authentication error")),
        )
            .into_response(),
    }
}

/// GET /api/mails - List emails in INBOX
pub async fn list_emails(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> impl IntoResponse {
    let maildir_root = std::path::Path::new(&state.maildir_root);

    match Mailbox::open(&claims.sub, "INBOX", maildir_root) {
        Ok(mailbox) => {
            let emails: Vec<EmailSummary> = mailbox
                .messages()
                .iter()
                .map(|msg| {
                    let content_str = String::from_utf8_lossy(&msg.content);
                    let headers = content_str
                        .split("\r\n\r\n")
                        .next()
                        .unwrap_or(&content_str);

                    EmailSummary {
                        sequence: msg.sequence,
                        uid: msg.uid.clone(),
                        subject: extract_header(headers, "Subject"),
                        from: extract_header(headers, "From"),
                        date: extract_header(headers, "Date"),
                        size: msg.size,
                        flags: msg.flags.clone(),
                    }
                })
                .collect();

            (StatusCode::OK, Json(emails)).into_response()
        }
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::new("Mailbox not found")),
        )
            .into_response(),
    }
}

/// GET /api/mails/:id - Get email by sequence number
pub async fn get_email(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Path(sequence): Path<usize>,
) -> impl IntoResponse {
    let maildir_root = std::path::Path::new(&state.maildir_root);

    match Mailbox::open(&claims.sub, "INBOX", maildir_root) {
        Ok(mailbox) => match mailbox.get_message(sequence) {
            Some(msg) => {
                let content_str = String::from_utf8_lossy(&msg.content);
                let (headers, body) = if let Some(pos) = content_str.find("\r\n\r\n") {
                    (&content_str[..pos], &content_str[pos + 4..])
                } else {
                    (content_str.as_ref(), "")
                };

                let detail = EmailDetail {
                    sequence: msg.sequence,
                    uid: msg.uid.clone(),
                    subject: extract_header(headers, "Subject"),
                    from: extract_header(headers, "From"),
                    to: extract_header(headers, "To"),
                    date: extract_header(headers, "Date"),
                    body: body.to_string(),
                    flags: msg.flags.clone(),
                };

                (StatusCode::OK, Json(detail)).into_response()
            }
            None => (
                StatusCode::NOT_FOUND,
                Json(ApiError::new("Email not found")),
            )
                .into_response(),
        },
        Err(_) => (
            StatusCode::NOT_FOUND,
            Json(ApiError::new("Mailbox not found")),
        )
            .into_response(),
    }
}

/// GET /api/folders - List available folders
pub async fn list_folders(
    State(state): State<Arc<AppState>>,
    claims: Claims,
) -> impl IntoResponse {
    let maildir_root = std::path::Path::new(&state.maildir_root);

    match Mailbox::list_mailboxes(&claims.sub, maildir_root) {
        Ok(mailboxes) => {
            let folders: Vec<FolderInfo> = mailboxes
                .iter()
                .filter_map(|name| {
                    Mailbox::open(&claims.sub, name, maildir_root)
                        .ok()
                        .map(|mb| FolderInfo {
                            name: name.clone(),
                            message_count: mb.message_count(),
                        })
                })
                .collect();

            (StatusCode::OK, Json(folders)).into_response()
        }
        Err(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new("Failed to list folders")),
        )
            .into_response(),
    }
}

/// Send email request
#[derive(Debug, Deserialize)]
pub struct SendEmailRequest {
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Send email response
#[derive(Debug, Serialize)]
pub struct SendEmailResponse {
    pub message_id: String,
    pub status: String,
}

/// POST /api/mails/send - Send an email
pub async fn send_email(
    State(state): State<Arc<AppState>>,
    claims: Claims,
    Json(req): Json<SendEmailRequest>,
) -> impl IntoResponse {
    use crate::smtp::SmtpClient;
    use crate::utils::dns::lookup_mx;

    // Generate message ID
    let message_id = format!(
        "<{}.{}@mail-rs>",
        uuid::Uuid::new_v4(),
        chrono::Utc::now().timestamp()
    );

    // Build email content
    let email_content = format!(
        "From: {}\r\n\
         To: {}\r\n\
         Subject: {}\r\n\
         Message-ID: {}\r\n\
         Date: {}\r\n\
         MIME-Version: 1.0\r\n\
         Content-Type: text/plain; charset=utf-8\r\n\
         \r\n\
         {}",
        claims.sub,
        req.to,
        req.subject,
        message_id,
        chrono::Utc::now().format("%a, %d %b %Y %H:%M:%S +0000"),
        req.body
    );

    // Extract recipient domain
    let recipient_domain = match req.to.split('@').nth(1) {
        Some(domain) => domain,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("Invalid recipient email")),
            )
                .into_response()
        }
    };

    // Lookup MX record
    let mx_host = match lookup_mx(recipient_domain).await {
        Ok(hosts) if !hosts.is_empty() => hosts[0].clone(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(ApiError::new("Could not find mail server for recipient domain")),
            )
                .into_response()
        }
    };

    // Send email via SMTP
    let smtp_addr = format!("{}:25", mx_host);
    let client = SmtpClient::new(smtp_addr);

    match client.send_mail(&claims.sub, &req.to, email_content.as_bytes()).await {
        Ok(_) => (
            StatusCode::OK,
            Json(SendEmailResponse {
                message_id,
                status: "sent".to_string(),
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ApiError::new(&format!("Failed to send email: {}", e))),
        )
            .into_response(),
    }
}

/// Health check endpoint with detailed status
pub async fn health(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    use std::time::SystemTime;

    // Check database connectivity
    let db_healthy = state.authenticator.health_check().await.is_ok();

    // Check maildir is accessible
    let maildir_healthy = std::path::Path::new(&state.maildir_root).exists();

    // Overall status
    let healthy = db_healthy && maildir_healthy;
    let status = if healthy { "healthy" } else { "unhealthy" };
    let status_code = if healthy { StatusCode::OK } else { StatusCode::SERVICE_UNAVAILABLE };

    (
        status_code,
        Json(serde_json::json!({
            "status": status,
            "service": "mail-rs",
            "version": env!("CARGO_PKG_VERSION"),
            "timestamp": SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            "checks": {
                "database": if db_healthy { "ok" } else { "failed" },
                "maildir": if maildir_healthy { "ok" } else { "failed" }
            }
        }))
    )
}

/// GET /metrics - Prometheus metrics endpoint
pub async fn metrics(
    State(state): State<Arc<MetricsState>>,
) -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain; charset=utf-8")],
        state.metrics.to_prometheus(),
    )
}

/// Shared metrics state
pub struct MetricsState {
    pub metrics: crate::api::Metrics,
}

/// Helper: Extract header value from headers string
fn extract_header(headers: &str, name: &str) -> Option<String> {
    for line in headers.lines() {
        if line.to_lowercase().starts_with(&format!("{}:", name.to_lowercase())) {
            return line.split_once(':').map(|(_, v)| v.trim().to_string());
        }
    }
    None
}
