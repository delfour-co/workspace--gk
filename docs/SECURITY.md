# Security Policy and Best Practices

## Overview

This document outlines the security measures implemented in the GK Communication Suite and best practices for secure deployment.

## Security Features

### 1. Input Validation

#### Email Address Validation
- **RFC 5321 compliance**: All email addresses are validated according to RFC 5321
- **Length limits**:
  - Local part (before @): max 64 characters
  - Domain part (after @): max 255 characters
  - Total email: max 320 characters
- **Injection prevention**:
  - Null byte checking
  - Control character rejection
  - Domain label validation
- **IP literal support**: IPv4 and IPv6 address literals properly validated

**Location**: `mail-rs/src/utils/email.rs`

#### SMTP Command Validation
- **Line length limits**: Maximum 1000 characters per line (RFC 5321)
- **Command parsing**: Strict parsing prevents malformed commands
- **Error tracking**: Disconnects clients after 10 consecutive errors

**Location**: `mail-rs/src/smtp/session.rs`

### 2. Resource Limits

#### Message Size Limits
- **Default maximum**: 10 MB per message
- **Configurable**: Can be adjusted in `config.toml`
- **Enforcement**: Checked during DATA transmission
- **Memory protection**: Prevents memory exhaustion attacks

#### Recipient Limits
- **Maximum recipients**: 100 per message
- **Purpose**: Prevents spam and resource exhaustion
- **Configurable**: Can be adjusted via constant `MAX_RECIPIENTS`

#### Connection Limits
- **Command timeout**: 5 minutes per command
- **Data timeout**: 10 minutes for DATA transmission
- **Purpose**: Prevents slowloris attacks

**Location**: `mail-rs/src/smtp/session.rs`

### 3. Timeout Protection

#### Slowloris Attack Prevention
All network operations have timeouts:
- Command reading: 300 seconds
- DATA transmission: 600 seconds
- Prevents resource exhaustion from slow clients

#### Implementation
```rust
timeout(COMMAND_TIMEOUT, reader.read_line(&mut line)).await
```

### 4. Error Handling

#### Error Counting
- Tracks consecutive errors per session
- Disconnects after `MAX_ERRORS` (10)
- Prevents abuse and reconnaissance

#### Secure Error Messages
- Generic error messages to external clients
- Detailed logging for administrators
- No information leakage

### 5. Storage Security

#### Maildir Format
- One message per file
- Atomic operations (tmp → new)
- Crash-safe
- No database corruption

#### File Permissions
- Ensure proper permissions on maildir:
  ```bash
  chmod 700 /var/mail/maildir
  chown mail:mail /var/mail/maildir
  ```

### 6. Logging

#### Security-Relevant Logging
- Connection attempts with IP addresses
- Authentication failures
- Abnormal disconnections
- Resource limit violations
- Suspicious patterns

#### Log Levels
- `ERROR`: Security violations, failures
- `WARN`: Suspicious behavior, soft limits
- `INFO`: Normal operations, connections
- `DEBUG`: Detailed protocol information

**Example**:
```rust
warn!("Too many recipients: {}", self.to.len());
error!("Line too long: {} bytes", line.len());
```

## Deployment Best Practices

### 1. Network Security

#### Firewall Configuration
```bash
# Allow only necessary ports
ufw allow 25/tcp   # SMTP
ufw allow 587/tcp  # Submission
ufw allow 993/tcp  # IMAPS
ufw enable
```

#### Reverse Proxy (proxy-rs)
- Always use TLS termination
- Let's Encrypt for automatic certificates
- Rate limiting per IP
- DDoS protection

### 2. TLS Configuration

#### Required for Production
```toml
[smtp]
enable_tls = true
tls_cert_path = "/etc/mail-rs/cert.pem"
tls_key_path = "/etc/mail-rs/key.pem"
```

#### Certificate Management
- Use Let's Encrypt for automatic renewal
- Monitor certificate expiration
- Use strong TLS versions (TLS 1.2+)

### 3. Authentication

#### SMTP AUTH (Future)
- Require authentication for sending
- Use strong password hashing (Argon2)
- Rate limit authentication attempts
- Lock accounts after failures

### 4. DNS Security

#### SPF Records
```dns
yourdomain.com. IN TXT "v=spf1 mx ~all"
```

#### DKIM Signing
- Generate secure keys (2048-bit RSA minimum)
- Rotate keys annually
- Publish DNS records

