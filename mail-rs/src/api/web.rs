use askama_axum::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum::http::{StatusCode, header};
use serde::Deserialize;
use std::sync::Arc;

use crate::api::auth::get_session_email;
use crate::security::Authenticator;

// Session cookie names
const SESSION_COOKIE: &str = "admin_session";
const CHAT_SESSION_COOKIE: &str = "chat_session";

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    error: String,
}

#[derive(Template)]
#[template(path = "dashboard.html")]
struct DashboardTemplate {
    email: String,
    total_users: i64,
    version: String,
}

#[derive(Template)]
#[template(path = "users.html")]
struct UsersTemplate {
    email: String,
    users: Vec<UserRow>,
    success: String,
    error: String,
}

#[derive(Debug)]
struct UserRow {
    id: i32,
    email: String,
    created_at: String,
}

#[derive(Deserialize)]
pub struct LoginForm {
    email: String,
    password: String,
}

#[derive(Deserialize)]
pub struct CreateUserForm {
    email: String,
    password: String,
}

pub struct AppState {
    pub authenticator: Authenticator,
}

// Login page (GET)
pub async fn login_page() -> impl IntoResponse {
    LoginTemplate { error: String::new() }
}

// Login form submission (POST)
pub async fn login_submit(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    match state.authenticator.authenticate(&form.email, &form.password).await {
        Ok(true) => {
            // Set session cookie and redirect
            let cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Lax", SESSION_COOKIE, form.email);
            (
                StatusCode::SEE_OTHER,
                [(header::SET_COOKIE, cookie), (header::LOCATION, "/admin/dashboard".to_string())],
                "Redirecting..."
            ).into_response()
        }
        _ => {
            LoginTemplate {
                error: "Invalid email or password".to_string(),
            }.into_response()
        }
    }
}

// Logout
pub async fn logout() -> impl IntoResponse {
    let cookie = format!("{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0", SESSION_COOKIE);
    (
        StatusCode::SEE_OTHER,
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/admin/login".to_string())],
        "Redirecting..."
    ).into_response()
}

// Dashboard page
pub async fn dashboard(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    // Get user count
    let total_users = state.authenticator.count_users().await.unwrap_or(0);

    DashboardTemplate {
        email,
        total_users,
        version: env!("CARGO_PKG_VERSION").to_string(),
    }.into_response()
}

// Users page
pub async fn users_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    // Fetch all users
    let users = match state.authenticator.list_users().await {
        Ok(users) => users.into_iter().map(|(id, email, created_at)| UserRow {
            id,
            email,
            created_at,
        }).collect(),
        Err(_) => Vec::new(),
    };

    UsersTemplate {
        email,
        users,
        success: String::new(),
        error: String::new(),
    }.into_response()
}

// Create user (POST)
pub async fn create_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Form(form): Form<CreateUserForm>,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    // Create user
    match state.authenticator.create_user(&form.email, &form.password).await {
        Ok(_) => {
            // Fetch updated users list
            let users = match state.authenticator.list_users().await {
                Ok(users) => users.into_iter().map(|(id, email, created_at)| UserRow {
                    id,
                    email,
                    created_at,
                }).collect(),
                Err(_) => Vec::new(),
            };

            UsersTemplate {
                email,
                users,
                success: format!("User {} created successfully", form.email),
                error: String::new(),
            }.into_response()
        }
        Err(e) => {
            let users = match state.authenticator.list_users().await {
                Ok(users) => users.into_iter().map(|(id, email, created_at)| UserRow {
                    id,
                    email,
                    created_at,
                }).collect(),
                Err(_) => Vec::new(),
            };

            UsersTemplate {
                email,
                users,
                success: String::new(),
                error: format!("Failed to create user: {}", e),
            }.into_response()
        }
    }
}

// Delete user (DELETE via HTMX)
pub async fn delete_user(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Path(user_id): Path<i32>,
) -> Response {
    let _email = match get_session_email(&headers) {
        Some(e) => e,
        None => return (StatusCode::UNAUTHORIZED, "Unauthorized").into_response(),
    };

    match state.authenticator.delete_user_by_id(user_id).await {
        Ok(_) => {
            // Return empty response to remove row
            (StatusCode::OK, "").into_response()
        }
        Err(e) => {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response()
        }
    }
}

// ========== CHAT INTERFACE ==========

#[derive(Template)]
#[template(path = "chat_login.html")]
struct ChatLoginTemplate {
    error: String,
}

#[derive(Template)]
#[template(path = "chat_app.html")]
struct ChatAppTemplate {
    email: String,
}

