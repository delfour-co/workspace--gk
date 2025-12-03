# GK - AI-Native Email Chat Interface

> Self-hosted email system with conversational AI interface - 100% Rust

## What is this?

A modern email system where you interact with your emails through natural conversation with an AI assistant, instead of clicking through traditional interfaces.

**Key Features**:
- 🤖 Conversational AI interface powered by local LLM (Ollama)
- 📧 Full SMTP/IMAP mail server with TLS support
- 🔌 Model Context Protocol (MCP) for AI-email integration
- 🎨 Clean web UI with real-time updates
- 🔒 Production-ready security (JWT auth, rate limiting, STARTTLS)
- ✅ Comprehensive test suite (20+ tests passing)

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      User Browser                        │
│              http://localhost:8080                       │
└──────────────────────┬──────────────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │    mail-rs     │  SMTP/IMAP Server + Web UI
              │  (Port 8080)   │  - SMTP: 2525
              │  (SMTP: 2525)  │  - Web API + Chat UI
              │  (IMAP: 1993)  │  - JWT Authentication
              └────────┬───────┘
                       │
                       │  WebSocket + HTTP
                       │
                       ▼
              ┌────────────────┐
              │  ai-runtime    │  LLM + MCP Orchestrator
              │  (Port 8888)   │  - Ollama LLM (llama3.1:8b)
              └────────┬───────┘  - Email summarization
                       │          - Streaming responses
                       │          - Real-time notifications
                       │
                       │  MCP Protocol
                       │
                       ▼
              ┌────────────────┐
              │mcp-mail-server │  Email Tools via MCP
              │  (Port 8090)   │  - send_email
              └────────────────┘  - list_emails
                                  - read_email
                                  - search_emails
                                  - mark_as_read
                                  - delete_email
                                  - get_email_count
```

## Status

| Component | Status | Description |
|-----------|--------|-------------|
| **mail-rs** | 🟢 Production Ready | Full SMTP/IMAP server with TLS, auth, web UI |
| **ai-runtime** | 🟢 Functional | LLM runtime with MCP orchestration |
| **mcp-mail-server** | 🟢 Functional | 7 MCP tools for email operations |
| **Tests** | ✅ 20/22 Passing | Comprehensive integration & unit tests |

### ✅ Completed Features

**mail-rs** (SMTP/IMAP Server):
- ✅ SMTP server with STARTTLS encryption
- ✅ SMTP AUTH (LOGIN, PLAIN)
- ✅ IMAP server (read-only mailbox)
- ✅ Maildir storage with atomic operations
- ✅ Rate limiting & security features
- ✅ Web UI with chat interface
- ✅ JWT authentication
- ✅ User management CLI

**ai-runtime** (AI Backend):
- ✅ Ollama LLM integration
- ✅ Streaming word-by-word responses
- ✅ Email summarization with persistence
- ✅ Real-time email notifications via WebSocket
- ✅ MCP protocol client
- ✅ Tool calling and orchestration

**mcp-mail-server** (Email Tools):
- ✅ send_email - Send emails via SMTP
- ✅ list_emails - List emails from maildir
- ✅ read_email - Read email content
- ✅ search_emails - Search in emails
- ✅ mark_as_read - Mark emails as read
- ✅ delete_email - Delete emails
- ✅ get_email_count - Count unread emails

**Testing**:
- ✅ 7 MCP integration tests
- ✅ 7 AI-runtime WebSocket tests
- ✅ 8 SMTP integration tests
- ✅ End-to-end test script

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup`)
- Ollama with llama3.1:8b model
- SQLite 3

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull LLM model
ollama pull llama3.1:8b
```

### Installation

```bash
# Clone repository
git clone https://github.com/yourusername/gk
cd gk

# Build all services
cargo build --release

# Create test user
cargo run --bin mail-user -- add admin@delfour.co password123
```

### Running Services

Open 3 terminals:

**Terminal 1 - Mail Server**:
```bash
cargo run --bin mail-rs -- --config mail-rs/config.toml
# Listening on:
# - Web UI: http://localhost:8080
# - SMTP: localhost:2525
# - IMAP: localhost:1993
```

**Terminal 2 - MCP Server**:
```bash
cd mcp-mail-server
cargo run
# MCP server listening on http://localhost:8090
```

**Terminal 3 - AI Runtime**:
```bash
cd ai-runtime
cargo run
# AI runtime listening on http://localhost:8888
# WebSocket endpoint: ws://localhost:8888/ws
```

### Using the Chat Interface

1. Open browser: http://localhost:8080
2. Login with credentials: `admin@delfour.co` / `password123`
3. Chat with AI:
   - "Liste mes emails"
   - "Résume mes emails non lus"
   - "Envoie un email à test@example.com"

## Testing

```bash
# Run all tests
cargo test

