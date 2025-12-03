use askama_axum::Template;
use axum::{
    extract::{Path, State},
    response::{IntoResponse, Redirect, Response},
    Form,
};
use axum::http::{StatusCode, header};
use serde::Deserialize;
use std::sync::Arc;

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

// Helper to extract email from cookie
fn get_session_email(headers: &axum::http::HeaderMap) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?;
    let cookies = cookie_header.to_str().ok()?;

    for cookie in cookies.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix(&format!("{}=", SESSION_COOKIE)) {
            return Some(value.to_string());
        }
    }
    None
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
