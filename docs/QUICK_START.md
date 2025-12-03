# Quick Start Guide - GK Mail Suite

Get the GK AI-native email system running in under 10 minutes.

---

## 📋 Prerequisites

### Required Software

1. **Rust 1.75+**
   ```bash
   # Install rustup
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source $HOME/.cargo/env

   # Verify installation
   rustc --version
   ```

2. **Ollama with LLM Model**
   ```bash
   # Install Ollama
   curl -fsSL https://ollama.com/install.sh | sh

   # Pull required model
   ollama pull llama3.1:8b

   # Verify Ollama is running
   ollama list
   # Should show: llama3.1:8b
   ```

3. **SQLite 3** (usually pre-installed on Linux/macOS)
   ```bash
   sqlite3 --version
   ```

### System Requirements

- **OS**: Linux, macOS, or Windows (WSL2)
- **RAM**: 8GB minimum (for LLM)
- **Disk**: 10GB free space (for model + data)
- **Ports**: 2525, 1993, 8080, 8090, 8888 available

---

## 🚀 Installation

### 1. Clone Repository

```bash
git clone https://github.com/yourusername/gk
cd gk
```

### 2. Build All Services

```bash
# Build all binaries (takes 5-10 minutes first time)
cargo build --release

# Verify builds succeeded
ls -lh target/release/mail-rs
ls -lh target/release/mcp-mail-server
ls -lh target/release/ai-runtime
```

### 3. Create Configuration

The default config files are already in place:

- `mail-rs/config.toml` - Mail server configuration
- `ai-runtime/config.toml` - AI runtime configuration (optional, uses defaults)

**Optional**: Edit `mail-rs/config.toml` to customize:

```toml
[smtp]
listen_addr = "0.0.0.0:2525"
enable_tls = false              # Set to true in production
enable_auth = true
require_auth = false            # Set to true in production

[imap]
listen_addr = "0.0.0.0:1993"

[api]
listen_addr = "0.0.0.0:8080"
jwt_secret = "change-this-in-production"  # ⚠️ Change this!
```

### 4. Create First User

```bash
# Create admin user
cargo run --bin mail-user -- add admin@delfour.co password123

# Verify user created
sqlite3 mail-rs/data/users.db "SELECT email FROM users;"
```

---

## 🎬 Running the Services

You need **3 terminal windows** to run all services.

### Terminal 1: Mail Server (SMTP + IMAP + Web UI)

```bash
cargo run --bin mail-rs -- --config mail-rs/config.toml
```

**Expected output**:
```
Starting mail-rs server...
SMTP server listening on 0.0.0.0:2525
IMAP server listening on 0.0.0.0:1993
Web server listening on 0.0.0.0:8080
```

**Services started**:
- ✅ SMTP Server: `localhost:2525`
- ✅ IMAP Server: `localhost:1993`
- ✅ Web UI: `http://localhost:8080`

### Terminal 2: MCP Mail Server

```bash
cd mcp-mail-server
cargo run --release
```

**Expected output**:
```
MCP Mail Server starting...
Server listening on 0.0.0.0:8090
Available tools: 7
```

**Service started**:
- ✅ MCP Server: `http://localhost:8090`

### Terminal 3: AI Runtime

```bash
cd ai-runtime
cargo run --release
```

**Expected output**:
```
AI Runtime starting...
Connecting to Ollama at http://localhost:11434
Model: llama3.1:8b
WebSocket server listening on 0.0.0.0:8888
HTTP server listening on 0.0.0.0:8888
```

**Service started**:
- ✅ AI Runtime: `http://localhost:8888`
- ✅ WebSocket: `ws://localhost:8888/ws`

---

## ✅ Verify Installation

### 1. Check Services Health

```bash
# Mail server (SMTP)
telnet localhost 2525
# Should respond: 220 mail-rs SMTP Server
# Type: QUIT

# MCP server
curl http://localhost:8090/health
# Should return: {"status":"ok"}

# AI runtime
curl http://localhost:8888/health
# Should return: {"status":"ok"}

# Ollama
ollama list
# Should show: llama3.1:8b
```

### 2. Access Web UI

Open browser to: **http://localhost:8080**

**Login credentials**:
- Email: `admin@delfour.co`
- Password: `password123`

You should see the conversational AI chat interface.

---

## 💬 Using the Chat Interface

### Basic Commands

Type these messages in the chat interface:

1. **List emails**:
   ```
   Liste mes emails
   ```

2. **Send an email**:
   ```
   Envoie un email à test@example.com avec le sujet "Test" et le message "Hello"
   ```

3. **Search emails**:
   ```
   Recherche les emails de sender@example.com
   ```