#### DMARC Policy
```dns
_dmarc.yourdomain.com. IN TXT "v=DMARC1; p=quarantine; rua=mailto:postmaster@yourdomain.com"
```

### 5. Monitoring

#### Metrics to Track
- Connection rate per IP
- Message volume per sender
- Rejection rates
- Authentication failures
- Resource usage (CPU, memory, disk)

#### Alerting
Set up alerts for:
- Abnormally high connection rates
- Disk space < 10%
- Service unavailability
- Certificate expiration < 30 days

### 6. Rate Limiting

#### Per-IP Limits (Future)
- Connections per minute
- Messages per hour
- Recipients per message

#### Implementation
```rust
// Future: IP-based rate limiting
// Use token bucket algorithm
// Store state in Redis for multi-instance
```

### 7. Backups

#### Maildir Backups
```bash
# Daily backup
rsync -av /var/mail/maildir/ /backup/maildir/$(date +%Y%m%d)/

# Retention: 30 days
find /backup/maildir -type d -mtime +30 -exec rm -rf {} \;
```

#### Database Backups
```bash
# SQLite backup
sqlite3 mail.db ".backup /backup/mail-$(date +%Y%m%d).db"
```

### 8. Container Security

#### Docker Best Practices
```dockerfile
# Use non-root user
USER mail:mail

# Read-only root filesystem
RUN chmod 700 /var/mail

# Drop capabilities
--cap-drop=ALL
--cap-add=NET_BIND_SERVICE

# Resource limits
--memory=512m
--cpus=1.0
```

#### docker-compose
```yaml
security_opt:
  - no-new-privileges:true
cap_drop:
  - ALL
cap_add:
  - NET_BIND_SERVICE
```

## Vulnerability Reporting

### Responsible Disclosure
If you discover a security vulnerability:

1. **DO NOT** open a public GitHub issue
2. Email security@yourdomain.com
3. Include:
   - Description of vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (optional)

### Response Timeline
- Acknowledgment: 24 hours
- Assessment: 72 hours
- Fix: 7-14 days (depending on severity)
- Public disclosure: After fix is deployed

## Security Checklist

### Development
- [ ] Input validation on all user inputs
- [ ] Error handling doesn't leak information
- [ ] No hardcoded credentials
- [ ] Dependencies regularly updated
- [ ] Security tests passing
- [ ] Code review completed

### Deployment
- [ ] TLS enabled
- [ ] Firewall configured
- [ ] Non-root user
- [ ] Log rotation configured
- [ ] Monitoring enabled
- [ ] Backups automated
- [ ] DNS records configured (SPF, DKIM, DMARC)

### Operations
- [ ] Regular security updates
- [ ] Log review (weekly)
- [ ] Failed auth monitoring
- [ ] Certificate renewal automated
- [ ] Incident response plan documented

## Code Review Guidelines

### Security-Focused Review
When reviewing code, check for:

1. **Input Validation**
   - All external inputs validated?
   - Proper error handling?
   - Length limits enforced?

2. **Resource Management**
   - Timeouts on all I/O?
   - Memory limits enforced?
   - Connection limits?

3. **Error Handling**
   - Errors logged appropriately?
   - No information leakage?
   - Graceful degradation?

4. **Dependencies**
   - Known vulnerabilities?
   - Minimal dependencies?
   - Regularly updated?

### Example Review Checklist
```rust
// ❌ BAD: No length check
fn process_email(email: &str) {
    // Process without validation
}

// ✅ GOOD: Validation before processing
fn process_email(email: &str) -> Result<()> {
    validate_email(email)?;
    // Safe to process
}
```

## Testing Security

### Unit Tests
```bash
cargo test
```

### Integration Tests
```bash
cargo test --test integration
```

### Security Audit
```bash
# Check for vulnerabilities
cargo audit

# Check for unsafe code
cargo geiger
```

### Fuzzing (Future)
```bash
# Fuzz SMTP parser
cargo fuzz run smtp_parser
```

## References

- [RFC 5321 - SMTP](https://www.rfc-editor.org/rfc/rfc5321)
- [RFC 5322 - Internet Message Format](https://www.rfc-editor.org/rfc/rfc5322)
- [RFC 6376 - DKIM](https://www.rfc-editor.org/rfc/rfc6376)
- [RFC 7208 - SPF](https://www.rfc-editor.org/rfc/rfc7208)
- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)

## Updates

This document is updated as new security features are implemented or new threats are identified.

**Last Updated**: 2024-11-18
**Version**: 1.0
