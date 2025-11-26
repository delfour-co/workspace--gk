//! HTTP Reverse Proxy Server

use axum::{
    body::Body,
    extract::State,
    http::{Request, Response, StatusCode, Uri},
    response::IntoResponse,
    routing::{any, get},
    Router,
};
use hyper_util::{client::legacy::Client, rt::TokioExecutor};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tower_http::trace::TraceLayer;
use tracing::{debug, error, info, warn};

use crate::config::ProxyConfig;
use crate::error::{ProxyError, Result};
use crate::health::{HealthChecker, HealthStatus};
use crate::router::Router as ProxyRouter;
use crate::tls::TlsManager;

/// HTTP client type for forwarding requests
type HttpClient = Client<hyper_util::client::legacy::connect::HttpConnector, Body>;

/// Shared proxy state
pub struct ProxyState {
    /// Request router
    pub router: ProxyRouter,
    /// HTTP client for forwarding
    pub client: HttpClient,
    /// Default timeout
    pub default_timeout: u64,
    /// Health checker for backends
    pub health_checker: Arc<HealthChecker>,
}

/// Proxy server
pub struct ProxyServer {
    config: ProxyConfig,
    state: Arc<ProxyState>,
    tls_acceptor: Option<TlsAcceptor>,
}

impl ProxyServer {
    /// Create a new proxy server
    pub fn new(config: ProxyConfig) -> Result<Self> {
        config.validate()?;

        let router = ProxyRouter::new(config.routes.clone());

        // Create HTTP client
        let client: HttpClient = Client::builder(TokioExecutor::new())
            .pool_idle_timeout(Duration::from_secs(30))
            .build_http();

        // Create health checker
        let health_checker = Arc::new(HealthChecker::new());

        let state = Arc::new(ProxyState {
            router,
            client,
            default_timeout: config.server.timeout_seconds,
            health_checker,
        });

        // Setup TLS if configured
        let tls_acceptor = if let Some(ref tls_config) = config.tls {
            let tls_manager = TlsManager::new(tls_config.clone());
            Some(tls_manager.build_acceptor()?)
        } else {
            None
        };

        Ok(Self {
            config,
            state,
            tls_acceptor,
        })
    }

    /// Build the Axum router
    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(health_endpoint))
            .route("/ready", get(readiness_endpoint))
            .route("/*path", any(proxy_handler))
            .route("/", any(proxy_handler))
            .layer(TraceLayer::new_for_http())
            .with_state(self.state.clone())
    }

    /// Run the proxy server (HTTP mode)
    pub async fn run(&self) -> Result<()> {
        // Start background health checks
        let routes = self.config.routes.clone();
        self.state.health_checker.clone().start_background_checks(routes);

        let router = self.router();
        let addr = &self.config.server.listen_addr;

        info!("Starting proxy server on {}", addr);
        if self.tls_acceptor.is_some() {
            info!("TLS: enabled");
        } else {
            info!("TLS: disabled (HTTP only)");
        }

        info!("Configured routes:");
        for route in self.state.router.routes() {
            info!("  {} {} -> {}", route.host, route.path_prefix, route.backend);
        }

        let listener = TcpListener::bind(addr).await?;

        if let Some(ref _tls_acceptor) = self.tls_acceptor {
            // TLS mode - use axum_server with rustls
            info!("Starting HTTPS server");
            self.run_tls_server(listener).await
        } else {
            // Plain HTTP mode
            axum::serve(listener, router)
                .await
                .map_err(|e| ProxyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;
            Ok(())
        }
    }

    /// Run TLS server
    async fn run_tls_server(&self, listener: TcpListener) -> Result<()> {
        use hyper::service::service_fn;
        use tower::ServiceExt;

        let tls_acceptor = self.tls_acceptor.clone().unwrap();
        let router = self.router();

        loop {
            let (stream, addr) = listener.accept().await?;
            let acceptor = tls_acceptor.clone();
            let router = router.clone();

            tokio::spawn(async move {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let io = hyper_util::rt::TokioIo::new(tls_stream);

                        // Create a service from the router
                        let service = service_fn(move |req| {
                            let router = router.clone();
                            async move {
                                router.oneshot(req).await
                            }
                        });

                        if let Err(e) = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
                            .serve_connection(io, service)
                            .await
                        {
                            error!("Error serving connection from {}: {}", addr, e);
                        }
                    }
                    Err(e) => {
                        error!("TLS handshake failed from {}: {}", addr, e);
                    }
                }
            });
        }
    }

    /// Run HTTP to HTTPS redirect server
    pub async fn run_http_redirect(&self, http_addr: &str) -> Result<()> {
        let https_port = self
            .config
            .server
            .listen_addr
            .split(':')
            .last()
            .unwrap_or("443")
            .to_string();

        let redirect_router = Router::new()
            .fallback(move |req: Request<Body>| {
                let https_port = https_port.clone();
                async move {
                    let host = req
                        .headers()
                        .get("host")
                        .and_then(|h| h.to_str().ok())
                        .unwrap_or("localhost")
                        .split(':')
                        .next()
                        .unwrap_or("localhost")
                        .to_string();

                    let path = req.uri().path_and_query().map(|p| p.as_str().to_string()).unwrap_or_else(|| "/".to_string());
                    let redirect_url = format!("https://{}:{}{}", host, https_port, path);

                    Response::builder()
                        .status(StatusCode::MOVED_PERMANENTLY)
                        .header("Location", redirect_url)
                        .body(Body::empty())
                        .unwrap()
                }
            });

        info!("Starting HTTP redirect server on {}", http_addr);
        let listener = TcpListener::bind(http_addr).await?;
        axum::serve(listener, redirect_router)
            .await
            .map_err(|e| ProxyError::Io(std::io::Error::new(std::io::ErrorKind::Other, e)))?;

        Ok(())
    }
}

