# proxy-rs - Reverse Proxy Minimal en Rust

## Vue d'ensemble

`proxy-rs` est un reverse proxy HTTP/HTTPS léger écrit en Rust, conçu spécifiquement pour router le trafic vers les différents composants du système (mail API, web UI, chat, etc.) avec support SSL automatique via Let's Encrypt.

## Contexte du projet global

Point d'entrée unique pour tous les services. Remplace Traefik/Nginx avec une solution sur mesure, minimale et en Rust.

### Interfaces avec les autres composants

- **Frontend** : Expose ports 80/443 vers l'extérieur
- **Backend** : Route vers `mail-rs:8080`, `web-ui:3000`, `chat-rs:8081`, `dav-rs:8082`, etc.
- **Indépendant** : Pourrait être remplacé par Traefik/Caddy sans impact sur les autres composants

## Responsabilités

### Primaires
1. **Reverse proxy HTTP/HTTPS**
   - Termination TLS
   - Routing basé sur hostname/path
   - Load balancing basique (round-robin)

2. **SSL/TLS automatique**
   - Certificats Let's Encrypt (ACME protocol)
   - Renouvellement automatique
   - HTTP-01 challenge

3. **Sécurité**
   - Rate limiting global
   - Headers sécurité (HSTS, CSP, etc.)
   - IP whitelisting/blacklisting optionnel

4. **Observabilité**
   - Logs accès
   - Métriques (latency, throughput)
   - Health checks backends

### Secondaires
- Compression (gzip, brotli)
- Caching basique (optionnel)
- WebSocket proxying
- Redirections HTTP → HTTPS

## Architecture technique

### Stack Rust

```toml
[dependencies]
# HTTP server
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "cors", "compression-gzip", "trace"] }
hyper = { version = "1", features = ["full"] }
hyper-util = "0.1"

# TLS
tokio-rustls = "0.24"
rustls = "0.21"
rustls-pemfile = "1"

# ACME / Let's Encrypt
acme-lib = "0.9"
rcgen = "0.11"

# Config
serde = { version = "1", features = ["derive"] }
toml = "0.8"

# Async
tokio = { version = "1", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utils
url = "2"
bytes = "1"
http-body-util = "0.1"
```

### Structure du projet

```
proxy-rs/
├── Cargo.toml
├── config.example.toml
├── README.md
├── Dockerfile
│
├── src/
│   ├── main.rs              # Entry point
│   ├── config.rs            # Configuration loading
│   ├── error.rs             # Error types
│   │
│   ├── proxy/
│   │   ├── mod.rs
│   │   ├── server.rs        # Main HTTP server
│   │   ├── router.rs        # Route matching
│   │   ├── upstream.rs      # Backend connection pool
│   │   ├── middleware/      # Middleware stack
│   │   │   ├── mod.rs
│   │   │   ├── ratelimit.rs # Rate limiting
│   │   │   ├── headers.rs   # Security headers
│   │   │   └── logging.rs   # Access logs
│   │   └── handler.rs       # Request handler
│   │
│   ├── tls/
│   │   ├── mod.rs
│   │   ├── manager.rs       # Certificate management
│   │   ├── acme.rs          # Let's Encrypt ACME
│   │   └── store.rs         # Certificate storage
│   │
│   ├── health/
│   │   ├── mod.rs
│   │   └── checker.rs       # Backend health checks
│   │
│   └── metrics/
│       ├── mod.rs
│       └── prometheus.rs    # Metrics export
│
├── tests/
│   ├── integration_test.rs
│   └── fixtures/
│
└── docs/
    └── ARCHITECTURE.md
```

## Spécifications fonctionnelles

### 1. Routing

**Configuration routes**

```toml
# config.toml
[[routes]]
host = "mail.example.com"
backends = ["http://mail-rs:8080"]

[[routes]]
host = "app.example.com"
path_prefix = "/api"
backends = ["http://web-ui:3000"]

[[routes]]
host = "app.example.com"
path_prefix = "/"
backends = ["http://web-ui:3000"]

[[routes]]
host = "chat.example.com"
backends = ["http://chat-rs:8081"]
websocket = true

[[routes]]
host = "dav.example.com"
backends = ["http://dav-rs:8082"]
```

**Algorithme de routing**

```rust
fn match_route(req: &Request) -> Option<Backend> {
    let host = req.headers().get("Host")?;
    let path = req.uri().path();
    
    // 1. Match exact host + path prefix
    for route in routes {
        if route.host == host && path.starts_with(&route.path_prefix) {
            return route.select_backend(); // Round-robin
        }
    }
    
    // 2. Match host only
    for route in routes {
        if route.host == host {
            return route.select_backend();
        }
    }
    
    None // 404
}
```

