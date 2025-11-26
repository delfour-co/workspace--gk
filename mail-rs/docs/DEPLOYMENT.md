# mail-rs Deployment Guide

This guide covers deploying mail-rs in production environments.

## Prerequisites

- Rust 1.70+ (for compilation)
- SQLite 3.x (for user database)
- A valid domain with proper DNS records
- TLS certificates (Let's Encrypt recommended)

## Building for Production

```bash
# Build optimized release binary
cargo build --release

# Binary will be at target/release/mail-rs
```

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `MAIL_DOMAIN` | Mail server domain | `localhost` |
| `SMTP_PORT` | SMTP server port | `2525` |
| `IMAP_PORT` | IMAP server port | `1993` |
| `API_PORT` | REST API port | `8080` |
| `MAILDIR_ROOT` | Maildir storage path | `./maildir` |
| `DB_PATH` | SQLite database path | `./mail.db` |
| `JWT_SECRET` | JWT signing secret | (required) |
| `TLS_CERT` | TLS certificate path | (optional) |
| `TLS_KEY` | TLS private key path | (optional) |
| `LOG_LEVEL` | Log level (trace/debug/info/warn/error) | `info` |

### Example Configuration File (config.toml)

```toml
[server]
domain = "mail.example.com"
hostname = "mail.example.com"

[smtp]
listen_addr = "0.0.0.0:25"
submission_port = 587

[imap]
listen_addr = "0.0.0.0:993"

[api]
listen_addr = "0.0.0.0:8080"
jwt_secret = "your-secret-key-here"

[storage]
maildir_path = "/var/mail"
database_path = "/var/lib/mail-rs/mail.db"

[tls]
cert_path = "/etc/letsencrypt/live/mail.example.com/fullchain.pem"
key_path = "/etc/letsencrypt/live/mail.example.com/privkey.pem"

[limits]
max_message_size = 26214400  # 25 MB
max_recipients = 100
rate_limit_per_minute = 100
```

## DNS Configuration

### Required DNS Records

```
; MX Record
example.com.    IN  MX  10 mail.example.com.

; A Record
mail.example.com.   IN  A   YOUR_SERVER_IP

; SPF Record
example.com.    IN  TXT "v=spf1 mx -all"

; DKIM Record (after generating DKIM keys)
selector._domainkey.example.com. IN TXT "v=DKIM1; k=rsa; p=YOUR_PUBLIC_KEY"

; DMARC Record
_dmarc.example.com. IN TXT "v=DMARC1; p=reject; rua=mailto:postmaster@example.com"

; Reverse DNS (PTR) - Configure with your hosting provider
YOUR_SERVER_IP  IN  PTR mail.example.com.
```

## Systemd Service

Create `/etc/systemd/system/mail-rs.service`:

```ini
[Unit]
Description=mail-rs Mail Server
After=network.target

[Service]
Type=simple
User=mail
Group=mail
WorkingDirectory=/opt/mail-rs
ExecStart=/opt/mail-rs/mail-rs
Restart=always
RestartSec=5
Environment=RUST_LOG=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/mail /var/lib/mail-rs
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

Enable and start:
```bash
sudo systemctl daemon-reload
sudo systemctl enable mail-rs
sudo systemctl start mail-rs
```

## Docker Deployment

### Dockerfile

```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mail-rs /usr/local/bin/
VOLUME ["/var/mail", "/var/lib/mail-rs"]
EXPOSE 25 587 993 8080
CMD ["mail-rs"]
```

### docker-compose.yml

```yaml
version: '3.8'
services:
  mail-rs:
    build: .
    ports:
      - "25:25"
      - "587:587"
      - "993:993"
      - "8080:8080"
    volumes:
      - maildir:/var/mail
      - data:/var/lib/mail-rs
      - ./config.toml:/etc/mail-rs/config.toml:ro
      - /etc/letsencrypt:/etc/letsencrypt:ro
    environment:
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  maildir:
  data:
```

## Monitoring

### Prometheus Metrics

The API server exposes metrics at `/metrics`:

```bash
curl http://localhost:8080/metrics
```

Available metrics:
- `mail_rs_http_requests_total` - Total HTTP requests
- `mail_rs_http_errors_total` - Total HTTP errors
- `mail_rs_active_connections` - Active connections
- `mail_rs_emails_sent_total` - Emails sent
- `mail_rs_emails_received_total` - Emails received
- `mail_rs_auth_attempts_total` - Auth attempts
- `mail_rs_auth_failures_total` - Auth failures
- `mail_rs_uptime_seconds` - Server uptime

### Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'mail-rs'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

## Security Checklist

- [ ] Change default JWT secret
- [ ] Configure TLS certificates
- [ ] Set up firewall rules
- [ ] Configure SPF, DKIM, DMARC records
- [ ] Enable rate limiting
- [ ] Set up log rotation
- [ ] Configure backup for maildir and database
- [ ] Set up monitoring alerts

## Firewall Configuration

```bash
# Allow SMTP (25, 587)
sudo ufw allow 25/tcp
sudo ufw allow 587/tcp

# Allow IMAP (993)
sudo ufw allow 993/tcp

# Allow API (internal only recommended)
# sudo ufw allow 8080/tcp
```

## Backup

### Maildir Backup
```bash
# Backup maildir
tar -czf maildir-backup-$(date +%Y%m%d).tar.gz /var/mail

# Backup database
sqlite3 /var/lib/mail-rs/mail.db ".backup /backup/mail-$(date +%Y%m%d).db"
```

## Troubleshooting

### Common Issues

1. **Connection refused on port 25**
   - Check if running as root or with proper capabilities
   - Verify firewall rules

2. **TLS handshake failed**
   - Check certificate paths and permissions
   - Verify certificate chain is complete

3. **Authentication failed**
   - Verify user exists: `mail-user list`
   - Check password hash in database

4. **Emails not delivered**
   - Check DNS records (SPF, DKIM, DMARC)
   - Verify reverse DNS is set up
   - Check mail logs for errors

### Logs

```bash
# View logs
journalctl -u mail-rs -f

# Debug logging
RUST_LOG=debug mail-rs
```