/// Health check endpoint
async fn health_endpoint() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Readiness endpoint - checks if backends are healthy
async fn readiness_endpoint(State(state): State<Arc<ProxyState>>) -> impl IntoResponse {
    let statuses = state.health_checker.all_statuses().await;

    let all_healthy = statuses.values().all(|r| r.status == HealthStatus::Healthy);
    let any_healthy = statuses.is_empty()
        || statuses.values().any(|r| {
            matches!(r.status, HealthStatus::Healthy | HealthStatus::Unknown)
        });

    if all_healthy || statuses.is_empty() {
        (StatusCode::OK, "Ready")
    } else if any_healthy {
        (StatusCode::OK, "Degraded")
    } else {
        (StatusCode::SERVICE_UNAVAILABLE, "Not Ready")
    }
}

/// Main proxy handler - forwards requests to backends
async fn proxy_handler(
    State(state): State<Arc<ProxyState>>,
    req: Request<Body>,
) -> impl IntoResponse {
    let host = req
        .headers()
        .get("host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    let path = req.uri().path();
    let query = req
        .uri()
        .query()
        .map(|q| format!("?{}", q))
        .unwrap_or_default();

    debug!("Proxying request: {} {} {}", req.method(), host, path);

    // Find matching route
    let matched = match state.router.match_route(host, path, state.default_timeout) {
        Ok(m) => m,
        Err(e) => {
            warn!("No route found for {}{}: {}", host, path, e);
            return (StatusCode::NOT_FOUND, "Not Found").into_response();
        }
    };

    // Check backend health
    let backend_status = state.health_checker.get_status(&matched.backend).await;
    if backend_status == HealthStatus::Unhealthy {
        warn!("Backend {} is unhealthy, returning 503", matched.backend);
        return (StatusCode::SERVICE_UNAVAILABLE, "Backend Unavailable").into_response();
    }

    // Build forwarding URL
    let forward_uri = format!("{}{}{}", matched.backend, matched.forward_path, query);
    debug!("Forwarding to: {}", forward_uri);

    // Parse the forwarding URI
    let uri: Uri = match forward_uri.parse() {
        Ok(u) => u,
        Err(e) => {
            error!("Invalid forward URI: {}", e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Internal Error").into_response();
        }
    };

    // Build the forwarded request
    let (mut parts, body) = req.into_parts();
    parts.uri = uri;

    // Remove hop-by-hop headers
    parts.headers.remove("host");
    parts.headers.remove("connection");
    parts.headers.remove("keep-alive");
    parts.headers.remove("proxy-authenticate");
    parts.headers.remove("proxy-authorization");
    parts.headers.remove("te");
    parts.headers.remove("trailers");
    parts.headers.remove("transfer-encoding");
    parts.headers.remove("upgrade");

    // Add X-Forwarded headers
    if let Some(client_ip) = parts.headers.get("x-real-ip").cloned() {
        parts.headers.insert("x-forwarded-for", client_ip);
    }

    let forward_req = Request::from_parts(parts, body);

    // Send request to backend
    match state.client.request(forward_req).await {
        Ok(response) => {
            let (parts, body) = response.into_parts();
            let body = Body::new(body);
            Response::from_parts(parts, body).into_response()
        }
        Err(e) => {
            error!("Backend error: {}", e);
            (StatusCode::BAD_GATEWAY, "Bad Gateway").into_response()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{RouteConfig, ServerConfig};

    fn test_config() -> ProxyConfig {
        ProxyConfig {
            server: ServerConfig {
                listen_addr: "127.0.0.1:0".to_string(),
                http_redirect_port: 80,
                timeout_seconds: 30,
            },
            tls: None,
            routes: vec![RouteConfig {
                host: "localhost".to_string(),
                path_prefix: "/".to_string(),
                backend: "http://127.0.0.1:8080".to_string(),
                strip_prefix: false,
                health_check: None,
                timeout_seconds: None,
            }],
        }
    }

    #[test]
    fn test_proxy_server_creation() {
        let config = test_config();
        let server = ProxyServer::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_proxy_server_router() {
        let config = test_config();
        let server = ProxyServer::new(config).unwrap();
        let _router = server.router();
        // Router builds successfully
    }
}
