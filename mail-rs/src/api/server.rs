//! API Server - HTTP server for REST API

use axum::{
    extract::{ConnectInfo, FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{delete, get, patch, post, put},
    Json, Router,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

use crate::api::{admin, auto_reply, caldav, greylisting, import_export, mfa, monitoring, quotas, search, security_stats, sieve, spam, templates, web};
use crate::api::auth::{Claims, JwtConfig};
use crate::api::handlers::{self, ApiError, AppState};
use crate::antispam::greylist::GreylistManager;
use crate::auto_reply::AutoReplyManager;
use crate::caldav::CalDavManager;
use crate::import_export::ImportExportManager;
use crate::mfa::MfaManager;
use crate::quota::manager::QuotaManager;
use crate::search::SearchManager;
use crate::security::Authenticator;
use crate::sieve::SieveManager;
use crate::spam::SpamManager;
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
    greylist_manager: Arc<GreylistManager>,
    quota_manager: Arc<QuotaManager>,
    security_stats_manager: Arc<security_stats::SecurityStatsManager>,
    monitoring_manager: Arc<monitoring::MonitoringManager>,
    mfa_manager: Arc<MfaManager>,
    sieve_manager: Arc<SieveManager>,
    search_manager: Arc<SearchManager>,
    spam_manager: Arc<SpamManager>,
    import_export_manager: Arc<ImportExportManager>,
    caldav_manager: Arc<CalDavManager>,
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
        let auto_reply_manager = Arc::new(AutoReplyManager::new(db.clone()));
        auto_reply_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize auto_reply tables: {}", e))
        })?;

        // Create greylist manager
        let greylist_manager = Arc::new(GreylistManager::new());

        // Create quota manager
        let quota_manager = Arc::new(QuotaManager::new());

        // Create security stats manager
        let security_stats_manager = Arc::new(security_stats::SecurityStatsManager::new());

        // Create monitoring manager
        let monitoring_manager = Arc::new(monitoring::MonitoringManager::new());

        // Create MFA manager
        let mfa_manager = Arc::new(MfaManager::new(db.clone()));
        mfa_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize MFA tables: {}", e))
        })?;

        // Create Sieve manager
        let sieve_manager = Arc::new(SieveManager::new(db.clone()));
        sieve_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize Sieve tables: {}", e))
        })?;

        // Create Search manager
        let search_manager = Arc::new(SearchManager::new());
        // Initialize search index (optional - may fail if path doesn't exist)
        if let Err(e) = search_manager.init().await {
            tracing::warn!("Failed to initialize search index: {} - search will be disabled", e);
        }

        // Create Spam manager
        let spam_manager = Arc::new(SpamManager::new(db));
        spam_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize spam tables: {}", e))
        })?;

        // Create Import/Export manager
        let export_path = std::path::PathBuf::from(&state.maildir_root).join("exports");
        let maildir_path = std::path::PathBuf::from(&state.maildir_root);
        let import_export_manager = Arc::new(ImportExportManager::new(export_path, maildir_path));
        import_export_manager.init().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize import/export: {}", e))
        })?;

        // Create CalDAV/CardDAV manager
        let caldav_db = SqlitePool::connect(&database_url).await?;
        let caldav_manager = Arc::new(CalDavManager::new(caldav_db));
        caldav_manager.init_db().await.map_err(|e| {
            sqlx::Error::Protocol(format!("Failed to initialize CalDAV tables: {}", e))
        })?;

        Ok(Self {
            state,
            rate_limiter,
            template_manager,
            auto_reply_manager,
            greylist_manager,
            quota_manager,
            security_stats_manager,
            monitoring_manager,
            mfa_manager,
            sieve_manager,
            search_manager,
            spam_manager,
            import_export_manager,
            caldav_manager,
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

        // Greylisting API routes (session-based auth via cookies)
        let greylist_state = Arc::new(greylisting::GreylistState {
            manager: self.greylist_manager.clone(),
        });

        let greylisting_api_routes = Router::new()
            .route("/admin/greylisting/stats", get(greylisting::get_stats))
            .route("/admin/greylisting/entries", get(greylisting::list_entries))
            .route("/admin/greylisting/whitelist", get(greylisting::list_whitelist))
            .route("/admin/greylisting/whitelist", post(greylisting::add_to_whitelist))
            .route("/admin/greylisting/whitelist/:pattern", delete(greylisting::remove_from_whitelist))
            .route("/admin/greylisting/blacklist", get(greylisting::list_blacklist))
            .route("/admin/greylisting/blacklist", post(greylisting::add_to_blacklist))
            .route("/admin/greylisting/blacklist/:pattern", delete(greylisting::remove_from_blacklist))
            .route("/admin/greylisting/cleanup", post(greylisting::cleanup_entries))
            .with_state(greylist_state);

        // Quotas API routes (session-based auth via cookies)
        let quota_state = Arc::new(quotas::QuotaState {
            manager: self.quota_manager.clone(),
        });

        let quotas_api_routes = Router::new()
            .route("/admin/quotas/stats", get(quotas::get_stats))
            .route("/admin/quotas", get(quotas::list_quotas))
            .route("/admin/quotas/defaults", get(quotas::get_defaults))
            .route("/admin/quotas/defaults", put(quotas::update_defaults))
            .route("/admin/quotas/reset-daily", post(quotas::reset_daily_counts))
            .route("/admin/quotas/:email", get(quotas::get_quota))
            .route("/admin/quotas/:email", put(quotas::update_quota))
            .with_state(quota_state);

        // Security stats API routes (session-based auth via cookies)
        let security_state = Arc::new(security_stats::SecurityStatsState {
            manager: self.security_stats_manager.clone(),
        });

        let security_api_routes = Router::new()
            .route("/admin/security/stats", get(security_stats::get_stats))
            .route("/admin/security/events", get(security_stats::get_events))
            .route("/admin/security/config", put(security_stats::update_config))
            .with_state(security_state);

        // Monitoring API routes (session-based auth via cookies)
        let monitoring_state = Arc::new(monitoring::MonitoringState {
            manager: self.monitoring_manager.clone(),
        });

        let monitoring_api_routes = Router::new()
            .route("/admin/monitoring/stats", get(monitoring::get_stats))
            .route("/admin/monitoring/logs", get(monitoring::get_logs))
            .route("/admin/monitoring/smtp/start", post(monitoring::start_smtp))
            .route("/admin/monitoring/smtp/stop", post(monitoring::stop_smtp))
            .route("/admin/monitoring/imap/start", post(monitoring::start_imap))
            .route("/admin/monitoring/imap/stop", post(monitoring::stop_imap))
            .with_state(monitoring_state);

        // MFA API routes (session-based auth via cookies)
        let mfa_state = Arc::new(mfa::MfaState {
            manager: self.mfa_manager.clone(),
        });

        let mfa_api_routes = Router::new()
            .route("/mfa/status", get(mfa::get_status))
            .route("/mfa/setup", post(mfa::start_setup))
            .route("/mfa/verify", post(mfa::complete_setup))
            .route("/mfa/disable", post(mfa::disable))
            .route("/mfa/backup-codes", post(mfa::regenerate_backup_codes))
            .route("/mfa/check", post(mfa::verify_login))
            .route("/mfa/required/:email", get(mfa::is_mfa_required))
            .with_state(mfa_state);

        // Sieve API routes (session-based auth via cookies)
        let sieve_state = Arc::new(sieve::SieveState {
            manager: self.sieve_manager.clone(),
        });

        let sieve_api_routes = Router::new()
            .route("/sieve/scripts", get(sieve::list_scripts))
            .route("/sieve/scripts", post(sieve::create_script))
            .route("/sieve/scripts/active", get(sieve::get_active_script))
            .route("/sieve/scripts/:id", get(sieve::get_script))
            .route("/sieve/scripts/:id", put(sieve::update_script))
            .route("/sieve/scripts/:id", delete(sieve::delete_script))
            .route("/sieve/scripts/:id/activate", post(sieve::activate_script))
            .route("/sieve/scripts/:id/deactivate", post(sieve::deactivate_script))
            .route("/sieve/validate", post(sieve::validate_script))
            .route("/sieve/logs", get(sieve::get_logs))
            .route("/sieve/logs", delete(sieve::clear_logs))
            .with_state(sieve_state);

        // Search API routes (session-based auth via cookies)
        let search_state = Arc::new(search::SearchState {
            search_manager: self.search_manager.clone(),
        });

        let search_api_routes = Router::new()
            .route("/search", get(search::search_emails))
            .route("/search/status", get(search::get_index_status))
            .route("/search/reindex", post(search::reindex))
            .route("/search/reindex-all", post(search::reindex_all))
            .route("/search/clear", delete(search::clear_index))
            .with_state(search_state);

        // Spam API routes (session-based auth via cookies)
        let spam_state = Arc::new(spam::SpamState {
            spam_manager: self.spam_manager.clone(),
        });

        let spam_api_routes = Router::new()
            .route("/spam/config", get(spam::get_config))
            .route("/spam/config", put(spam::update_config))
            .route("/spam/stats", get(spam::get_stats))
            .route("/spam/rules", get(spam::list_rules))
            .route("/spam/rules", post(spam::create_rule))
            .route("/spam/rules/:id", put(spam::update_rule))
            .route("/spam/rules/:id", delete(spam::delete_rule))
            .route("/spam/test", post(spam::test_message))
            .route("/spam/learn/spam", post(spam::learn_spam))
            .route("/spam/learn/ham", post(spam::learn_ham))
            .route("/spam/logs", get(spam::get_logs))
            .route("/spam/logs", delete(spam::clear_logs))
            .with_state(spam_state);

        // Import/Export API routes (session-based auth via cookies)
        let import_export_state = Arc::new(import_export::ImportExportState {
            manager: self.import_export_manager.clone(),
        });

        let import_export_api_routes = Router::new()
            .route("/import-export/stats", get(import_export::get_stats))
            .route("/import-export/export", post(import_export::start_export))
            .route("/import-export/export/:job_id", get(import_export::get_export_job))
            .route("/import-export/export/:job_id/download", get(import_export::download_export))
            .route("/import-export/export/:job_id", delete(import_export::delete_export))
            .route("/import-export/exports", get(import_export::list_export_jobs))
            .route("/import-export/import", post(import_export::start_import))
            .route("/import-export/import/:job_id", get(import_export::get_import_job))
            .route("/import-export/imports", get(import_export::list_import_jobs))
            .route("/import-export/:job_type/:job_id/cancel", post(import_export::cancel_job))
            .with_state(import_export_state);

        // CalDAV/CardDAV API routes (session-based auth via cookies)
        let caldav_state = Arc::new(caldav::CalDavState {
            manager: self.caldav_manager.clone(),
        });

        let caldav_api_routes = Router::new()
            .route("/caldav/stats", get(caldav::get_stats))
            // Calendars
            .route("/caldav/calendars", get(caldav::list_calendars))
            .route("/caldav/calendars", post(caldav::create_calendar))
            .route("/caldav/calendars/:calendar_id", get(caldav::get_calendar))
            .route("/caldav/calendars/:calendar_id", put(caldav::update_calendar))
            .route("/caldav/calendars/:calendar_id", delete(caldav::delete_calendar))
            // Events
            .route("/caldav/calendars/:calendar_id/events", get(caldav::list_events))
            .route("/caldav/calendars/:calendar_id/events", post(caldav::create_event))
            .route("/caldav/calendars/:calendar_id/import", post(caldav::import_ics))
            .route("/caldav/events/:event_id", get(caldav::get_event))
            .route("/caldav/events/:event_id", put(caldav::update_event))
            .route("/caldav/events/:event_id", delete(caldav::delete_event))
            // Address Books
            .route("/caldav/addressbooks", get(caldav::list_addressbooks))
            .route("/caldav/addressbooks", post(caldav::create_addressbook))
            .route("/caldav/addressbooks/:addressbook_id", get(caldav::get_addressbook))
            .route("/caldav/addressbooks/:addressbook_id", delete(caldav::delete_addressbook))
            // Contacts
            .route("/caldav/addressbooks/:addressbook_id/contacts", get(caldav::list_contacts))
            .route("/caldav/addressbooks/:addressbook_id/contacts", post(caldav::create_contact))
            .route("/caldav/addressbooks/:addressbook_id/import", post(caldav::import_vcf))
            .route("/caldav/contacts/:contact_id", get(caldav::get_contact))
            .route("/caldav/contacts/:contact_id", put(caldav::update_contact))
            .route("/caldav/contacts/:contact_id", delete(caldav::delete_contact))
            .with_state(caldav_state);

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
            .route("/admin/greylisting", get(web::greylisting_page))
            .route("/admin/quotas", get(web::quotas_page))
            .route("/admin/security", get(web::security_page))
            .route("/admin/monitoring", get(web::monitoring_page))
            .route("/admin/mfa", get(web::mfa_page))
            .route("/admin/sieve", get(web::sieve_page))
            .route("/admin/search", get(web::search_page))
            .route("/admin/spam", get(web::spam_page))
            .route("/admin/import-export", get(web::import_export_page))
            .route("/admin/caldav", get(web::caldav_page))
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
                    .merge(auto_reply_api_routes)
                    .merge(greylisting_api_routes)
                    .merge(quotas_api_routes)
                    .merge(security_api_routes)
                    .merge(monitoring_api_routes)
                    .merge(mfa_api_routes)
                    .merge(sieve_api_routes)
                    .merge(search_api_routes)
                    .merge(spam_api_routes)
                    .merge(import_export_api_routes)
                    .merge(caldav_api_routes),
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
