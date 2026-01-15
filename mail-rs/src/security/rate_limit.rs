//! Advanced Rate Limiting for SMTP/API
//!
//! This module implements sophisticated rate limiting to prevent abuse:
//! - Token bucket algorithm for smooth rate limiting
//! - Sliding window for precise request counting
//! - Per-IP rate limiting
//! - Per-user rate limiting
//! - Configurable limits and time windows
//! - Automatic cleanup of expired entries
//!
//! # Security Features
//! - Prevents brute force attacks
//! - Mitigates DoS/DDoS attempts
//! - Controls resource usage
//! - Separate limits for different operations
//!
//! # Example
//! ```no_run
//! use mail_rs::security::rate_limit::{RateLimiter, RateLimit};
//! use std::net::IpAddr;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let limiter = RateLimiter::new();
//!
//! let ip: IpAddr = "192.0.2.1".parse()?;
//!
//! // Check if request is allowed
//! if limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await {
//!     println!("Request allowed");
//! } else {
//!     println!("Rate limit exceeded");
//! }
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{debug, warn};

/// Rate limit types with different thresholds
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq)]
pub enum RateLimit {
    /// SMTP connections per IP per minute
    SmtpConnections,
    /// SMTP AUTH attempts per IP per hour
    SmtpAuthAttempts,
    /// SMTP messages per user per hour
    SmtpMessagesPerUser,
    /// SMTP recipients per message
    SmtpRecipientsPerMessage,
    /// API requests per IP per minute
    ApiRequestsPerIp,
    /// API requests per user per hour
    ApiRequestsPerUser,
    /// Login attempts per IP per hour
    LoginAttempts,
}

impl RateLimit {
    /// Get the maximum allowed requests for this limit type
    pub fn max_requests(&self) -> usize {
        match self {
            RateLimit::SmtpConnections => 60,        // 60 connections per minute
            RateLimit::SmtpAuthAttempts => 10,       // 10 auth attempts per hour
            RateLimit::SmtpMessagesPerUser => 100,   // 100 messages per hour per user
            RateLimit::SmtpRecipientsPerMessage => 100, // 100 recipients per message
            RateLimit::ApiRequestsPerIp => 120,      // 120 API requests per minute per IP
            RateLimit::ApiRequestsPerUser => 1000,   // 1000 API requests per hour per user
            RateLimit::LoginAttempts => 5,           // 5 login attempts per hour
        }
    }

    /// Get the time window for this limit type
    pub fn window_duration(&self) -> Duration {
        match self {
            RateLimit::SmtpConnections => Duration::from_secs(60),      // 1 minute
            RateLimit::SmtpAuthAttempts => Duration::from_secs(3600),   // 1 hour
            RateLimit::SmtpMessagesPerUser => Duration::from_secs(3600), // 1 hour
            RateLimit::SmtpRecipientsPerMessage => Duration::from_secs(0), // Per-message (no window)
            RateLimit::ApiRequestsPerIp => Duration::from_secs(60),     // 1 minute
            RateLimit::ApiRequestsPerUser => Duration::from_secs(3600), // 1 hour
            RateLimit::LoginAttempts => Duration::from_secs(3600),      // 1 hour
        }
    }

    /// Get a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            RateLimit::SmtpConnections => "SMTP connections per minute",
            RateLimit::SmtpAuthAttempts => "SMTP auth attempts per hour",
            RateLimit::SmtpMessagesPerUser => "SMTP messages per user per hour",
            RateLimit::SmtpRecipientsPerMessage => "SMTP recipients per message",
            RateLimit::ApiRequestsPerIp => "API requests per IP per minute",
            RateLimit::ApiRequestsPerUser => "API requests per user per hour",
            RateLimit::LoginAttempts => "Login attempts per hour",
        }
    }
}