// Helper to extract email from chat cookie
fn get_chat_session_email(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?;
    let cookies = cookie_header.to_str().ok()?;

    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", CHAT_SESSION_COOKIE)) {
            return Some(value.to_string());
        }
    }
    None
}

// Chat login page (GET)
pub async fn chat_login_page() -> impl IntoResponse {
    ChatLoginTemplate { error: String::new() }
}

// Chat login form submission (POST)
pub async fn chat_login_submit(
    State(state): State<Arc<AppState>>,
    Form(form): Form<LoginForm>,
) -> Response {
    match state.authenticator.authenticate(&form.email, &form.password).await {
        Ok(true) => {
            let cookie = format!("{}={}; Path=/; HttpOnly; SameSite=Lax", CHAT_SESSION_COOKIE, form.email);
            (
                StatusCode::SEE_OTHER,
                [(header::SET_COOKIE, cookie), (header::LOCATION, "/chat/app".to_string())],
                "Redirecting..."
            ).into_response()
        }
        _ => {
            ChatLoginTemplate {
                error: "Invalid email or password".to_string(),
            }.into_response()
        }
    }
}

// Chat logout
pub async fn chat_logout() -> impl IntoResponse {
    let cookie = format!("{}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0", CHAT_SESSION_COOKIE);
    (
        StatusCode::SEE_OTHER,
        [(header::SET_COOKIE, cookie), (header::LOCATION, "/chat/login".to_string())],
        "Redirecting..."
    ).into_response()
}

// Chat app (main interface)
pub async fn chat_app(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_chat_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/chat/login").into_response(),
    };

    ChatAppTemplate { email }.into_response()
}

// ========== ADMIN PAGES ==========

#[derive(Template)]
#[template(path = "dns.html")]
struct DnsTemplate {
    email: String,
}

#[derive(Template)]
#[template(path = "diagnostics.html")]
struct DiagnosticsTemplate {
    email: String,
}

#[derive(Template)]
#[template(path = "backups.html")]
struct BackupsTemplate {
    email: String,
}

#[derive(Template)]
#[template(path = "ssl.html")]
struct SslTemplate {
    email: String,
}

#[derive(Template)]
#[template(path = "settings.html")]
struct SettingsTemplate {
    email: String,
    version: String,
    total_users: i64,
}

#[derive(Template)]
#[template(path = "email_templates.html")]
struct TemplatesTemplate {
    email: String,
}

// DNS configuration page
pub async fn dns_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    DnsTemplate { email }.into_response()
}

// System diagnostics page
pub async fn diagnostics_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    DiagnosticsTemplate { email }.into_response()
}

// Backups management page
pub async fn backups_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    BackupsTemplate { email }.into_response()
}

// SSL certificates page
pub async fn ssl_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    SslTemplate { email }.into_response()
}

// Settings page
pub async fn settings_page(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    // Get user count and version
    let total_users = state.authenticator.count_users().await.unwrap_or(0);

    SettingsTemplate {
        email,
        version: env!("CARGO_PKG_VERSION").to_string(),
        total_users,
    }.into_response()
}

// Email templates page
pub async fn templates_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    TemplatesTemplate { email }.into_response()
}

#[derive(Template)]
#[template(path = "auto_reply.html")]
struct AutoReplyTemplate {
    email: String,
}

// Auto-reply configuration page
pub async fn auto_reply_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    AutoReplyTemplate { email }.into_response()
}

#[derive(Template)]
#[template(path = "greylisting.html")]
struct GreylistingTemplate {
    email: String,
}

// Greylisting management page
pub async fn greylisting_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    GreylistingTemplate { email }.into_response()
}

#[derive(Template)]
#[template(path = "quotas.html")]
struct QuotasTemplate {
    email: String,
}

// Quota management page
pub async fn quotas_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    QuotasTemplate { email }.into_response()
}

#[derive(Template)]
#[template(path = "security.html")]
struct SecurityTemplate {
    email: String,
}

// Security dashboard page
pub async fn security_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    SecurityTemplate { email }.into_response()
}

#[derive(Template)]
#[template(path = "monitoring.html")]
struct MonitoringTemplate {
    email: String,
}

// Server monitoring page
pub async fn monitoring_page(
    headers: axum::http::HeaderMap,
) -> Response {
    let email = match get_session_email(&headers) {
        Some(e) => e,
        None => return Redirect::to("/admin/login").into_response(),
    };

    MonitoringTemplate { email }.into_response()
}
