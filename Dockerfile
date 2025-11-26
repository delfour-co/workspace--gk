# =============================================================================
# GK Mail - Multi-stage Dockerfile for Rust services
# =============================================================================

# -----------------------------------------------------------------------------
# Stage 1: Build all Rust binaries
# -----------------------------------------------------------------------------
FROM rust:1.85-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY mail-rs ./mail-rs
COPY mcp-mail-server ./mcp-mail-server
COPY ai-runtime ./ai-runtime
COPY proxy-rs ./proxy-rs
COPY e2e-tests ./e2e-tests

# Build all binaries in release mode
RUN cargo build --release -p mail-rs -p mcp-mail-server -p ai-runtime -p proxy-rs

# -----------------------------------------------------------------------------
# Stage 2: Runtime image for mail-rs
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS mail-rs

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libsqlite3-0 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binaries
COPY --from=builder /app/target/release/mail-rs /app/mail-rs
COPY --from=builder /app/target/release/mail-user /app/mail-user
COPY --from=builder /app/target/release/add-user /app/add-user

# Create data directories
RUN mkdir -p /data/maildir /data/queue /data/db

# Environment variables
ENV RUST_LOG=info
ENV SMTP_ADDR=0.0.0.0:2525
ENV IMAP_ADDR=0.0.0.0:1993
ENV MAILDIR_PATH=/data/maildir
ENV QUEUE_PATH=/data/queue
ENV DATABASE_URL=sqlite:///data/db/users.db
ENV DOMAIN=localhost

EXPOSE 2525 1993

CMD ["/app/mail-rs"]

# -----------------------------------------------------------------------------
# Stage 3: Runtime image for mcp-mail-server
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS mcp-mail-server

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/mcp-mail-server /app/mcp-mail-server

ENV RUST_LOG=info
ENV MCP_PORT=8090
ENV MAILDIR_PATH=/data/maildir
ENV SMTP_HOST=mail-rs
ENV SMTP_PORT=2525

EXPOSE 8090

CMD ["/app/mcp-mail-server"]

# -----------------------------------------------------------------------------
# Stage 4: Runtime image for ai-runtime
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS ai-runtime

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/ai-runtime /app/ai-runtime

ENV RUST_LOG=info
ENV AI_PORT=8888
ENV MCP_URL=http://mcp-mail-server:8090
ENV OLLAMA_URL=http://ollama:11434
ENV OLLAMA_MODEL=llama3.1:8b

EXPOSE 8888

CMD ["/app/ai-runtime"]

# -----------------------------------------------------------------------------
# Stage 5: Runtime image for proxy-rs
# -----------------------------------------------------------------------------
FROM debian:bookworm-slim AS proxy-rs

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/proxy-rs /app/proxy-rs

# Create directories for certs and config
RUN mkdir -p /data/certs /app/config

ENV RUST_LOG=info
ENV HTTP_PORT=80
ENV HTTPS_PORT=443
ENV CERT_DIR=/data/certs

EXPOSE 80 443

CMD ["/app/proxy-rs"]
