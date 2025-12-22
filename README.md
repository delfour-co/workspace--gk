# GK - AI-Native Email Chat Interface

> Self-hosted email system with conversational AI interface - 100% Rust

## What is this?

A modern email system where you interact with your emails through natural conversation with an AI assistant, instead of clicking through traditional interfaces.

**Key Features**:
- 🤖 Conversational AI interface powered by local LLM (Ollama)
- 📧 Full SMTP/IMAP mail server with TLS support
- 🛡️ Complete Mail-in-a-Box administration tools (SPF/DKIM/DMARC, backups, SSL, diagnostics)
- 🎨 Modern web admin panel with dark mode support
- 🔌 Model Context Protocol (MCP) for AI-email integration
- 🔒 Production-ready security (JWT auth, rate limiting, STARTTLS, greylisting)
- ✅ Comprehensive test suite (175+ tests passing)

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
| **mail-rs** | 🟢 Production Ready | Full SMTP/IMAP server with SPF/DKIM/DMARC, admin panel |
| **ai-runtime** | 🟢 Functional | LLM runtime with MCP orchestration |
| **mcp-mail-server** | 🟢 Functional | 7 MCP tools for email operations |
| **Admin Panel** | 🟢 Complete | DNS config, diagnostics, backups, SSL, settings |
| **Tests** | ✅ 175+ Passing | Comprehensive integration & unit tests |

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
- ✅ SPF/DKIM/DMARC validation and signing
- ✅ MIME parser with attachment support
- ✅ Quota management (storage and message limits)
- ✅ Greylisting anti-spam system
- ✅ Mail-in-a-Box administration tools

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

**Admin Panel** (Web Interface):
- ✅ DNS Configuration - Display all DNS records (A, MX, SPF, DKIM, DMARC)
- ✅ System Diagnostics - Real-time health monitoring
- ✅ Backup Management - Create, restore, delete backups
- ✅ SSL Certificates - Let's Encrypt integration, auto-renewal
- ✅ Settings - Configure quotas, security, email parameters
- ✅ User Management - Create, edit, delete users
- ✅ Dark mode support with Tailwind CSS

**Testing**:
- ✅ 175+ tests passing across all components
- ✅ 7 MCP integration tests
- ✅ 7 AI-runtime WebSocket tests
- ✅ 8 SMTP integration tests
- ✅ End-to-end test script

## Quick Start

### Prerequisites

- Rust 1.75+ (`rustup`)
- Ollama with llama3.1:8b model
- Just command runner (`cargo install just`)
- SQLite 3

### Installation & Setup

```bash
# Clone repository
git clone https://github.com/yourusername/gk
cd gk

# Install Just (command runner)
cargo install just

# Run complete setup (installs deps, builds, creates admin user)
just setup
```

This will:
- Check all prerequisites (Rust, Ollama, SQLite)
- Pull the LLM model (llama3.1:8b)
- Build the project
- Create default admin user: `admin@delfour.co` / `password123`

### Running in Development Mode

**Option 1: Start all services at once**
```bash
just dev
```

This starts:
- Mail Server on http://localhost:8080
- Admin Panel on http://localhost:8080/admin/login
- Chat Interface on http://localhost:8080/chat/login
- MCP Server on http://localhost:8090
- AI Runtime on http://localhost:8888

**Option 2: Start services individually**
```bash
# Terminal 1 - Mail Server only
just dev-mail

# Terminal 2 - MCP Server
just dev-mcp

# Terminal 3 - AI Runtime
just dev-ai
```

### Quick Access Commands

```bash
# Open admin panel in browser
just admin

# Open chat interface in browser
just chat

# Create a new user
just create-user user@example.com password123

# List all users
just list-users

# Run all tests
just test
```

### Using the Interfaces