### 2. TLS / ACME

**Certificats Let's Encrypt**

```rust
// Workflow ACME
async fn obtain_certificate(domain: &str) -> Result<Certificate> {
    // 1. Create ACME account (email)
    let account = acme::Account::create("admin@example.com").await?;
    
    // 2. Request certificate
    let order = account.new_order(domain).await?;
    
    // 3. Get HTTP-01 challenge
    let challenge = order.http01_challenge()?;
    
    // 4. Serve challenge at /.well-known/acme-challenge/{token}
    store_challenge(challenge.token(), challenge.key_authorization());
    
    // 5. Notify Let's Encrypt
    challenge.validate().await?;
    
    // 6. Download certificate
    let cert = order.finalize().await?;
    
    // 7. Store on disk
    save_certificate(domain, cert)?;
    
    Ok(cert)
}
```

**Challenge HTTP-01**

```
GET http://example.com/.well-known/acme-challenge/{token}
→ Réponse: {key_authorization}
```

Le proxy doit intercepter ces requêtes et servir la réponse avant routing.

**Renouvellement automatique**

```rust
// Background task
async fn auto_renew_certificates() {
    loop {
        tokio::time::sleep(Duration::from_hours(12)).await;
        
        for cert in list_certificates() {
            if cert.expires_in_days() < 30 {
                info!("Renewing certificate for {}", cert.domain);
                match renew_certificate(&cert.domain).await {
                    Ok(_) => info!("Certificate renewed"),
                    Err(e) => error!("Failed to renew: {}", e),
                }
            }
        }
    }
}
```

**Storage**

```
/etc/proxy/certs/
├── example.com/
│   ├── cert.pem
│   ├── key.pem
│   └── chain.pem
└── mail.example.com/
    ├── cert.pem
    ├── key.pem
    └── chain.pem
```

### 3. Rate Limiting

**Configuration**

```toml
[ratelimit]
enabled = true
requests_per_second = 100  # Global
burst = 200

# Per-IP limits
per_ip_requests_per_minute = 60
per_ip_burst = 10

# Whitelist IPs (no limits)
whitelist = ["127.0.0.1", "10.0.0.0/8"]
```

**Algorithme Token Bucket**

```rust
struct RateLimiter {
    capacity: u32,      // Max tokens
    tokens: AtomicU32,  // Current tokens
    refill_rate: u32,   // Tokens/second
    last_refill: Mutex<Instant>,
}

impl RateLimiter {
    async fn check(&self, cost: u32) -> bool {
        self.refill().await;
        
        loop {
            let current = self.tokens.load(Ordering::Relaxed);
            if current < cost {
                return false; // Rate limited
            }
            
            if self.tokens.compare_exchange(
                current, 
                current - cost,
                Ordering::Release,
                Ordering::Relaxed
            ).is_ok() {
                return true; // Allowed
            }
        }
    }
    
    async fn refill(&self) {
        let mut last = self.last_refill.lock().await;
        let now = Instant::now();
        let elapsed = now.duration_since(*last).as_secs();
        
        if elapsed > 0 {
            let tokens_to_add = elapsed * self.refill_rate;
            self.tokens.fetch_add(
                tokens_to_add.min(self.capacity),
                Ordering::Relaxed
            );
            *last = now;
        }
    }
}
```

**Réponse 429**

```http
HTTP/1.1 429 Too Many Requests
Retry-After: 60
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 0
X-RateLimit-Reset: 1640000000

{"error": "Rate limit exceeded"}
```

### 4. Security Headers

```rust
fn add_security_headers(response: &mut Response) {
    response.headers_mut().insert(
        "Strict-Transport-Security",
        "max-age=31536000; includeSubDomains".parse().unwrap()
    );
    
    response.headers_mut().insert(
        "X-Frame-Options",
        "DENY".parse().unwrap()
    );
    
    response.headers_mut().insert(
        "X-Content-Type-Options",
        "nosniff".parse().unwrap()
    );
    
    response.headers_mut().insert(
        "X-XSS-Protection",
        "1; mode=block".parse().unwrap()
    );
    
    response.headers_mut().insert(
        "Content-Security-Policy",
        "default-src 'self'".parse().unwrap()
    );
    
    response.headers_mut().insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap()
    );
}
```

### 5. Health Checks

**Configuration**

