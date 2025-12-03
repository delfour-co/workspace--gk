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

use crate::api::{admin, web};
use crate::api::auth::{Claims, JwtConfig};
use crate::api::handlers::{self, ApiError, AppState};
use crate::security::Authenticator;

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
    addr: String,
}

impl ApiServer {
    /// Create a new API server
    pub fn new(
        authenticator: Authenticator,
        jwt_secret: String,
        maildir_root: String,
        addr: String,
    ) -> Self {
        let state = Arc::new(AppState {
            authenticator,
            jwt_config: JwtConfig::new(jwt_secret, 24),
            maildir_root,
        });

        // Rate limiter: 100 requests per minute per IP
        let rate_limiter = Arc::new(RateLimiter::new(100, 60));

        Self { state, rate_limiter, addr }
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
        use axum::routing::{delete, patch};
        let admin_api_routes = Router::new()
            .route("/users", get(admin::list_users))
            .route("/users/:id", get(admin::get_user))
            .route("/users", post(admin::create_user))
            .route("/users/:id", patch(admin::update_user))
            .route("/users/:id", delete(admin::delete_user))
            .route("/stats", get(admin::get_system_stats))
            .route("/config", get(admin::get_config))
            .route_layer(middleware::from_fn_with_state(
                self.state.clone(),
                auth_middleware,
            ));

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
            .nest("/api", public_routes.merge(protected_routes))
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