**Admin Panel** (http://localhost:8080/admin/login):
- Manage users, DNS configuration, backups
- Monitor system health and diagnostics
- Configure SSL certificates
- Adjust quotas and security settings

**Chat Interface** (http://localhost:8080/chat/login):
- Chat with AI about your emails
- "Liste mes emails"
- "Résume mes emails non lus"
- "Envoie un email à test@example.com"

## Testing

```bash
# Run all tests
just test

# Run specific test suites
just test-mail      # Mail server tests
just test-smtp      # SMTP integration tests
just test-mcp       # MCP integration tests
just test-ai        # AI runtime tests
just test-e2e       # End-to-end tests

# Run tests with verbose output
just test-verbose
```

**Test Results**:
- ✅ Mail-rs Tests: 175+ tests passing
- ✅ MCP Integration Tests: 7/7 passed
- ✅ AI-Runtime Tests: 7/7 passed
- ✅ SMTP Tests: 6/6 passed

### Available Just Commands

Run `just` or `just --list` to see all available commands:

```bash
# Development
just dev              # Start all services
just dev-mail         # Start mail server only
just dev-mcp          # Start MCP server
just dev-ai           # Start AI runtime

# Build
just build            # Build in debug mode
just build-release    # Build optimized release

# Testing
just test             # Run all tests
just test-verbose     # Run with output

# User Management
just create-user EMAIL PASSWORD
just list-users
just delete-user EMAIL
just create-admin     # Create default admin

# Database
just reset-db         # Reset all databases
just backup-db        # Backup databases

# Code Quality
just fmt              # Format code
just lint             # Run clippy
just check            # Check without building

# Utilities
just clean            # Clean build artifacts
just clean-all        # Clean everything
just stats            # Show project stats
just docs             # Generate documentation
```

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

### Quick Start

```bash
# Initial setup
just setup

# Start development
just dev

# Run tests
just test
```

### Development Commands

```bash
# Code formatting
just fmt              # Format all code
just fmt-check        # Check formatting

# Code quality
just lint             # Run clippy
just lint-fix         # Auto-fix issues
just check            # Type check only

# Building
just build            # Debug build
just build-release    # Release build
just build-verbose    # With verbose output

# Database management
just reset-db         # Reset databases
just backup-db        # Backup databases
just clean-maildir    # Clean mailboxes

# Utilities
just stats            # Project statistics
just docs             # Generate docs
just update           # Update dependencies
just audit            # Security audit
```

### Development Workflow

1. **Start developing**: `just dev`
2. **Make changes**: Edit code in your IDE
3. **Format & lint**: `just fmt && just lint`
4. **Test changes**: `just test`
5. **Clean up**: `just clean`

### Logs and Debugging

```bash
# Run with debug logging
RUST_LOG=debug just dev-mail

# Show logs
just logs-mail       # Mail server logs
just logs            # All logs
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

### Phase 2: Production Hardening (✅ Complete - Sprints 11-16)
- ✅ SPF/DKIM/DMARC validation and signing (Sprint 11-12)
- ✅ MIME parser with attachment support (Sprint 13)
- ✅ Quota management system (Sprint 14)
- ✅ Greylisting anti-spam (Sprint 15)
- ✅ Mail-in-a-Box admin tools (Sprint 16)
- ✅ Complete web admin interface
- ✅ DNS configuration display
- ✅ System diagnostics & monitoring
- ✅ Backup management
- ✅ SSL certificate management
- ✅ 175+ tests passing (5,631+ LOC)

### Phase 3: Extensions & New Features (🚀 In Planning)

See [ROADMAP.md](./ROADMAP.md) for complete roadmap with all planned features.

#### 🔴 Short Term (1-3 months)

**Security Features** ⭐
- **Blockchain Proof of Email** - Infalsifiable timestamping via OpenTimestamps
- **Human Lock Captcha** - Challenge-response for new senders (anti-spam)
- **AI Link Scanner** - Automatic phishing/malware detection in links
- **Security Dashboard** - Real-time security metrics + weekly reports

**Productivity Essentials** ⭐
- **Email Templates** - Signatures, quick replies, reusable templates
- **Auto-Reply / Vacation** - Out-of-office automatic responder
- **Email Scheduling** - Send emails at a specific date/time
- **Email Threading** - Conversation view (like Gmail)

**IMAP Complete**
- IMAP write operations (STORE, DELETE, EXPUNGE, COPY)
- IMAP IDLE (push notifications)
- Full-text search (Tantivy integration)

#### 🟡 Medium Term (3-6 months)

**chat-rs - Real-Time Messaging**
- WebSocket real-time chat
- 1-to-1 and group chats
- Message threading and reactions
- Bridges (WhatsApp, Slack, Google Chat)

**dav-rs - Calendar & Contacts**
- CalDAV server (calendars)
- CardDAV server (contacts)
- Multi-device sync (iOS, Android, desktop)
- Event invitations and recurring events

**Multi-User & Domains**
- Virtual domains (multi-domain support)
- Email aliases and catch-all
- Shared mailboxes (team@company.com)
- Email forwarding

#### 🟢 Long Term (6+ months)
- Mobile & Desktop apps (iOS, Android, Tauri)
- Advanced integrations (SSO, LDAP, Zapier)
- Advanced AI (multi-model, RAG, voice)
- Enterprise features (multi-tenant, white-label, billing)

**Note**: Features marked ⭐ have detailed ready-to-use implementation prompts.
See [`docs/FEATURES_PROMPTS.md`](./docs/FEATURES_PROMPTS.md) for technical specifications.

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

**Current Phase**: Phase 2 Complete - Production-hardened with admin interface
**Test Coverage**: 175+ tests passing (5,631+ LOC)
**Status**: Production-ready with comprehensive administration tools
**Admin Panel**: http://localhost:8080/admin/login
