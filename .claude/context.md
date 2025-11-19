# Contexte Technique - GK Project

## Architecture détaillée

### Flux de données

#### Email (mail-rs)
```
Client SMTP → proxy-rs → mail-rs → Maildir storage
                                    ↓
                              SQLite queue (outgoing)
                                    ↓
                              DNS MX lookup → External SMTP
```

#### AI Conversation (ai-runtime)
```
User → web-ui → ai-runtime → LLM inference
                              ↓
                         MCP Registry
                              ↓
                    [mcp-mail-server, mcp-chat, ...]
                              ↓
                    [mail-rs API, chat-rs API, ...]
```

### Ports par défaut

| Service | Port | Protocole | Description |
|---------|------|-----------|-------------|
| proxy-rs | 80, 443 | HTTP/HTTPS | Point d'entrée |
| web-ui | 3000 | HTTP | Interface web |
| mail-rs | 8080 | HTTP | API REST mail |
| chat-rs | 8081 | HTTP/WS | API chat + WebSocket |
| dav-rs | 8082 | HTTP | CalDAV/CardDAV |
| ai-runtime | 8888 | HTTP | API conversation |
| mcp-mail | 8090 | HTTP | MCP server mail |
| mcp-chat | 8091 | HTTP | MCP server chat |

### Bases de données

#### SQLite
- **mail-rs** : Queue emails sortants, utilisateurs AUTH
- **ai-runtime** : Historique conversations, cache LLM
- **dav-rs** : Calendriers, contacts

#### PostgreSQL
- **chat-rs** : Messages, rooms, utilisateurs chat

### Protocoles implémentés

#### SMTP (mail-rs)
- ✅ HELO, EHLO
- ✅ MAIL FROM, RCPT TO
- ✅ DATA
- ✅ QUIT, RSET, NOOP
- ✅ STARTTLS
- ✅ AUTH (PLAIN, LOGIN)
- ⏳ AUTH (CRAM-MD5, DIGEST-MD5)

#### IMAP (mail-rs)
- ⏳ LOGIN
- ⏳ SELECT, EXAMINE
- ⏳ FETCH
- ⏳ SEARCH
- ⏳ IDLE

#### MCP (ai-runtime)
- ✅ JSON-RPC 2.0
- ✅ Tool discovery
- ✅ Tool calling
- ✅ Error handling
- ⏳ Streaming responses

### Formats de données

#### Email (RFC 5322)
```
From: sender@example.com
To: recipient@example.com
Subject: Test
Date: Wed, 19 Nov 2025 10:00:00 +0100

Body content...
```

#### MCP Request
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "send_email",
    "arguments": {
      "to": "user@example.com",
      "subject": "Hello",
      "body": "World"
    }
  }
}
```

#### Configuration (TOML)
```toml
[server]
host = "0.0.0.0"
port = 2525

[storage]
maildir_path = "/var/mail"

[security]
enable_tls = true
require_auth = false
```

## Décisions architecturales

### Pourquoi Rust ?
- Performance native
- Sécurité mémoire (pas de segfaults)
- Écosystème async mature (Tokio)
- Pas de GC (latence prévisible)

### Pourquoi Tokio ?
- Runtime async performant
- Écosystème complet (fs, net, time)
- Compatible avec la plupart des crates Rust

### Pourquoi SQLite pour mail ?
- Simple à déployer (fichier unique)
- Suffisant pour la plupart des cas
- Pas besoin de serveur séparé
- Migration vers PostgreSQL possible si besoin

### Pourquoi PostgreSQL pour chat ?
- Besoin de concurrence élevée
- Requêtes complexes (search, threads)
- Réplication possible

### Pourquoi MCP ?
- Protocole standardisé
- Extensible (nouveaux tools facilement)
- Découplage LLM ↔ Services
- Support multi-LLM possible

### Pourquoi HTMX au lieu de React ?
- Plus simple (moins de JS)
- Performance (moins de bundle)
- Aligné avec philosophie "simple"
- Suffisant pour interface conversationnelle

## Patterns utilisés

### State Machine (SMTP Session)
```rust
enum SmtpState {
    Initial,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
    Quit,
}
```

### Builder Pattern (Configuration)
```rust
let config = Config::builder()
    .host("0.0.0.0")
    .port(2525)
    .enable_tls(true)
    .build()?;
