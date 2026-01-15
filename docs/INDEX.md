# Documentation Index

## Overview

This directory contains technical specifications for the GK self-hosted communication suite with AI-native interface.

## Documentation Structure

### Main Documentation

| File | Description |
|------|-------------|
| [README.md](README.md) | Architecture overview and project vision |
| [QUICK_START.md](QUICK_START.md) | Getting started guide |
| [SECURITY.md](SECURITY.md) | Security policies and guidelines |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines |
| [FEATURES_PROMPTS.md](FEATURES_PROMPTS.md) | Ready-to-use feature implementation prompts |

### Component Specifications (*_CLAUDE.md)

Each CLAUDE.md file contains complete technical specifications:
- Architecture and dependencies
- File structure
- Functional specifications
- Code examples
- Tests and deployment

| Component | File | Status |
|-----------|------|--------|
| **Mail Server** | [mail-rs_CLAUDE.md](mail-rs_CLAUDE.md) | Production Ready |
| **AI Runtime** | [ai-runtime_CLAUDE.md](ai-runtime_CLAUDE.md) | Functional |
| **MCP Mail Server** | [mcp-mail-server_CLAUDE.md](mcp-mail-server_CLAUDE.md) | Functional |
| **Reverse Proxy** | [proxy-rs_CLAUDE.md](proxy-rs_CLAUDE.md) | Planned |
| **Web UI** | [web-ui_CLAUDE.md](web-ui_CLAUDE.md) | Integrated in mail-rs |
| **Chat Server** | [chat-rs_CLAUDE.md](chat-rs_CLAUDE.md) | Planned |
| **DAV Server** | [dav-rs_CLAUDE.md](dav-rs_CLAUDE.md) | Planned |

## Project Status

- **Phase 1**: Core Email - Complete
- **Phase 2**: Production Hardening - Complete
- **Phase 3**: Extensions - In Progress

See [ROADMAP.md](../ROADMAP.md) for detailed roadmap.

## Quick Links

### For Development
1. Read [QUICK_START.md](QUICK_START.md)
2. Read component spec (e.g., [mail-rs_CLAUDE.md](mail-rs_CLAUDE.md))
3. Check [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines

### For Understanding Architecture
1. Read [README.md](README.md) for overview
2. Read [ai-runtime_CLAUDE.md](ai-runtime_CLAUDE.md) for AI integration
3. Read [mail-rs_CLAUDE.md](mail-rs_CLAUDE.md) for email foundation

## Useful Resources

### RFCs
- RFC 5321 (SMTP): https://www.rfc-editor.org/rfc/rfc5321
- RFC 3501 (IMAP): https://www.rfc-editor.org/rfc/rfc3501
- RFC 6376 (DKIM): https://www.rfc-editor.org/rfc/rfc6376
- RFC 7208 (SPF): https://www.rfc-editor.org/rfc/rfc7208

### MCP Protocol
- Documentation: https://modelcontextprotocol.io/

### Rust Resources
- Tokio: https://tokio.rs/
- Axum: https://github.com/tokio-rs/axum