```toml
[health]
enabled = true
interval_seconds = 10
timeout_seconds = 5
unhealthy_threshold = 3  # Marquer down après 3 échecs
healthy_threshold = 2    # Marquer up après 2 succès

[[routes]]
host = "mail.example.com"
backends = ["http://mail-rs:8080"]
health_path = "/health"  # GET /health doit retourner 200
```

**Background checker**

```rust
async fn health_check_loop(backends: Arc<Vec<Backend>>) {
    loop {
        for backend in backends.iter() {
            match check_health(backend).await {
                Ok(_) => {
                    backend.consecutive_failures.store(0, Ordering::Relaxed);
                    if backend.consecutive_successes.fetch_add(1, Ordering::Relaxed) 
                        >= config.healthy_threshold {
                        backend.mark_healthy();
                    }
                }
                Err(e) => {
                    backend.consecutive_successes.store(0, Ordering::Relaxed);
                    if backend.consecutive_failures.fetch_add(1, Ordering::Relaxed)
                        >= config.unhealthy_threshold {
                        backend.mark_unhealthy();
                        warn!("Backend {} marked unhealthy", backend.url);
                    }
                }
            }
        }
        
        tokio::time::sleep(Duration::from_secs(config.interval)).await;
    }
}

async fn check_health(backend: &Backend) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(config.timeout))
        .build()?;
    
    let url = format!("{}{}", backend.url, backend.health_path);
    let resp = client.get(&url).send().await?;
    
    if resp.status().is_success() {
        Ok(())
    } else {
        Err(Error::HealthCheckFailed(resp.status()))
    }
}
```

### 6. Load Balancing

**Round-robin simple**

```rust
struct Backend {
    url: String,
    healthy: AtomicBool,
    consecutive_failures: AtomicU32,
    consecutive_successes: AtomicU32,
}

struct Route {
    host: String,
    path_prefix: String,
    backends: Vec<Backend>,
    current_index: AtomicUsize,
}

impl Route {
    fn select_backend(&self) -> Option<&Backend> {
        let healthy_backends: Vec<_> = self.backends.iter()
            .filter(|b| b.healthy.load(Ordering::Relaxed))
            .collect();
        
        if healthy_backends.is_empty() {
            return None; // All backends down
        }
        
        let idx = self.current_index.fetch_add(1, Ordering::Relaxed);
        Some(healthy_backends[idx % healthy_backends.len()])
    }
}
```

### 7. Configuration Complète

```toml
# config.toml

[server]
http_port = 80
https_port = 443
bind_addr = "0.0.0.0"

[tls]
enabled = true
acme_email = "admin@example.com"
acme_directory = "https://acme-v02.api.letsencrypt.org/directory"
# acme_directory = "https://acme-staging-v02.api.letsencrypt.org/directory"  # Staging
cert_storage = "/etc/proxy/certs"
auto_renew = true

[ratelimit]
enabled = true
global_requests_per_second = 100
global_burst = 200
per_ip_requests_per_minute = 60
per_ip_burst = 10
whitelist_ips = ["127.0.0.1"]

[security]
add_security_headers = true
hsts_max_age = 31536000
enable_compression = true  # gzip

[health]
enabled = true
interval_seconds = 10
timeout_seconds = 5
unhealthy_threshold = 3
healthy_threshold = 2

[logging]
level = "info"
format = "json"  # or "pretty"
access_log = true

[metrics]
enabled = true
prometheus_port = 9090

# Routes
[[routes]]
host = "mail.example.com"
backends = ["http://mail-rs:8080"]
health_path = "/health"

[[routes]]
host = "app.example.com"
path_prefix = "/api"
backends = ["http://web-ui:3000"]
health_path = "/health"
websocket = true

[[routes]]
host = "chat.example.com"
backends = ["http://chat-rs:8081"]
health_path = "/health"
websocket = true

[[routes]]
host = "dav.example.com"
backends = ["http://dav-rs:8082"]
health_path = "/health"
```

## Logging

**Access logs**

```json
{
  "timestamp": "2024-11-18T10:30:45Z",
  "method": "GET",
  "path": "/api/messages",
  "host": "mail.example.com",
  "status": 200,
  "duration_ms": 42,
  "client_ip": "203.0.113.42",
  "user_agent": "Mozilla/5.0...",
  "backend": "http://mail-rs:8080",
  "request_id": "req_abc123"
}
```

**Error logs**

```json
{
  "timestamp": "2024-11-18T10:31:12Z",
  "level": "ERROR",
  "message": "Backend connection failed",
  "backend": "http://mail-rs:8080",
  "error": "Connection refused",
  "request_id": "req_def456"
}
```