```

### Error Propagation
```rust
fn process_email(email: &str) -> Result<()> {
    validate_email(email)?;
    store_email(email)?;
    Ok(())
}
```

### Async Task Spawning
```rust
tokio::spawn(async move {
    if let Err(e) = handle_session(socket).await {
        error!("Session error: {}", e);
    }
});
```

## Dépendances principales

### mail-rs
- `tokio` - Runtime async
- `trust-dns-resolver` - DNS resolution
- `sqlx` - SQLite queries
- `rustls` - TLS
- `argon2` - Password hashing

### ai-runtime
- `tokio` - Runtime async
- `axum` - HTTP server
- `serde_json` - JSON serialization
- `llama-cpp-rs` (future) - LLM inference

### web-ui
- `axum` - HTTP server
- `askama` (future) - Templates
- `tower-http` - Static files, CORS

## Limitations connues

### mail-rs
- ⚠️ STARTTLS stream upgrade incomplet (nécessite refactoring)
- ⚠️ SPF : CIDR matching partiel
- ⚠️ DKIM : Vérification cryptographique incomplète
- ⚠️ IMAP : Pas encore implémenté

### ai-runtime
- ⚠️ LLM réel pas encore intégré (mock seulement)
- ⚠️ Streaming responses pas implémenté
- ⚠️ Registry MCP servers basique

## Prochaines étapes techniques

### Court terme (1-2 semaines)
1. Compléter STARTTLS stream upgrade
2. Finaliser vérification DKIM cryptographique
3. Démarrer IMAP read-only

### Moyen terme (1 mois)
1. IMAP complet avec IDLE
2. API REST mail-rs
3. Intégration LLM réel dans ai-runtime

### Long terme (3-6 mois)
1. Proxy-rs avec Let's Encrypt
2. Web UI complète
3. Chat-rs + DAV-rs
4. MVP production-ready

## Environnement de développement

### Prérequis
```bash
# Rust
rustup install stable
rustup component add rustfmt clippy

# Outils
cargo install cargo-audit cargo-watch

# Tests SMTP
sudo apt install swaks  # ou équivalent
```

### Commandes utiles
```bash
# Build
cargo build

# Tests
cargo test

# Lint
cargo clippy -- -D warnings

# Format
cargo fmt

# Doc
cargo doc --open

# Run mail-rs
cd mail-rs && cargo run
```

### Structure des tests
```
mail-rs/
├── src/
│   └── [modules].rs
└── tests/
    ├── smtp_commands_test.rs
    ├── smtp_integration_test.rs
    ├── smtp_auth_test.rs
    └── smtp_sender_test.rs
```

## Notes d'implémentation

### SMTP Receiver
- State machine pour gérer les sessions
- Timeouts sur chaque opération
- Validation stricte selon RFC 5321
- Maildir pour stockage atomique

### SMTP Sender
- Queue SQLite pour persistance
- Retry exponentiel (2min → 32min)
- Fallback multi-serveurs MX
- Worker asynchrone pour traitement

### SMTP AUTH
- Argon2 pour hashage passwords
- Base64 pour encodage credentials
- Support PLAIN et LOGIN
- Option require TLS avant AUTH

### SPF/DKIM
- DNS lookup asynchrone
- Parsing strict des enregistrements
- Résultats typés (Pass, Fail, etc.)
- Foundation pour DMARC future

### AI Runtime
- JSON-RPC 2.0 strict
- Tool registry avec découverte
- Mock LLM avec pattern matching
- Support français natif

## Références techniques

### RFCs
- RFC 5321 - SMTP
- RFC 5322 - Email format
- RFC 3501 - IMAP
- RFC 6376 - DKIM
- RFC 7208 - SPF
- RFC 4616 - AUTH PLAIN

### Documentation Rust
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Tokio Tutorial](https://tokio.rs/tokio/tutorial)
- [Axum Guide](https://docs.rs/axum/latest/axum/)

### MCP
- [MCP Specification](https://modelcontextprotocol.io/)
- [MCP Examples](https://github.com/modelcontextprotocol)