# Run specific test suites
cargo test --package mcp-mail-server --test integration_test
cargo test --package ai-runtime --test integration_test
cargo test --package mail-rs --test smtp_test

# Run end-to-end tests
./test_e2e.sh
```

**Test Results**:
- ✅ MCP Integration Tests: 7/7 passed
- ✅ AI-Runtime Tests: 7/7 passed
- ✅ SMTP Tests: 6/6 passed (2 ignored as flaky)

## Configuration

### mail-rs (mail-rs/config.toml)
```toml
[smtp]
listen_addr = "0.0.0.0:2525"
enable_tls = false
enable_auth = true
require_auth = false  # Set to true in production

[imap]
listen_addr = "0.0.0.0:1993"

[api]
listen_addr = "0.0.0.0:8080"
jwt_secret = "change-this-in-production"
```

### ai-runtime (ai-runtime/config.toml)
```toml
[llm]
provider = "ollama"
model = "llama3.1:8b"
base_url = "http://localhost:11434"

[mcp]
mail_server_url = "http://localhost:8090"
```

## Architecture Details

### Email Flow

1. **Incoming Email** (SMTP → Maildir):
   ```
   External SMTP → mail-rs:2525 → Maildir storage →
   Webhook → ai-runtime → Generate summary → Notify WebSocket clients
   ```

2. **Outgoing Email** (Chat → SMTP):
   ```
   User message → ai-runtime → LLM detects send intent →
   MCP tool call → mcp-mail-server → mail-rs SMTP → External server
   ```

3. **Email Query** (Chat → MCP):
   ```
   User query → ai-runtime → LLM generates tool calls →
   MCP protocol → mcp-mail-server → Read maildir → Return results
   ```

### Data Storage

- **Emails**: Maildir format in `mail-rs/data/maildir/{email}/`
- **Users**: SQLite in `mail-rs/data/users.db`
- **Email Queue**: SQLite in `mail-rs/data/queue.db`
- **Summaries**: SQLite in `ai-runtime/summaries.db`

## Tech Stack

- **Language**: 100% Rust
- **LLM**: Ollama (llama3.1:8b, qwen2.5:3b)
- **Databases**: SQLite
- **Frontend**: HTMX + Alpine.js + Tailwind CSS (server-rendered)
- **Protocols**: SMTP, IMAP, WebSocket, MCP (Model Context Protocol)
- **Security**: JWT, Argon2 password hashing, TLS/STARTTLS

## Documentation

- [📖 Project Status](PROJECT_STATUS.md) - Current implementation status
- [🚀 Quick Start Guide](docs/QUICK_START.md) - Detailed setup guide
- [🧪 Test Documentation](tests/README.md) - Test suite overview
- [📑 Full Documentation Index](docs/INDEX.md) - All specs

### Component Documentation

- [mail-rs README](mail-rs/README.md) - SMTP/IMAP server details
- [ai-runtime README](ai-runtime/README.md) - AI runtime details
- [mcp-mail-server](mcp-mail-server/) - MCP server implementation

## Development

```bash
# Format code
cargo fmt --all

# Run linter
cargo clippy --all-targets --all-features

# Build for release
cargo build --release

# Run with logging
RUST_LOG=debug cargo run --bin mail-rs
```

## Production Deployment

See [mail-rs/docs/DEPLOYMENT.md](mail-rs/docs/DEPLOYMENT.md) for production deployment guide including:
- TLS/SSL certificate setup
- Firewall configuration
- DNS/MX records
- Monitoring and logging
- Backup strategies

## Roadmap

### Phase 1: Core Email (✅ Complete)
- ✅ SMTP/IMAP server
- ✅ AI chat interface
- ✅ MCP integration
- ✅ Basic security
- ✅ Comprehensive tests

### Phase 2: Production Hardening (🚧 In Progress)
- ⏳ SPF/DKIM/DMARC validation
- ⏳ Advanced spam filtering
- ⏳ Email attachments support
- ⏳ Full IMAP implementation
- ⏳ Performance optimization

### Phase 3: Advanced Features (📋 Planned)
- 📋 Multi-user support with domains
- 📋 Calendar integration (CalDAV)
- 📋 Contacts (CardDAV)
- 📋 Real-time chat (WebSocket)
- 📋 Mobile app

## Contributing

Contributions welcome! This project is in active development.

1. Fork the repository
2. Create a feature branch
3. Make your changes with tests
4. Submit a pull request

## License

AGPL-3.0 - See [LICENSE](LICENSE) file

## Contact

For questions or issues, please open a GitHub issue or refer to the documentation in `docs/`.

---

**Current Phase**: Phase 1 Complete - Production-ready email system with AI chat interface
**Test Coverage**: 20/22 tests passing (91%)
**Status**: Functional and ready for testing/feedback
