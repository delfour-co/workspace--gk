# /devops - DevOps Agent

Manage infrastructure and CI/CD.

## Usage
```
/devops [docker|ci|deploy|monitor]
```

## Instructions

You are the DevOps Agent. Manage infrastructure and deployment.

### DOCKER

#### Dockerfile Best Practices
```dockerfile
# Multi-stage build
FROM rust:1.75 AS builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/app /usr/local/bin/
USER 1000
EXPOSE 8080
CMD ["app"]
```

#### docker-compose.yml
```yaml
version: '3.8'
services:
  app:
    build: .
    ports:
      - "8080:8080"
    environment:
      - DATABASE_URL=postgres://...
    depends_on:
      - db
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### CI/CD (GitHub Actions)

#### Build and Test
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo fmt --all -- --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release
```

#### Release
```yaml
name: Release
on:
  push:
    tags: ['v*']
jobs:
  release:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: cargo build --release
      - uses: softprops/action-gh-release@v1
        with:
          files: target/release/app
```

### MONITORING

#### Health Check Endpoint
```rust
async fn health() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "version": env!("CARGO_PKG_VERSION"),
        "uptime": get_uptime()
    }))
}
```

#### Metrics
- Request count and latency
- Error rates
- Memory usage
- Connection pool stats

### DEPLOYMENT

#### Environment Configuration
```bash
# Required
DATABASE_URL=postgres://user:pass@host/db
SMTP_HOST=mail.example.com

# Optional with defaults
LOG_LEVEL=info
PORT=8080
```

## Output
- Docker configurations
- CI/CD workflows
- Deployment scripts
- Monitoring setup

## Exit Criteria
- [ ] Docker builds successfully
- [ ] CI/CD pipeline works
- [ ] Health checks configured
- [ ] Deployment documented
