# mail-rs

SMTP/IMAP mail server written in Rust.

## Features

### Currently Implemented (Sprint 1)
- ✅ SMTP receiver (basic)
- ✅ Maildir storage
- ✅ Command parsing (HELO, EHLO, MAIL FROM, RCPT TO, DATA, QUIT)
- ✅ Session state machine

### Roadmap
- [ ] SMTP sender (client)
- [ ] SMTP queue with retry
- [ ] TLS support
- [ ] SMTP AUTH
- [ ] IMAP server
- [ ] SPF checking
- [ ] DKIM signing
- [ ] REST API

## Quick Start

### Build

```bash
cargo build
```

### Run

```bash
# Copy example config
cp config.example.toml config.toml

# Edit config as needed
# ...

# Run server
cargo run
```

The SMTP server will listen on `0.0.0.0:2525` by default (non-privileged port for testing).

### Test with swaks

```bash
# Install swaks (Swiss Army Knife for SMTP)
# On Debian/Ubuntu: apt-get install swaks
# On macOS: brew install swaks

# Send test email
swaks --to test@localhost \
      --from sender@example.com \
      --server localhost:2525 \
      --body "Test email from swaks"
```

### Check received mail

```bash
# Emails are stored in Maildir format
ls -la /tmp/maildir/test@localhost/new/
cat /tmp/maildir/test@localhost/new/*
```

## Configuration

See `config.example.toml` for all available options.

## Development

### Run tests

```bash
cargo test
```

### Watch mode (auto-recompile on change)

```bash
cargo install cargo-watch
cargo watch -x run
```

## Architecture

See [../docs/mail-rs_CLAUDE.md](../docs/mail-rs_CLAUDE.md) for detailed architecture documentation.

## License

AGPL-3.0
