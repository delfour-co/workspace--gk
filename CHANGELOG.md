# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Initial project structure with Rust workspace
- Complete documentation suite (150KB+ of technical specifications)
- Security policy and best practices documentation
- Contributing guidelines with code quality standards

#### mail-rs (SMTP Server)

**Sprint 1: SMTP Receiver**
- SMTP receiver with RFC 5321 compliance
- Comprehensive email validation:
  - Length limits (local: 64, domain: 255, total: 320 chars)
  - Control character rejection
  - Null byte injection prevention
  - IP literal support (IPv4/IPv6)
- Security features:
  - Command timeout (5 minutes) to prevent slowloris attacks
  - DATA timeout (10 minutes)
  - Message size limit (10MB configurable)
  - Recipient limit (100 per message)
  - Line length limit (1000 chars)
  - Error tracking (auto-disconnect after 10 errors)
- SMTP command parsing (HELO, EHLO, MAIL FROM, RCPT TO, DATA, QUIT, RSET, NOOP)
- Session state machine with proper validation
- Maildir storage backend with atomic writes

**Sprint 2: SMTP Sender**
- SMTP client for outgoing email delivery
- MX record lookup with DNS resolver
- Priority-based mail server selection
- Automatic fallback to A/AAAA records
- Queue system with SQLite persistence:
  - Automatic retry with exponential backoff (2min → 32min)
  - Maximum 5 retry attempts
  - Bounce handling for permanent failures
  - Status tracking (Pending/Sending/Sent/Failed/Bounced)
  - Batch processing (10 emails at a time)
- Multi-server fallback (tries all MX servers in order)
- Async queue worker with configurable intervals

**Sprint 3: TLS + Authentication**
- TLS/STARTTLS support:
  - TLS configuration loading from PEM files
  - Self-signed certificate generation for development
  - STARTTLS command parsing (full upgrade implementation in progress)
  - Certificate validation and error handling
- SMTP AUTH implementation:
  - PLAIN authentication mechanism (RFC 4616)
  - LOGIN authentication mechanism
  - Argon2 password hashing for secure storage
  - User management (add, authenticate, delete)
  - SQLite-based user database
  - Base64 credential encoding/decoding
  - AUTH capability advertisement in EHLO
  - Require TLS before AUTH (configurable)
  - Optional authentication requirement for sending
- Security module:
  - `security::auth` - Authentication mechanisms and user management
  - `security::tls` - TLS configuration and certificate handling
- Configuration enhancements:
  - `enable_tls`, `tls_cert_path`, `tls_key_path`
  - `enable_auth`, `auth_database_url`, `require_auth`

**CLI Tools**
- `mail-user` - User management utility:
  - Add users with password hashing
  - Delete users
  - List all users with creation and last login dates
  - Check if user exists
  - Configurable database path

**Testing** (67/67 tests passing)
- Unit tests for all modules
- Integration tests for SMTP receiver
- Integration tests for SMTP sender/queue
- Security test cases (injection, limits, timeouts)
- Auth mechanism tests (PLAIN, LOGIN decoding)
- TLS certificate generation and loading tests
- Doc-tests for all public APIs
- Full rustdoc documentation for public APIs
- Structured logging with tracing

### Security
- Input validation on all external inputs
- Resource limits to prevent DoS attacks
- Timeout protection against slowloris
- Error handling without information leakage
- Security audit documentation
- Threat model documentation

## [0.1.0] - 2024-11-18

### Added
- Initial project setup
- Git repository initialization
- Workspace structure
- Documentation organization

---

## Security Vulnerabilities

If you discover a security vulnerability, please follow our [Security Policy](docs/SECURITY.md).

## Notes

### Versioning Strategy
- **0.x.x**: Development phase, breaking changes possible
- **1.0.0**: First stable release after MVP validation
- **1.x.x**: Stable API, only backwards-compatible changes

### Release Process
1. Update CHANGELOG.md
2. Update version in Cargo.toml
3. Run full test suite
4. Build release: `cargo build --release`
5. Create git tag: `git tag -a v0.1.0 -m "Release v0.1.0"`
6. Push tag: `git push origin v0.1.0`
7. Create GitHub release with binaries