4. **Check unread count**:
   ```
   Combien d'emails non lus j'ai?
   ```

5. **Get summary**:
   ```
   Résume mes emails non lus
   ```

### Expected Behavior

- AI responds in **real-time streaming** (word by word)
- **Interactive buttons** appear for common actions
- **Email cards** show summaries with metadata
- **Real-time notifications** when new emails arrive

---

## 📧 Testing Email Reception

### Send Test Email via SMTP

```bash
# Install swaks (SMTP test tool)
# Ubuntu/Debian: apt install swaks
# macOS: brew install swaks

# Send test email
swaks --to admin@delfour.co \
      --from sender@example.com \
      --server localhost:2525 \
      --header "Subject: Test Email" \
      --body "This is a test email from swaks"
```

### Verify Email Received

1. **Via Web UI**: Refresh chat, ask "Liste mes emails"
2. **Via Maildir**:
   ```bash
   ls -la mail-rs/data/maildir/admin@delfour.co/new/
   cat mail-rs/data/maildir/admin@delfour.co/new/*.eml
   ```
3. **Via IMAP**:
   ```bash
   telnet localhost 1993
   a1 LOGIN admin@delfour.co password123
   a2 SELECT INBOX
   a3 FETCH 1:* (FLAGS BODY.PEEK[HEADER])
   a4 LOGOUT
   ```

---

## 🔧 Configuration Options

### Mail Server (mail-rs/config.toml)

```toml
[smtp]
listen_addr = "0.0.0.0:2525"    # SMTP listen address
enable_tls = false               # Enable STARTTLS
enable_auth = true               # Enable authentication
require_auth = false             # Require auth for all emails
max_message_size = 10485760     # 10MB max email size

[imap]
listen_addr = "0.0.0.0:1993"    # IMAP listen address

[api]
listen_addr = "0.0.0.0:8080"    # Web UI listen address
jwt_secret = "your-secret-key"   # JWT signing secret (⚠️ change in production)
token_expiry_hours = 24         # JWT token validity

[storage]
maildir_path = "data/maildir"   # Maildir base path
```

### AI Runtime (ai-runtime/config.toml)

```toml
[llm]
provider = "ollama"
model = "llama3.1:8b"           # LLM model to use
base_url = "http://localhost:11434"
temperature = 0.7
max_tokens = 2000

[mcp]
mail_server_url = "http://localhost:8090"  # MCP server URL

[server]
listen_addr = "0.0.0.0:8888"
```

---

## 🐛 Troubleshooting

### Service Won't Start

**Problem**: Port already in use
```
Error: Address already in use (os error 98)
```

**Solution**: Kill existing process
```bash
# Find process using port
lsof -i :8080

# Kill process
kill -9 <PID>
```

---

### LLM Not Responding

**Problem**: AI runtime hangs, no responses in chat

**Solutions**:
1. **Check Ollama is running**:
   ```bash
   ollama list
   # If empty or error, restart:
   ollama serve
   ```

2. **Check model is loaded**:
   ```bash
   ollama pull llama3.1:8b
   ```

3. **Check AI runtime logs**:
   ```bash
   RUST_LOG=debug cargo run --bin ai-runtime
   ```

---

### WebSocket Connection Failed

**Problem**: Chat interface shows "Connecting..." indefinitely

**Solutions**:
1. **Check ai-runtime is running** on port 8888
2. **Check browser console** for WebSocket errors (F12)
3. **Verify firewall** allows port 8888
4. **Check logs**:
   ```bash
   # In ai-runtime terminal
   # Should see: WebSocket connection established
   ```

---

### Emails Not Received

**Problem**: Sent email via swaks but not appearing in mailbox

**Solutions**:
1. **Check SMTP server logs**:
   ```bash
   # In mail-rs terminal
   # Should see: Received email from sender@example.com
   ```

2. **Verify maildir exists**:
   ```bash
   ls -la mail-rs/data/maildir/admin@delfour.co/new/
   ```

3. **Check user exists**:
   ```bash
   sqlite3 mail-rs/data/users.db "SELECT email FROM users WHERE email='admin@delfour.co';"
   ```

---

### Authentication Failed

**Problem**: Cannot login to Web UI or SMTP AUTH fails

**Solutions**:
1. **Verify user credentials**:
   ```bash
   # List users
   sqlite3 mail-rs/data/users.db "SELECT email FROM users;"

   # Create new user
   cargo run --bin mail-user -- add admin@delfour.co newpassword
   ```

2. **Restart mail-rs** to reload user database
3. **Check JWT secret** is set in config.toml

---

## 📊 Monitoring and Logs

### Enable Debug Logging