## Métriques Prometheus

```
# Requests
proxy_requests_total{method, status, route}
proxy_request_duration_seconds{method, route}

# Backends
proxy_backend_requests_total{backend, status}
proxy_backend_up{backend}
proxy_backend_response_time_seconds{backend}

# Rate limiting
proxy_ratelimit_rejected_total{reason}

# TLS
proxy_tls_certificate_expiry_seconds{domain}
```

## Tests

### Tests unitaires

```rust
#[tokio::test]
async fn test_route_matching() {
    let routes = load_test_routes();
    let req = Request::builder()
        .uri("https://mail.example.com/api/messages")
        .header("Host", "mail.example.com")
        .body(())
        .unwrap();
    
    let backend = match_route(&req, &routes);
    assert!(backend.is_some());
    assert_eq!(backend.unwrap().url, "http://mail-rs:8080");
}

#[tokio::test]
async fn test_rate_limiter() {
    let limiter = RateLimiter::new(10, 1); // 10 cap, 1/sec
    
    // Should allow 10 requests
    for _ in 0..10 {
        assert!(limiter.check(1).await);
    }
    
    // 11th should be rejected
    assert!(!limiter.check(1).await);
}
```

### Tests d'intégration

```rust
#[tokio::test]
async fn test_proxy_request_to_backend() {
    // Start mock backend
    let backend = MockServer::start().await;
    backend.mock_get("/test").with_status(200).create().await;
    
    // Start proxy with backend
    let proxy = start_test_proxy(&[backend.url()]).await;
    
    // Make request through proxy
    let client = reqwest::Client::new();
    let resp = client.get(format!("{}/test", proxy.url()))
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
}
```

## Déploiement

### Docker

```dockerfile
FROM rust:1.75-alpine AS builder
RUN apk add --no-cache musl-dev openssl-dev
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/proxy-rs /usr/local/bin/
EXPOSE 80 443 9090
CMD ["proxy-rs", "--config", "/etc/proxy/config.toml"]
```

### docker-compose.yml

```yaml
version: '3.8'
services:
  proxy:
    build: .
    ports:
      - "80:80"
      - "443:443"
      - "9090:9090"  # Metrics
    volumes:
      - ./config.toml:/etc/proxy/config.toml:ro
      - proxy-certs:/etc/proxy/certs
    environment:
      - RUST_LOG=info
    restart: unless-stopped
    depends_on:
      - mail-rs
      - web-ui
      - chat-rs

volumes:
  proxy-certs:
```

## Roadmap

### MVP (Phase 1) - 2 semaines
- [x] HTTP reverse proxy basique
- [x] Routing host + path
- [x] TLS avec certificats manuels
- [x] Rate limiting global
- [x] Logs accès

### Phase 2 - 2 semaines
- [ ] ACME / Let's Encrypt auto
- [ ] Health checks
- [ ] Load balancing round-robin
- [ ] Security headers
- [ ] Métriques Prometheus

### Post-MVP
- [ ] Weighted load balancing
- [ ] Sticky sessions
- [ ] Circuit breaker
- [ ] Request/response buffering
- [ ] Cache layer (optionnel)

## Métriques de succès

- ✅ Latency overhead <5ms (p95)
- ✅ Throughput >10k req/sec
- ✅ Uptime >99.9%
- ✅ Certificats renouvelés automatiquement
- ✅ Zero downtime reload config

## Notes d'implémentation

### Décisions architecturales

**Pourquoi pas Traefik/Caddy ?**
- Contrôle total du code (propriétaire)
- Minimaliste (pas de features inutiles)
- Rust = performance + sécurité
- Intégration native avec l'écosystème

**Pourquoi ACME HTTP-01 et pas DNS-01 ?**
- Plus simple (pas besoin API DNS provider)
- Suffisant pour usage standard
- Peut ajouter DNS-01 plus tard si wildcard certs nécessaires

**Pourquoi rate limiting in-memory ?**
- MVP = single instance
- Redis overkill pour commencer
- Peut migrer vers Redis si cluster needed

### Pièges à éviter
- ❌ Ne pas implémenter cache sophistiqué dans MVP
- ❌ Ne pas gérer WebSocket au début (ajouter phase 2)
- ❌ Ne pas optimiser prématurément (profiler d'abord)

## Références

- https://letsencrypt.org/docs/
- https://github.com/tokio-rs/axum/tree/main/examples
- https://github.com/cloudflare/pingora (inspiration architecture)
