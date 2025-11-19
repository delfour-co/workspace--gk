# GK - AI-Native Communication Suite

> Self-hosted communication suite with conversational AI interface

## What is this?

An alternative to Google Workspace that you can self-host, with a unique twist: instead of clicking through multiple interfaces, you interact with an AI assistant that controls all services through natural conversation.

**Key differentiator**: AI-native UX powered by local LLM via the Model Context Protocol (MCP).

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Internet                             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │   proxy-rs     │  Reverse Proxy (SSL/TLS)
              │  (Port 80/443) │
              └────────┬───────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
        ▼              ▼              ▼
  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │ web-ui   │  │  mail-rs │  │ chat-rs  │
  │(Port 3000│  │(Port 8080│  │(Port 8081│
  └────┬─────┘  └────┬─────┘  └────┬─────┘
       │             │              │
       │    ┌────────┴────────┐     │
       │    │                 │     │
       ▼    ▼                 ▼     ▼
  ┌────────────────┐    ┌──────────────┐
  │  ai-runtime    │    │   dav-rs     │
  │  (Port 8888)   │    │ (Port 8082)  │
  └────────┬───────┘    └──────────────┘
           │
    ┌──────┴──────┐
    │             │
    ▼             ▼
┌─────────┐  ┌─────────┐
│mcp-mail │  │mcp-chat │  MCP Servers
│ (8090)  │  │ (8091)  │
└─────────┘  └─────────┘
```

## Components

| Component | Status | Description |
|-----------|--------|-------------|
| **mail-rs** | 🟢 SMTP Ready | SMTP/IMAP mail server (SMTP complete) |
| **proxy-rs** | ⏳ Planned | Reverse proxy with SSL/TLS |
| **ai-runtime** | ⏳ Planned | LLM runtime + MCP orchestrator |
| **mcp-mail-server** | ⏳ Planned | MCP server for mail |
| **web-ui** | ⏳ Planned | Conversational interface |
| **chat-rs** | ⏳ Planned | Real-time messaging |
| **dav-rs** | ⏳ Planned | CalDAV/CardDAV server |

## Current Status

### ✅ Completed (Sprint 1 & 2)
- Project structure and workspace setup
- Complete documentation (~153KB of technical specs)
- **mail-rs SMTP receiver (Sprint 1)**:
  - SMTP command parsing
  - Session state machine with security
  - Maildir storage (atomic writes)
  - TCP server with async/await
  - Input validation (RFC 5321)
  - Timeout protection & rate limits
  - 66/66 tests passing
- **mail-rs SMTP sender (Sprint 2)**:
  - SMTP client for outgoing emails
  - MX record lookup with DNS
  - Queue system with SQLite persistence
  - Automatic retry with exponential backoff
  - Bounce handling
  - Multi-server fallback

### 🚧 Next Up (Sprint 3)
- TLS/STARTTLS support
- SMTP AUTH (authentication)
- SPF/DKIM validation
- IMAP server implementation

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup`)
- Docker (optional, for deployment)

### Build & Run mail-rs

```bash
# Build
cd mail-rs
cargo build

# Copy example config
cp config.example.toml config.toml

# Run
cargo run
```

The SMTP server will listen on `0.0.0.0:2525` (non-privileged port for testing).

### Test with swaks

```bash
# Send test email
swaks --to test@localhost \
      --from sender@example.com \
      --server localhost:2525 \
      --body "Test email"

# Check received mail
ls -la /tmp/maildir/test@localhost/new/
```

## Documentation

- [📖 Project Overview](docs/README.md) - Architecture, roadmap, strategy
- [🚀 Quick Start Guide](docs/QUICK_START.md) - Step-by-step development guide
- [📑 Index](docs/INDEX.md) - All technical specifications

### Component Specs

Each component has detailed specifications in `docs/*_CLAUDE.md`:
- Architecture and design
- Rust dependencies (Cargo.toml)
- File structure
- Code examples
- Tests and deployment

## Tech Stack

- **Language**: 100% Rust (proprietary code)
- **LLM**: Mistral 7B or Llama 3.1 8B (quantized)
- **Databases**: SQLite (mail, ai, dav) + PostgreSQL (chat)
- **Frontend**: HTMX + Alpine.js + Tailwind CSS
- **Protocols**: SMTP, IMAP, WebSocket, CalDAV, CardDAV, MCP
- **Deployment**: Docker + docker-compose

## Timeline

- **MVP Minimal** (Mail + AI + UI): ~20 weeks (5 months)
- **MVP Complete** (+ Chat + DAV): ~26-30 weeks (6-7 months)

## License

AGPL-3.0

## Strategy

1. Build MVP
2. Release open source
3. Validate market (target: 100+ GitHub stars, 10+ production installs)
4. If successful → Monetization (hosted version, enterprise features, support)

## Contributing

This project is currently in early development. Contributions welcome once MVP is stable.

## Contact

For questions about architecture or specifications, refer to the relevant `docs/*_CLAUDE.md` files.

---

**Current Phase**: Phase 1 - Mail Server Implementation
**Sprint**: 1/6 - SMTP Receiver ✅
**Next Sprint**: SMTP Sender