```bash
# Mail server
RUST_LOG=debug cargo run --bin mail-rs -- --config mail-rs/config.toml

# MCP server
RUST_LOG=mcp_mail_server=debug cargo run --release

# AI runtime
RUST_LOG=ai_runtime=debug cargo run --release
```

### Check Database Contents

```bash
# Users database
sqlite3 mail-rs/data/users.db "SELECT * FROM users;"

# Email queue
sqlite3 mail-rs/data/queue.db "SELECT * FROM queue;"

# Email summaries
sqlite3 ai-runtime/summaries.db "SELECT email_id, summary FROM summaries LIMIT 10;"
```

### Monitor Maildir

```bash
# Watch for new emails
watch -n 1 "ls -la mail-rs/data/maildir/admin@delfour.co/new/"

# Count emails
find mail-rs/data/maildir/admin@delfour.co/new/ -type f | wc -l
```

---

## 🧪 Running Tests

### All Tests

```bash
# Run all tests (requires services running)
cargo test

# Run specific test suite
cargo test --package mcp-mail-server --test integration_test
cargo test --package ai-runtime --test integration_test
cargo test --package mail-rs --test smtp_test
```

### Test Coverage

- ✅ **MCP Integration Tests**: 7/7 passing
- ✅ **AI-Runtime Tests**: 7/7 passing
- ✅ **SMTP Tests**: 6/6 passing (2 ignored)
- **Total**: 20/22 tests passing (91% coverage)

See [tests/README.md](../tests/README.md) for detailed test documentation.

---

## 🚢 Production Deployment

For production deployment, see:
- [mail-rs/docs/DEPLOYMENT.md](../mail-rs/docs/DEPLOYMENT.md) - Production setup
- [mail-rs/docs/SECURITY.md](../mail-rs/docs/SECURITY.md) - Security hardening
- [PROJECT_STATUS.md](../PROJECT_STATUS.md) - Current status

### Quick Production Checklist

- [ ] Change `jwt_secret` in `mail-rs/config.toml`
- [ ] Set `require_auth = true` in SMTP config
- [ ] Enable TLS with proper certificates
- [ ] Configure DNS MX records
- [ ] Set up SPF/DKIM/DMARC records
- [ ] Configure firewall (allow ports 25, 587, 993, 443)
- [ ] Set up backup automation
- [ ] Enable monitoring and alerting

---

## 📚 Additional Resources

### Documentation

- [README.md](../README.md) - Project overview
- [PROJECT_STATUS.md](../PROJECT_STATUS.md) - Current development status
- [tests/README.md](../tests/README.md) - Test suite documentation
- [ai-runtime/README.md](../ai-runtime/README.md) - AI runtime details
- [mail-rs/README.md](../mail-rs/README.md) - Mail server details

### External Resources

- [Ollama Documentation](https://ollama.com/docs)
- [SMTP RFC 5321](https://datatracker.ietf.org/doc/html/rfc5321)
- [IMAP RFC 3501](https://datatracker.ietf.org/doc/html/rfc3501)
- [MCP Protocol Spec](https://spec.modelcontextprotocol.io/)
- [Maildir Format](https://cr.yp.to/proto/maildir.html)

---

## 🎯 Next Steps

Once you have the system running:

1. **Explore the Chat Interface**
   - Try different email operations
   - Test the AI assistant capabilities
   - Experiment with search queries

2. **Send Real Emails**
   - Configure your email client (Thunderbird, etc.)
   - Test with external email addresses
   - Set up email forwarding

3. **Customize the System**
   - Modify LLM prompts in `ai-runtime/src/llm.rs`
   - Add custom MCP tools in `mcp-mail-server/src/main.rs`
   - Customize web UI templates in `mail-rs/templates/`

4. **Review Test Suite**
   - Run tests to understand system behavior
   - Read test code to learn API usage
   - Add your own integration tests

5. **Plan Production Deployment**
   - Review security requirements
   - Plan DNS configuration
   - Set up monitoring and backups

---

## ❓ Getting Help

### Common Issues

Check the **Troubleshooting** section above for solutions to common problems.

### Reporting Bugs

If you encounter a bug:

1. Check existing issues: [GitHub Issues](https://github.com/yourusername/gk/issues)
2. Include:
   - Steps to reproduce
   - Expected vs actual behavior
   - Log output (with `RUST_LOG=debug`)
   - System info (OS, Rust version)

### Feature Requests

See [PROJECT_STATUS.md](../PROJECT_STATUS.md) for planned features and roadmap.

---

**Last Updated**: 2025-12-03
**System Version**: 1.0 (Phase 1 Complete)
**Status**: ✅ Production-ready

**Congratulations! You now have a fully functional AI-native email system running locally.**
