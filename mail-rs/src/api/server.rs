//! API Server - HTTP server for REST API

use axum::{
    extract::{ConnectInfo, FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use crate::api::{admin, auto_reply, templates, web};
use crate::api::auth::{Claims, JwtConfig};
use crate::api::handlers::{self, ApiError, AppState};
use crate::auto_reply::AutoReplyManager;
use crate::security::Authenticator;
use crate::templates::TemplateManager;
use sqlx::SqlitePool;

/// Rate limiter state for tracking requests per IP
pub struct RateLimiter {
    /// Map of IP -> (request count, window start time)
    requests: RwLock<HashMap<String, (u32, Instant)>>,
    /// Maximum requests per window
    max_requests: u32,
    /// Window duration
    window_duration: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(max_requests: u32, window_seconds: u64) -> Self {
        Self {
            requests: RwLock::new(HashMap::new()),
            max_requests,
            window_duration: Duration::from_secs(window_seconds),
        }
    }

    /// Check if request should be allowed for given IP
    pub async fn check_rate_limit(&self, ip: &str) -> bool {
        let now = Instant::now();
        let mut requests = self.requests.write().await;

        let entry = requests.entry(ip.to_string()).or_insert((0, now));

        // Reset if window has passed
        if now.duration_since(entry.1) > self.window_duration {
            entry.0 = 0;
            entry.1 = now;
        }

        // Check limit
        if entry.0 >= self.max_requests {
            return false;
        }

        // Increment counter
        entry.0 += 1;
        true
    }

    /// Clean up old entries (call periodically)
    pub async fn cleanup(&self) {
        let now = Instant::now();
        let mut requests = self.requests.write().await;
        requests.retain(|_, (_, start)| now.duration_since(*start) <= self.window_duration * 2);
    }
}

/// API Server configuration
pub struct ApiServer {
    state: Arc<AppState>,
    rate_limiter: Arc<RateLimiter>,
    template_manager: Arc<TemplateManager>,
    auto_reply_manager: Arc<AutoReplyManager>,
    addr: String,
}

impl ApiServer {
    /// Create a new API server
    pub async fn new(
        authenticator: Authenticator,
        jwt_secret: String,
        maildir_root: String,
        database_url: String,
        addr: String,
    ) -> Result<Self, sqlx::Error> {
        let state = Arc::new(AppState {
            authenticator,
            jwt_config: JwtConfig::new(jwt_secret, 24),
            maildir_root,
        });

        // Rate limiter: 100 requests per minute per IP
        let rate_limiter = Arc::new(RateLimiter::new(100, 60));

        // Create database connection pool
        let db = SqlitePool::connect(&database_url).await?;

        // Create template manager
        let template_manager = Arc::new(TemplateManager::new(db.clone()));
        template_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize templates table: {}", e))
        })?;

        // Create auto-reply manager
        let auto_reply_manager = Arc::new(AutoReplyManager::new(db));
        auto_reply_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize auto_reply tables: {}", e))
        })?;

        Ok(Self {
            state,
            rate_limiter,
            template_manager,
            auto_reply_manager,
            addr,
        })
    }

    /// Build the router with all routes
    pub fn router(&self) -> Router {
        // CORS configuration
        let cors = CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any);

        // Public routes (no auth required)
        let public_routes = Router::new()
            .route("/health", get(handlers::health))
            .route("/auth/login", post(handlers::login));

        // Protected routes (auth required)
        let protected_routes = Router::new()
            .route("/mails", get(handlers::list_emails))
            .route("/mails/:id", get(handlers::get_email))
            .route("/mails/send", post(handlers::send_email))
            .route("/folders", get(handlers::list_folders))
            .route_layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ));

        // Admin API routes (auth required + admin role check)
        use axum::routing::{delete, patch, put};
        let admin_api_routes = Router::new()
            .route("/users", get(admin::list_users))
            .route("/users/:id", get(admin::get_user))
            .route("/users", post(admin::create_user))
            .route("/users/:id", patch(admin::update_user))
            .route("/users/:id", delete(admin::delete_user))
            .route("/stats", get(admin::get_system_stats))
            .route("/config", get(admin::get_config))
            .route("/dns", get(admin::get_dns_config))
            .route("/diagnostics", get(admin::get_diagnostics))
            .route("/backups", get(admin::list_backups))
            .route("/backups", post(admin::create_backup))
            .route("/backups/:filename", delete(admin::delete_backup))
            .route("/backups/:filename/restore", post(admin::restore_backup))
            .route("/ssl", get(admin::get_ssl_status))
            .route("/ssl/request", post(admin::request_ssl_certificate))
            .route("/ssl/renew", post(admin::renew_ssl_certificate))
            .route_layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ));

        // Template API routes (session-based auth via cookies)
        let template_state = Arc::new(templates::TemplateState {
            template_manager: self.template_manager.clone(),
        });

        let template_api_routes = Router::new()
            .route("/templates", get(templates::list_templates))
            .route("/templates/category/:category", get(templates::list_templates_by_category))
            .route("/templates/:id", get(templates::get_template))
            .route("/templates", post(templates::create_template))
            .route("/templates/:id", put(templates::update_template))
            .route("/templates/:id", delete(templates::delete_template))
            .route("/templates/:id/render", post(templates::render_template))
            .route("/templates/signature/default", get(templates::get_default_signature))
            .with_state(template_state);

        // Auto-reply API routes (session-based auth via cookies)
        let auto_reply_state = Arc::new(auto_reply::AutoReplyState {
            manager: self.auto_reply_manager.clone(),
        });

        let auto_reply_api_routes = Router::new()
            .route("/auto-reply", get(auto_reply::get_auto_reply))
            .route("/auto-reply", post(auto_reply::create_auto_reply))
            .route("/auto-reply", put(auto_reply::update_auto_reply))
            .route("/auto-reply", delete(auto_reply::delete_auto_reply))
            .route("/auto-reply/toggle", post(auto_reply::toggle_auto_reply))
            .with_state(auto_reply_state);

        // Web routes (HTML pages)
        let web_state = Arc::new(web::AppState {
            authenticator: self.state.authenticator.clone(),
        });

        let web_routes = Router::new()
            .route("/admin/login", get(web::login_page))
            .route("/admin/login", post(web::login_submit))
            .route("/admin/logout", get(web::logout))
            .route("/admin/dashboard", get(web::dashboard))
            .route("/admin/users", get(web::users_page))
            .route("/admin/users", post(web::create_user))
            .route("/admin/users/:id", delete(web::delete_user))
            .route("/admin/templates", get(web::templates_page))
            .route("/admin/auto-reply", get(web::auto_reply_page))
            .route("/admin/dns", get(web::dns_page))
            .route("/admin/diagnostics", get(web::diagnostics_page))
            .route("/admin/backups", get(web::backups_page))
            .route("/admin/ssl", get(web::ssl_page))
            .route("/admin/settings", get(web::settings_page))
            .with_state(web_state.clone());

        // Chat routes (user-facing chatbot interface)
        let chat_routes = Router::new()
            .route("/chat/login", get(web::chat_login_page))
            .route("/chat/login", post(web::chat_login_submit))
            .route("/chat/logout", get(web::chat_logout))
            .route("/chat/app", get(web::chat_app))
            .with_state(web_state);

        // Combine all routes
        Router::new()
            .nest(
                "/api",
                public_routes
                    .merge(protected_routes)
                    .merge(template_api_routes)
                    .merge(auto_reply_api_routes),
            )
            .nest("/api/admin", admin_api_routes)
            .merge(web_routes)
            .merge(chat_routes)
            .layer(cors)
            .with_state(self.state.clone())
    }

    /// Start the API server
    pub async fn run(&self) -> std::io::Result<()> {
        let router = self.router();

        info!("Starting API server on {}", self.addr);

        let listener = tokio::net::TcpListener::bind(&self.addr).await?;
        axum::serve(listener, router).await?;

        Ok(())
    }
}

/// Authentication middleware - validates JWT token
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Extract Authorization header
    let auth_header = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok());

    let token = match auth_header {
        Some(h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            warn!("Missing or invalid Authorization header");
            return (
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new("Missing or invalid Authorization header")),
            )
                .into_response();
        }
    };

    // Validate token
    match state.jwt_config.validate_token(token) {
        Ok(claims) => {
            // Store claims in request extensions for handlers
            req.extensions_mut().insert(claims);
            next.run(req).await
        }
        Err(e) => {
            warn!("Invalid JWT token: {}", e);
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new("Invalid or expired token")),
            )
                .into_response()
        }
    }
}

/// Extract Claims from request (for handlers)
#[axum::async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<ApiError>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or((
                StatusCode::UNAUTHORIZED,
                Json(ApiError::new("Not authenticated")),
            ))
    }
}
