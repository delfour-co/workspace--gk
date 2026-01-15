# GK Project - Claude Code Context

## Project Overview

GK is a self-hosted email and productivity platform built in Rust. It provides:
- SMTP/IMAP email server (mail-rs)
- AI runtime with MCP protocol (ai-runtime)
- Reverse proxy with Let's Encrypt (proxy-rs)
- CalDAV/CardDAV server (dav-rs)
- Real-time chat (chat-rs)
- Web UI (web-ui)

## Quick Reference

### Codebase Structure
```
gk/
├── mail-rs/          # Email server (SMTP/IMAP)
├── ai-runtime/       # AI orchestration
├── proxy-rs/         # Reverse proxy
├── dav-rs/           # CalDAV/CardDAV
├── chat-rs/          # Real-time chat
├── web-ui/           # Web interface
└── docs/             # Documentation
```

### Quality Gates
All code changes must pass:
```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

### Commit Convention
```
type(scope): description

Types: feat, fix, refactor, test, docs, chore
```

## Automated Workflow Agents

This project has 12 workflow agents available as slash commands:

| Command | Purpose |
|---------|---------|
| `/feature <issue>` | Develop feature from GitHub issue |
| `/bugfix <issue>` | Fix bug with regression test |
| `/audit-quality` | Code quality audit |
| `/audit-security` | Security vulnerability audit |
| `/docs` | Generate/update documentation |
| `/tests` | Improve test coverage |
| `/review <pr>` | Review pull request |
| `/perf` | Performance analysis |
| `/deps` | Dependency management |
| `/release` | Prepare release |
| `/refactor` | Safe code refactoring |
| `/devops` | Docker, CI/CD, deployment |

### Usage Examples

```
/feature 42
→ Creates branch, implements issue #42, runs quality gates, prepares PR

/bugfix 123
→ Creates failing test, fixes bug #123, ensures regression test passes

/audit-security mail-rs
→ Security audit of mail-rs module with OWASP checks

/review 15
→ Reviews PR #15 with security, performance, and quality checks
```

## Development Workflow

### Standard Feature Flow
1. Pick GitHub issue
2. `/feature <issue-number>`
3. Agent creates branch, implements, tests, prepares PR

### Bug Fix Flow
1. Pick bug issue
2. `/bugfix <issue-number>`
3. Agent writes failing test first, then fixes

### Before Merge
1. `/audit-quality` - Check code quality
2. `/audit-security` - Check for vulnerabilities
3. `/review <pr>` - Complete PR review

## Key Documentation

- `docs/mail-rs_CLAUDE.md` - Email server specs
- `docs/ai-runtime_CLAUDE.md` - AI runtime specs
- `.claude/agents/` - Detailed agent prompts
- `.claude/rules.md` - Project conventions

## Tech Stack

- **Language**: Rust
- **Async**: Tokio
- **HTTP**: Axum
- **Database**: PostgreSQL
- **Frontend**: HTMX + Alpine.js + Tailwind
