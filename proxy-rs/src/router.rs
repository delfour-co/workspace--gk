//! Request router for proxy-rs
//!
//! Handles routing of incoming requests to backend services
//! based on host and path matching.

use crate::config::RouteConfig;
use crate::error::{ProxyError, Result};

/// Router for matching requests to backends
pub struct Router {
    routes: Vec<RouteConfig>,
}

/// Matched route with resolved backend
#[derive(Debug, Clone)]
pub struct MatchedRoute {
    /// Backend URL
    pub backend: String,
    /// Original path
    pub path: String,
    /// Path to forward (after stripping prefix if configured)
    pub forward_path: String,
    /// Request timeout
    pub timeout_seconds: u64,
}

impl Router {
    /// Create a new router with the given routes
    pub fn new(routes: Vec<RouteConfig>) -> Self {
        // Sort routes by path prefix length (longest first) for most specific matching
        let mut routes = routes;
        routes.sort_by(|a, b| b.path_prefix.len().cmp(&a.path_prefix.len()));
        Self { routes }
    }

    /// Find matching route for a request
    pub fn match_route(&self, host: &str, path: &str, default_timeout: u64) -> Result<MatchedRoute> {
        // Normalize host (remove port if present)
        let host = host.split(':').next().unwrap_or(host);

        for route in &self.routes {
            // Check host match (case-insensitive)
            if !route.host.eq_ignore_ascii_case(host) && route.host != "*" {
                continue;
            }

            // Check path prefix match
            if !path.starts_with(&route.path_prefix) {
                continue;
            }

            // Found a match
            let forward_path = if route.strip_prefix {
                let stripped = path.strip_prefix(&route.path_prefix).unwrap_or(path);
                if stripped.is_empty() || !stripped.starts_with('/') {
                    format!("/{}", stripped.trim_start_matches('/'))
                } else {
                    stripped.to_string()
                }
            } else {
                path.to_string()
            };

            return Ok(MatchedRoute {
                backend: route.backend.clone(),
                path: path.to_string(),
                forward_path,
                timeout_seconds: route.timeout_seconds.unwrap_or(default_timeout),
            });
        }

        Err(ProxyError::RouteNotFound {
            host: host.to_string(),
            path: path.to_string(),
        })
    }

    /// Get all configured routes
    pub fn routes(&self) -> &[RouteConfig] {
        &self.routes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_routes() -> Vec<RouteConfig> {
        vec![
            RouteConfig {
                host: "example.com".to_string(),
                path_prefix: "/api".to_string(),
                backend: "http://localhost:8080".to_string(),
                strip_prefix: false,
                health_check: None,
                timeout_seconds: None,
            },
            RouteConfig {
                host: "example.com".to_string(),
                path_prefix: "/".to_string(),
                backend: "http://localhost:3000".to_string(),
                strip_prefix: false,
                health_check: None,
                timeout_seconds: None,
            },
            RouteConfig {
                host: "*".to_string(),
                path_prefix: "/".to_string(),
                backend: "http://localhost:9000".to_string(),
                strip_prefix: false,
                health_check: None,
                timeout_seconds: None,
            },
        ]
    }

    #[test]
    fn test_match_api_route() {
        let router = Router::new(test_routes());
        let matched = router.match_route("example.com", "/api/users", 30).unwrap();
        assert_eq!(matched.backend, "http://localhost:8080");
        assert_eq!(matched.forward_path, "/api/users");
    }

    #[test]
    fn test_match_root_route() {
        let router = Router::new(test_routes());
        let matched = router.match_route("example.com", "/index.html", 30).unwrap();
        assert_eq!(matched.backend, "http://localhost:3000");
    }

    #[test]
    fn test_match_wildcard_host() {
        let router = Router::new(test_routes());
        let matched = router.match_route("other.com", "/anything", 30).unwrap();
        assert_eq!(matched.backend, "http://localhost:9000");
    }

    #[test]
    fn test_strip_prefix() {
        let routes = vec![RouteConfig {
            host: "example.com".to_string(),
            path_prefix: "/api/v1".to_string(),
            backend: "http://localhost:8080".to_string(),
            strip_prefix: true,
            health_check: None,
            timeout_seconds: None,
        }];

        let router = Router::new(routes);
        let matched = router.match_route("example.com", "/api/v1/users", 30).unwrap();
        assert_eq!(matched.forward_path, "/users");
    }
}
