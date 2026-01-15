# GK Project - Claude Code Context

## Vision

**GK** est une suite de communication self-hosted avec interface **AI-native conversationnelle**. Alternative à Google Workspace où l'utilisateur dialogue avec un assistant IA local (via MCP) pour gérer tous les services au lieu de naviguer entre plusieurs interfaces.

**Différenciateur clé**: Interface conversationnelle propulsée par un LLM local via le protocole MCP (Model Context Protocol).

## Project Statistics

| Metric | Value |
|--------|-------|
| **Lines of Code** | ~37,500 Rust |
| **Tests** | 427 total (385 in mail-rs) |
| **Source Files** | ~120 |
| **Phase** | Phase 2 Complete, Phase 3 in progress |

## Component Status

| Component | Status | Description |
|-----------|--------|-------------|
| **mail-rs** | 🟢 Production Ready | SMTP/IMAP server, Web UI, Admin Panel |
| **ai-runtime** | 🟢 Functional | LLM runtime (Ollama) + MCP orchestrator |
| **mcp-mail-server** | 🟢 Functional | 7 MCP tools for email operations |
| **proxy-rs** | ⚪ Planned | Reverse proxy with Let's Encrypt |
| **chat-rs** | ⚪ Planned | Real-time messaging |
| **dav-rs** | ⚪ Planned | CalDAV/CardDAV server |

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
              │  (SMTP: 2525)  │  - IMAP: 1993
              │  (IMAP: 1993)  │  - Admin Panel + Chat UI
              └────────┬───────┘
                       │
                       │  WebSocket + HTTP
                       ▼
              ┌────────────────┐
              │  ai-runtime    │  LLM + MCP Orchestrator
              │  (Port 8888)   │  - Ollama (llama3.1:8b)
              └────────┬───────┘  - Email summarization
                       │          - Streaming responses
                       │
                       │  MCP Protocol (JSON-RPC 2.0)
                       ▼
              ┌────────────────┐
              │mcp-mail-server │  7 Email Tools via MCP
              │  (Port 8090)   │  - send_email, list_emails
              └────────────────┘  - read_email, search_emails
                                  - mark_as_read, delete_email
                                  - get_email_count
```

## Implemented Features

### mail-rs (Production Ready)

**SMTP Server**:
- SMTP receiver with STARTTLS (port 2525)
- SMTP AUTH (LOGIN, PLAIN) with Argon2 hashing
- Maildir storage with atomic operations
- Queue SQLite for outgoing emails
- DNS MX lookup with failover
- Rate limiting and security limits

**IMAP Server**:
- Full IMAP4rev1 implementation (port 1993)
- Commands: LOGIN, SELECT, EXAMINE, FETCH, SEARCH, STORE, COPY, EXPUNGE, CLOSE, LOGOUT
- Flags support (\\Seen, \\Answered, \\Flagged, \\Deleted, \\Draft)
- UID support for all commands
- Maildir backend integration

**Email Security** (Sprint 11-12):
- SPF validation for incoming emails
- DKIM signing for outgoing emails
- DKIM validation for incoming emails
- DMARC policy enforcement
- Authentication-Results headers (RFC 8601)

**Email Features** (Sprint 13-18):
- MIME parser with attachment support
- Quota management (storage + message limits)
- Greylisting anti-spam system
- Email templates system
- Auto-reply / Vacation responder
- Import/Export (MBOX, EML formats)
- 2FA/MFA with TOTP support

**Admin Panel**:
- DNS Configuration display (A, MX, SPF, DKIM, DMARC)
- System Diagnostics with real-time health monitoring
- Backup Management (create, restore, delete)
- SSL Certificates management (Let's Encrypt integration)
- User Management (CRUD operations)
- Greylisting, Quotas, Security, Monitoring pages
- Dark mode support with Tailwind CSS

**Web UI**:
- Conversational chat interface
- JWT authentication
- WebSocket real-time updates
- Server-side rendering (Askama templates)
- Streaming AI responses

### ai-runtime (Functional)

- Ollama LLM integration (llama3.1:8b, qwen2.5:3b)
- Streaming word-by-word responses
- Email summarization with SQLite persistence
- Real-time email notifications via WebSocket
- MCP client implementation
- Tool discovery and orchestration

### mcp-mail-server (Functional)

7 MCP tools implemented:
1. `send_email` - Send emails via SMTP
2. `list_emails` - List emails from maildir
3. `read_email` - Read email content
4. `search_emails` - Search in emails
5. `mark_as_read` - Mark emails as read
6. `delete_email` - Delete emails
7. `get_email_count` - Count unread emails

## Codebase Structure

```
gk/
├── mail-rs/                 # Email server (SMTP/IMAP)
│   ├── src/
│   │   ├── smtp/            # SMTP server implementation
│   │   ├── imap/            # IMAP server implementation
│   │   ├── maildir/         # Maildir storage
│   │   ├── authentication/  # SPF, DKIM, DMARC
│   │   ├── security/        # Rate limiting, validation
│   │   ├── mfa/             # 2FA/TOTP support
│   │   ├── templates/       # Email templates
│   │   ├── auto_reply/      # Vacation responder
│   │   ├── import_export/   # MBOX, EML support
│   │   ├── admin/           # Admin panel handlers
│   │   ├── api/             # REST API
│   │   └── web/             # Web UI (chat interface)
│   └── tests/               # Integration tests
│
├── ai-runtime/              # AI orchestration
│   ├── src/
│   │   ├── llm/             # Ollama integration
│   │   ├── mcp/             # MCP client
│   │   └── websocket/       # Real-time communication
│   └── tests/
│
├── mcp-mail-server/         # MCP server for email
│   ├── src/
│   │   └── tools/           # 7 email tools
│   └── tests/
│
├── docs/                    # Technical specifications
│   ├── *_CLAUDE.md          # Component specs
│   └── INDEX.md             # Documentation index
│
└── .claude/                 # Claude Code configuration
    ├── agents/              # 12 workflow agents
    ├── commands/            # 12 slash commands
    ├── agents.md            # Agent guide
    └── rules.md             # Project conventions
