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
- Comprehensive test suite:
  - Unit tests for all modules
  - Integration tests for end-to-end SMTP transactions
  - Security test cases (injection, limits, timeouts)
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