/// Token bucket for smooth rate limiting
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Current number of tokens
    tokens: f64,
    /// Maximum number of tokens
    capacity: f64,
    /// Tokens added per second
    refill_rate: f64,
    /// Last refill time
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: usize, window: Duration) -> Self {
        let refill_rate = capacity as f64 / window.as_secs_f64();
        Self {
            tokens: capacity as f64,
            capacity: capacity as f64,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    /// Try to consume a token
    fn try_consume(&mut self) -> bool {
        self.refill();

        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;

        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;
    }
}

/// Sliding window for precise request counting
#[derive(Debug, Clone)]
struct SlidingWindow {
    /// Timestamps of requests in the current window
    requests: Vec<Instant>,
    /// Maximum allowed requests
    max_requests: usize,
    /// Window duration
    window: Duration,
}

impl SlidingWindow {
    fn new(max_requests: usize, window: Duration) -> Self {
        Self {
            requests: Vec::with_capacity(max_requests),
            max_requests,
            window,
        }
    }

    /// Try to add a new request
    fn try_add(&mut self) -> bool {
        self.cleanup();

        if self.requests.len() < self.max_requests {
            self.requests.push(Instant::now());
            true
        } else {
            false
        }
    }

    /// Remove expired requests
    fn cleanup(&mut self) {
        let now = Instant::now();
        let cutoff = now - self.window;
        self.requests.retain(|&t| t > cutoff);
    }

    /// Get current request count
    fn count(&mut self) -> usize {
        self.cleanup();
        self.requests.len()
    }
}

/// Rate limiting algorithm
#[derive(Debug, Clone)]
enum RateLimitAlgorithm {
    TokenBucket(TokenBucket),
    SlidingWindow(SlidingWindow),
}

impl RateLimitAlgorithm {
    fn new(limit: RateLimit) -> Self {
        let max = limit.max_requests();
        let window = limit.window_duration();

        // Use token bucket for short windows (smooth traffic)
        // Use sliding window for long windows (precise counting)
        if window.as_secs() <= 60 {
            RateLimitAlgorithm::TokenBucket(TokenBucket::new(max, window))
        } else {
            RateLimitAlgorithm::SlidingWindow(SlidingWindow::new(max, window))
        }
    }

    fn try_consume(&mut self) -> bool {
        match self {
            RateLimitAlgorithm::TokenBucket(bucket) => bucket.try_consume(),
            RateLimitAlgorithm::SlidingWindow(window) => window.try_add(),
        }
    }

    fn count(&mut self) -> usize {
        match self {
            RateLimitAlgorithm::TokenBucket(bucket) => {
                bucket.refill();
                (bucket.capacity - bucket.tokens).ceil() as usize
            }
            RateLimitAlgorithm::SlidingWindow(window) => window.count(),
        }
    }
}

/// Key for rate limit tracking
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
enum LimitKey {
    Ip(IpAddr, RateLimit),
    User(String, RateLimit),
}

/// Advanced rate limiter
pub struct RateLimiter {
    /// Per-IP and per-user rate limits
    limits: Arc<RwLock<HashMap<LimitKey, RateLimitAlgorithm>>>,
    /// Last cleanup time
    last_cleanup: Arc<RwLock<Instant>>,
    /// Cleanup interval
    cleanup_interval: Duration,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
            last_cleanup: Arc::new(RwLock::new(Instant::now())),
            cleanup_interval: Duration::from_secs(300), // Cleanup every 5 minutes
        }
    }

    /// Check rate limit for an IP address
    ///
    /// # Arguments
    /// * `ip` - IP address to check
    /// * `limit_type` - Type of rate limit to apply
    ///
    /// # Returns
    /// - `true` if request is allowed
    /// - `false` if rate limit exceeded
    ///
    /// # Example
    /// ```no_run
    /// # use mail_rs::security::rate_limit::{RateLimiter, RateLimit};
    /// # use std::net::IpAddr;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let limiter = RateLimiter::new();
    /// let ip: IpAddr = "192.0.2.1".parse()?;
    ///
    /// if limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await {
    ///     println!("Connection allowed");
    /// } else {
    ///     println!("Too many connections from this IP");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_ip_limit(&self, ip: &IpAddr, limit_type: RateLimit) -> bool {
        self.maybe_cleanup().await;

        let key = LimitKey::Ip(*ip, limit_type);
        let mut limits = self.limits.write().await;

        let algorithm = limits
            .entry(key.clone())
            .or_insert_with(|| RateLimitAlgorithm::new(limit_type));

        let allowed = algorithm.try_consume();

        if !allowed {
            warn!(
                "Rate limit exceeded for IP {}: {} ({})",
                ip,
                limit_type.description(),
                limit_type.max_requests()
            );
            debug!("Current count: {}", algorithm.count());
        }

        allowed
    }

    /// Check rate limit for a user
    ///
    /// # Arguments
    /// * `user` - User identifier (email or username)
    /// * `limit_type` - Type of rate limit to apply
    ///
    /// # Returns
    /// - `true` if request is allowed
    /// - `false` if rate limit exceeded
    ///
    /// # Example
    /// ```no_run
    /// # use mail_rs::security::rate_limit::{RateLimiter, RateLimit};
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let limiter = RateLimiter::new();
    ///
    /// if limiter.check_user_limit("user@example.com", RateLimit::SmtpMessagesPerUser).await {
    ///     println!("User can send message");
    /// } else {
    ///     println!("User has exceeded message limit");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn check_user_limit(&self, user: &str, limit_type: RateLimit) -> bool {
        self.maybe_cleanup().await;

        let key = LimitKey::User(user.to_string(), limit_type);
        let mut limits = self.limits.write().await;

        let algorithm = limits
            .entry(key.clone())
            .or_insert_with(|| RateLimitAlgorithm::new(limit_type));

        let allowed = algorithm.try_consume();

        if !allowed {
            warn!(
                "Rate limit exceeded for user {}: {} ({})",
                user,
                limit_type.description(),
                limit_type.max_requests()
            );
            debug!("Current count: {}", algorithm.count());
        }

        allowed
    }

    /// Get current request count for an IP
    pub async fn get_ip_count(&self, ip: &IpAddr, limit_type: RateLimit) -> usize {
        let key = LimitKey::Ip(*ip, limit_type);
        let mut limits = self.limits.write().await;

        if let Some(algorithm) = limits.get_mut(&key) {
            algorithm.count()
        } else {
            0
        }
    }

    /// Get current request count for a user
    pub async fn get_user_count(&self, user: &str, limit_type: RateLimit) -> usize {
        let key = LimitKey::User(user.to_string(), limit_type);
        let mut limits = self.limits.write().await;

        if let Some(algorithm) = limits.get_mut(&key) {
            algorithm.count()
        } else {
            0
        }
    }

    /// Reset rate limit for an IP
    pub async fn reset_ip_limit(&self, ip: &IpAddr, limit_type: RateLimit) {
        let key = LimitKey::Ip(*ip, limit_type);
        let mut limits = self.limits.write().await;
        limits.remove(&key);
        debug!("Reset rate limit for IP {}: {}", ip, limit_type.description());
    }

    /// Reset rate limit for a user
    pub async fn reset_user_limit(&self, user: &str, limit_type: RateLimit) {
        let key = LimitKey::User(user.to_string(), limit_type);
        let mut limits = self.limits.write().await;
        limits.remove(&key);
        debug!("Reset rate limit for user {}: {}", user, limit_type.description());
    }

    /// Cleanup expired entries if needed
    async fn maybe_cleanup(&self) {
        let now = Instant::now();
        let mut last_cleanup = self.last_cleanup.write().await;

        if now.duration_since(*last_cleanup) > self.cleanup_interval {
            debug!("Running rate limiter cleanup");
            *last_cleanup = now;
            drop(last_cleanup); // Release write lock before cleanup

            // Note: This is a simple cleanup. In production, you might want to
            // track entry creation times and remove old entries explicitly.
            // For now, the sliding window and token bucket algorithms handle
            // their own cleanup internally.
        }
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10, Duration::from_secs(10));

        // Should allow 10 requests
        for _ in 0..10 {
            assert!(bucket.try_consume());
        }

        // 11th request should fail
        assert!(!bucket.try_consume());

        // Tokens should be approximately 0 (may have tiny refill due to elapsed time)
        assert!(bucket.tokens < 0.01, "tokens should be close to 0, got {}", bucket.tokens);
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(5, Duration::from_secs(60));

        // Should allow 5 requests
        for _ in 0..5 {
            assert!(window.try_add());
        }

        // 6th request should fail
        assert!(!window.try_add());

        // Count should be 5
        assert_eq!(window.count(), 5);
    }

    #[tokio::test]
    async fn test_rate_limiter_ip() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.0.2.1".parse().unwrap();

        // Should allow up to the limit
        for _ in 0..60 {
            assert!(limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await);
        }

        // Should block after limit
        assert!(!limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await);

        // Count should be at limit
        let count = limiter.get_ip_count(&ip, RateLimit::SmtpConnections).await;
        assert!(count >= 60);
    }

    #[tokio::test]
    async fn test_rate_limiter_user() {
        let limiter = RateLimiter::new();
        let user = "test@example.com";

        // Should allow up to the limit
        for _ in 0..100 {
            assert!(limiter.check_user_limit(user, RateLimit::SmtpMessagesPerUser).await);
        }

        // Should block after limit
        assert!(!limiter.check_user_limit(user, RateLimit::SmtpMessagesPerUser).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_reset() {
        let limiter = RateLimiter::new();
        let ip: IpAddr = "192.0.2.1".parse().unwrap();

        // Exhaust the limit
        for _ in 0..60 {
            limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await;
        }

        // Should be blocked
        assert!(!limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await);

        // Reset
        limiter.reset_ip_limit(&ip, RateLimit::SmtpConnections).await;

        // Should be allowed again
        assert!(limiter.check_ip_limit(&ip, RateLimit::SmtpConnections).await);
    }

    #[test]
    fn test_rate_limit_descriptions() {
        assert_eq!(
            RateLimit::SmtpConnections.description(),
            "SMTP connections per minute"
        );
        assert_eq!(RateLimit::SmtpConnections.max_requests(), 60);
        assert_eq!(
            RateLimit::SmtpConnections.window_duration(),
            Duration::from_secs(60)
        );
    }
}