```

## Quick Start Commands

```bash
# Install Just (command runner)
cargo install just

# Run complete setup
just setup

# Start all services
just dev

# Start individual services
just dev-mail    # Mail server on :8080
just dev-mcp     # MCP server on :8090
just dev-ai      # AI runtime on :8888

# Run tests
just test
just test-mail
just test-verbose

# User management
just create-user user@example.com password123
just list-users
just create-admin  # admin@delfour.co / password123

# Code quality
just fmt          # Format code
just lint         # Run clippy
just check        # Type check

# Open interfaces
just admin        # Open admin panel
just chat         # Open chat interface
```

## Tech Stack

| Category | Technology |
|----------|------------|
| **Language** | 100% Rust |
| **Async Runtime** | Tokio |
| **HTTP Framework** | Axum |
| **Database** | SQLite (mail, ai), PostgreSQL (chat - planned) |
| **LLM** | Ollama (llama3.1:8b, qwen2.5:3b) |
| **Frontend** | HTMX + Alpine.js + Tailwind CSS |
| **Templates** | Askama (server-side rendering) |
| **Auth** | JWT + Argon2 password hashing |
| **Protocols** | SMTP, IMAP, WebSocket, MCP (JSON-RPC 2.0) |

## Quality Gates

All code changes must pass:
```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

## Commit Convention

```
type(scope): description

Types: feat, fix, refactor, test, docs, chore, perf
```

Examples:
- `feat(imap): add IDLE command support`
- `fix(smtp): handle timeout correctly`
- `test(auth): add SPF validation tests`

## Automated Workflow Agents

12 slash commands available for development workflows:

| Command | Purpose |
|---------|---------|
| `/feature <issue>` | Develop feature from GitHub issue |
| `/bugfix <issue>` | Fix bug with mandatory regression test |
| `/audit-quality` | Code quality audit (smells, SOLID, patterns) |
| `/audit-security` | Security audit (OWASP, vulnerabilities) |
| `/docs` | Generate/update documentation |
| `/tests` | Improve test coverage |
| `/review <pr>` | Review pull request |
| `/perf` | Performance analysis |
| `/deps` | Dependency management |
| `/release` | Prepare release (version, changelog, tag) |
| `/refactor` | Safe code refactoring |
| `/devops` | Docker, CI/CD, deployment |

### Workflow Example

```
/feature 42
→ Creates branch feature/issue-42-description
→ Implements the feature
→ Runs all quality gates
→ Creates PR when ready

/bugfix 123
→ Creates branch fix/issue-123-description
→ Writes failing test FIRST
→ Implements minimal fix
→ Ensures regression test passes
→ Creates PR
```

## Key Documentation

| File | Description |
|------|-------------|
| `docs/mail-rs_CLAUDE.md` | Email server specifications |
| `docs/ai-runtime_CLAUDE.md` | AI runtime specifications |
| `docs/mcp-mail-server_CLAUDE.md` | MCP server specifications |
| `.claude/rules.md` | Project conventions and rules |
| `.claude/agents/` | Detailed agent prompts |
| `ROADMAP.md` | Project roadmap |

## Security Rules

- **ALWAYS** validate external inputs
- **ALWAYS** use timeouts on I/O operations
- **NEVER** use `unwrap()` or `expect()` in production
- **NEVER** log passwords or tokens
- **NEVER** use `unsafe` without documentation and review
- Validate emails according to RFC 5321
- Use `?` for error propagation

## GitHub Issues

50 issues created covering remaining work:
- P0 (Critical): Core features
- P1 (High): Important features
- P2 (Medium): Nice-to-have
- P3 (Low): Future considerations

Use `/feature <issue-number>` or `/bugfix <issue-number>` to work on issues.

## Useful Links

- RFC 5321 (SMTP): https://www.rfc-editor.org/rfc/rfc5321
- RFC 3501 (IMAP): https://www.rfc-editor.org/rfc/rfc3501
- RFC 6376 (DKIM): https://www.rfc-editor.org/rfc/rfc6376
- RFC 7208 (SPF): https://www.rfc-editor.org/rfc/rfc7208
- MCP Protocol: https://modelcontextprotocol.io/
- Tokio: https://tokio.rs/
- Axum: https://github.com/tokio-rs/axum
