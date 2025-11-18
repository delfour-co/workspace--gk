# mail-rs - Serveur Mail SMTP/IMAP en Rust

## Vue d'ensemble

`mail-rs` est un serveur mail complet écrit en Rust, fournissant les protocoles SMTP (envoi/réception) et IMAP (lecture/gestion) avec un focus sur la sécurité, la performance et la simplicité de déploiement.

## Contexte du projet global

Ce composant fait partie d'une suite complète d'outils de communication self-hosted avec interface conversationnelle AI-native. `mail-rs` est la fondation critique du système - sans mail fonctionnel, le reste n'a pas de sens.

### Interfaces avec les autres composants

- **Expose** : REST API pour `mcp-mail-server`
- **Consommé par** : `web-ui` (via API), clients IMAP/SMTP standards
- **Derrière** : `proxy-rs` (reverse proxy pour API REST)
- **Indépendant** : Peut fonctionner standalone sans les autres composants

## Responsabilités

### Primaires
1. **Réception de mails** (SMTP server)
   - Accepter connexions SMTP entrantes (port 25, 587)
   - Valider expéditeurs (SPF, reverse DNS)
   - Stocker mails reçus
   - Anti-spam basique

2. **Envoi de mails** (SMTP client)
   - Envoyer mails sortants vers serveurs externes
   - Signature DKIM
   - Queue avec retry logic
   - Gestion rebonds (bounces)

3. **Accès aux mails** (IMAP server)
   - Lecture boîtes mail (port 993)
   - Gestion dossiers (INBOX, Sent, Trash, Drafts)
   - Recherche mails
   - Flags (read, starred, deleted)
   - Push notifications (IDLE)

4. **API REST**
   - Endpoints HTTP pour MCP server
   - Authentication JWT
   - CRUD opérations sur mails
   - Search API

### Secondaires
- Logs structurés (observabilité)
- Métriques Prometheus
- Configuration hot-reload
- Backup/restore mailboxes

## Architecture technique

### Stack Rust

```toml
[dependencies]
# Async runtime
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"

# TLS
tokio-rustls = "0.24"
rustls = "0.21"
rustls-pemfile = "1"

# Mail parsing/generation
mail-parser = "0.9"
mail-builder = "0.3"

# DNS
trust-dns-resolver = "0.23"

# HTTP API
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# Auth
jsonwebtoken = "9"
argon2 = "0.5"

# Database
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"
toml = "0.8"

# Crypto
ring = "0.17"
rsa = "0.9"
sha2 = "0.10"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bytes = "1"
futures = "0.3"

# CLI
clap = { version = "4", features = ["derive"] }

# Testing
mockall = "0.12"
```

### Structure du projet

```
mail-rs/
├── Cargo.toml
├── config.example.yaml
├── README.md
├── Dockerfile
├── docker-compose.yml
│
├── src/
│   ├── main.rs                 # Entry point
│   ├── config.rs               # Configuration loading
│   ├── error.rs                # Error types
│   │
│   ├── smtp/
│   │   ├── mod.rs
│   │   ├── server.rs           # SMTP server listener
│   │   ├── session.rs          # SMTP session handler
│   │   ├── commands.rs         # SMTP command parsing (HELO, MAIL, RCPT, DATA)
│   │   ├── client.rs           # SMTP client (outgoing)
│   │   ├── auth.rs             # SMTP AUTH mechanisms
│   │   ├── queue.rs            # Outgoing mail queue
│   │   └── relay.rs            # Mail relay logic
│   │
│   ├── imap/
│   │   ├── mod.rs
│   │   ├── server.rs           # IMAP server listener
│   │   ├── session.rs          # IMAP session handler
│   │   ├── commands/           # IMAP commands
│   │   │   ├── mod.rs
│   │   │   ├── authenticate.rs # LOGIN, AUTHENTICATE
│   │   │   ├── select.rs       # SELECT, EXAMINE
│   │   │   ├── fetch.rs        # FETCH
│   │   │   ├── store.rs        # STORE (flags)
│   │   │   ├── search.rs       # SEARCH
│   │   │   ├── copy.rs         # COPY, MOVE
│   │   │   └── idle.rs         # IDLE (push)
│   │   ├── parser.rs           # IMAP protocol parser
│   │   └── response.rs         # IMAP response builder
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── maildir.rs          # Maildir format implementation
│   │   ├── db.rs               # SQLite metadata store
│   │   ├── models.rs           # Database models
│   │   └── migrations/         # SQL migrations
│   │       └── 001_init.sql
│   │
│   ├── security/
│   │   ├── mod.rs
│   │   ├── dkim.rs             # DKIM signing/verification
│   │   ├── spf.rs              # SPF checking
│   │   ├── dmarc.rs            # DMARC policy
│   │   ├── tls.rs              # TLS certificate management
│   │   └── ratelimit.rs        # Rate limiting
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs           # REST endpoints
│   │   ├── auth.rs             # JWT authentication
│   │   ├── handlers/           # Request handlers
│   │   │   ├── mailbox.rs
│   │   │   ├── message.rs
│   │   │   └── search.rs
│   │   └── models.rs           # API request/response types
│   │
│   └── utils/
│       ├── mod.rs
│       ├── dns.rs              # DNS helpers (MX lookup)
│       ├── email.rs            # Email validation
│       └── metrics.rs          # Prometheus metrics
│
├── tests/
│   ├── integration/
│   │   ├── smtp_test.rs
│   │   ├── imap_test.rs
│   │   └── api_test.rs
│   └── fixtures/
│       └── test_emails/
│
└── docs/
    ├── ARCHITECTURE.md
    ├── PROTOCOL_SMTP.md
    ├── PROTOCOL_IMAP.md
    └── API.md
```

## Spécifications fonctionnelles

### 1. SMTP Server (Réception)

**Ports**
- 25 : Réception inter-serveurs (non authentifié)
- 587 : Submission (authentification requise, STARTTLS obligatoire)

**Commandes supportées**
```
HELO/EHLO <domain>
MAIL FROM:<address>
RCPT TO:<address>
DATA
RSET
QUIT
STARTTLS
AUTH PLAIN/LOGIN
```

**Flow de réception**
```
1. Client connect → TLS handshake
2. EHLO → Server répond capabilities
3. AUTH → Vérification credentials
4. MAIL FROM → Validation sender
5. RCPT TO → Validation recipient (local domain ?)
6. DATA → Réception corps mail
7. SPF check → Accept/Reject
8. Store in Maildir + Update SQLite index
9. 250 OK
```

**Validation**
- SPF check sur sender domain
- Reverse DNS lookup
- Recipient existe localement ?
- Taille max mail (25MB par défaut)
- Rate limiting par IP/sender

**Storage**
```
/var/mail/maildir/
├── user@domain.com/
│   ├── new/        # Nouveaux mails
│   ├── cur/        # Mails lus
│   └── tmp/        # Temporaire
```

Nom fichier : `{timestamp}.{uniqid}.{hostname}`

### 2. SMTP Client (Envoi)

**Flow d'envoi**
```
1. API call POST /api/message/send
2. Add to send queue (SQLite)
3. Background worker picks up
4. MX lookup for recipient domain
5. Connect to remote SMTP
6. DKIM sign message
7. Send via SMTP
8. Handle response:
   - 250 OK → Mark sent
   - 4xx → Retry later
   - 5xx → Mark failed, bounce
```

**DKIM Signing**
```
Headers to sign:
- From, To, Subject, Date
- Message-ID, Content-Type

Signature:
- RSA 2048-bit
- SHA-256 hash
- Selector: "default._domainkey.domain.com"
```

**Queue management**
```sql
CREATE TABLE send_queue (
    id TEXT PRIMARY KEY,
    from_addr TEXT NOT NULL,
    to_addr TEXT NOT NULL,
    message_path TEXT NOT NULL,
    attempts INTEGER DEFAULT 0,
    max_attempts INTEGER DEFAULT 5,
    next_retry TIMESTAMP,
    status TEXT, -- pending, sending, sent, failed
    error TEXT,
    created_at TIMESTAMP,
    updated_at TIMESTAMP
);
```

**Retry policy**
- Attempt 1: Immediate
- Attempt 2: +5 min
- Attempt 3: +15 min
- Attempt 4: +1h
- Attempt 5: +4h
- After 5 attempts: Generate bounce mail

### 3. IMAP Server

**Commandes supportées**

**Authentication**
```
LOGIN <username> <password>
AUTHENTICATE PLAIN/LOGIN
```

**Mailbox operations**
```
LIST "" *                    # Liste dossiers
SELECT INBOX                 # Ouvre mailbox
EXAMINE INBOX                # Ouvre en read-only
CREATE <mailbox>             # Crée dossier
DELETE <mailbox>             # Supprime dossier
RENAME <old> <new>           # Renomme dossier
STATUS <mailbox> (MESSAGES UNSEEN)
```

**Message operations**
```
FETCH <set> (FLAGS BODY[])   # Récupère mails
STORE <set> +FLAGS (\Seen)   # Modifie flags
COPY <set> <mailbox>         # Copie mails
EXPUNGE                      # Supprime définitivement
SEARCH <criteria>            # Recherche
UID FETCH/STORE/SEARCH       # Versions UID
```

**IDLE (Push)**
```
IDLE
* OK Waiting for changes
[Client waits]
[Server detects new mail]
* 42 EXISTS
* 42 RECENT
DONE
```

**Flags supportés**
- `\Seen` : Lu
- `\Answered` : Répondu
- `\Flagged` : Marqué/Important
- `\Deleted` : Supprimé (expunge later)
- `\Draft` : Brouillon

**Search criteria**
```
ALL
UNSEEN
SEEN
FROM "sender@example.com"
TO "recipient@example.com"
SUBJECT "meeting"
BODY "urgent"
SINCE "1-Jan-2024"
BEFORE "31-Dec-2024"
OR <crit1> <crit2>
NOT <criteria>
```

### 4. REST API

**Endpoints**

```yaml
# Authentication
POST /api/auth/login
  Request: { email, password }
  Response: { token, refresh_token, expires_in }

POST /api/auth/refresh
  Request: { refresh_token }
  Response: { token, expires_in }

# Mailboxes
GET /api/mailboxes
  Response: [{ name, total, unread, path }]

POST /api/mailboxes
  Request: { name }
  Response: { id, name, created_at }

DELETE /api/mailboxes/:name

# Messages
GET /api/mailbox/:name/messages?limit=50&offset=0&unread=true
  Response: [{
    id, uid, subject, from, to, date,
    flags: ["\Seen"], preview, size
  }]

GET /api/message/:id
  Response: {
    id, uid, subject, from, to, cc, bcc, date,
    headers: {}, body_text, body_html, attachments: []
  }

POST /api/message/send
  Request: {
    to: ["user@example.com"],
    cc: [], bcc: [],
    subject, body_text, body_html,
    attachments: [{ filename, content_base64 }]
  }
  Response: { id, status: "queued" }

DELETE /api/message/:id

PATCH /api/message/:id/flags
  Request: { add: ["\Seen"], remove: [] }

POST /api/message/:id/move
  Request: { to_mailbox: "Archive" }

# Search
GET /api/search?q=urgent&mailbox=INBOX&unread=true
  Response: [{ id, subject, from, date, ... }]

# Stats
GET /api/stats
  Response: {
    total_messages, unread_messages,
    total_size_bytes, mailboxes_count
  }
```

**Authentication**
- JWT Bearer token
- Expiration: 1h
- Refresh token: 30 jours
- Claims: `{ sub: user_id, email, exp, iat }`

### 5. Configuration

```yaml
# config.yaml
server:
  domain: example.com          # Domaine principal
  hostname: mail.example.com   # FQDN du serveur

smtp:
  # Reception
  listen_addr: 0.0.0.0:25
  submission_addr: 0.0.0.0:587
  enable_tls: true
  tls_cert: /etc/mail/cert.pem
  tls_key: /etc/mail/key.pem
  
  # Envoi
  max_size_mb: 25
  queue_retry_interval: 300    # 5 min
  
  # Sécurité
  require_auth: true
  enable_spf: true
  reject_on_spf_fail: false    # Log only

imap:
  listen_addr: 0.0.0.0:993
  enable_tls: true
  tls_cert: /etc/mail/cert.pem
  tls_key: /etc/mail/key.pem
  idle_timeout: 1800           # 30 min

api:
  listen_addr: 0.0.0.0:8080
  jwt_secret: changeme         # À générer
  jwt_expiration: 3600         # 1h
  cors_origins: ["https://ui.example.com"]

storage:
  maildir_path: /var/mail/maildir
  database_url: sqlite:///var/mail/mail.db
  max_mailbox_size_gb: 10

dkim:
  enabled: true
  selector: default
  private_key_path: /etc/mail/dkim.key
  headers_to_sign:
    - From
    - To
    - Subject
    - Date
    - Message-ID

logging:
  level: info                  # trace, debug, info, warn, error
  format: json                 # json or pretty
  output: stdout               # stdout or /var/log/mail.log

security:
  rate_limit_per_ip: 100       # Mails/hour
  rate_limit_per_user: 1000    # Mails/hour
  max_recipients: 50
  enable_fail2ban: true

users:
  - email: admin@example.com
    password_hash: $argon2id$...
    quota_gb: 10
  - email: user@example.com
    password_hash: $argon2id$...
    quota_gb: 5
```

## Sécurité

### Priorités
1. **TLS obligatoire** partout (SMTP submission, IMAP, API)
2. **SPF checking** sur réception
3. **DKIM signing** sur envoi
4. **Rate limiting** agressif
5. **Argon2** pour hash passwords
6. **JWT** pour API auth
7. **Input validation** stricte

### Threat model
- **Spam/Phishing** : SPF + rate limiting + content filtering futur
- **Brute force** : Rate limit login + fail2ban
- **Man-in-the-middle** : TLS obligatoire
- **Open relay** : Auth requise pour submission
- **Injection** : Validation stricte email addresses
- **DoS** : Rate limiting + max connections

### Checklist sécurité
- [ ] Pas de credentials en clair dans logs
- [ ] Validation taille mails (max 25MB)
- [ ] Timeout sur toutes connexions (30s)
- [ ] Pas de path traversal (mailbox names)
- [ ] Headers sanitization
- [ ] SQL injection protection (sqlx prepared statements)
- [ ] CORS policy stricte sur API

## Tests

### Tests unitaires
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_smtp_parse_mail_from() { }
    
    #[tokio::test]
    async fn test_spf_check() { }
    
    #[tokio::test]
    async fn test_dkim_sign() { }
    
    #[test]
    fn test_maildir_path_generation() { }
}
```

### Tests d'intégration
```rust
// tests/integration/smtp_test.rs
#[tokio::test]
async fn test_receive_mail_via_smtp() {
    // Start server
    // Connect via SMTP client
    // Send mail
    // Verify stored in maildir
}

#[tokio::test]
async fn test_send_mail_via_api() {
    // POST /api/message/send
    // Check queue
    // Verify DKIM signature
}
```

### Tests manuels
```bash
# Test SMTP reception
swaks --to user@domain.test --from sender@example.com \
      --server localhost:2525 --tls

# Test IMAP
openssl s_client -connect localhost:993 -crlf
a1 LOGIN user@domain.test password
a2 SELECT INBOX
a3 FETCH 1 BODY[]
a4 LOGOUT

# Test API
curl -X POST http://localhost:8080/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email":"user@domain.test","password":"pass"}'
```

### Benchmarks
- SMTP throughput : >1000 mails/sec
- IMAP concurrent sessions : >1000
- API latency : <50ms (p95)
- Storage : <1ms write latency

## Observabilité

### Logs structurés (tracing)
```rust
tracing::info!(
    email = %recipient,
    size = mail_size,
    spf_result = ?spf,
    "Mail received"
);

tracing::error!(
    error = %e,
    remote_ip = %ip,
    "SMTP connection failed"
);
```

### Métriques Prometheus
```
# Compteurs
mail_received_total{result="accepted|rejected"}
mail_sent_total{result="success|failed"}
imap_sessions_total
api_requests_total{method, endpoint, status}

# Gauges
smtp_active_connections
imap_active_sessions
queue_size
storage_size_bytes

# Histograms
smtp_processing_duration_seconds
imap_fetch_duration_seconds
api_request_duration_seconds
```

### Health checks
```
GET /health
Response: {
  status: "healthy",
  smtp: { listening: true, active_connections: 5 },
  imap: { listening: true, active_sessions: 12 },
  api: { listening: true },
  storage: { available_gb: 45.2 },
  queue: { pending: 3 }
}
```

## Déploiement

### Docker
```dockerfile
FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.19
RUN apk add --no-cache ca-certificates
COPY --from=builder /app/target/release/mail-rs /usr/local/bin/
EXPOSE 25 587 993 8080
CMD ["mail-rs", "--config", "/etc/mail/config.yaml"]
```

### docker-compose.yml
```yaml
version: '3.8'
services:
  mail:
    build: .
    ports:
      - "25:25"
      - "587:587"
      - "993:993"
      - "8080:8080"
    volumes:
      - ./config.yaml:/etc/mail/config.yaml:ro
      - ./certs:/etc/mail/certs:ro
      - mail-data:/var/mail
    environment:
      - RUST_LOG=info
    restart: unless-stopped

volumes:
  mail-data:
```

## Roadmap

### MVP (Phase 1) - 10-12 semaines
- [x] SMTP receiver fonctionnel
- [x] SMTP sender + queue
- [x] IMAP read-only
- [x] IMAP full (STORE, DELETE)
- [x] REST API
- [x] SPF + DKIM
- [x] Docker

### Post-MVP
- [ ] WebUI admin (gestion users, domains, aliases)
- [ ] DMARC reporting
- [ ] Sieve filtering (rules)
- [ ] Full-text search (Tantivy)
- [ ] S/MIME support
- [ ] Cluster mode (multi-nodes)
- [ ] Backup/restore automatique

## Métriques de succès

**Technique**
- ✅ Délivrabilité >95% vers Gmail/Outlook
- ✅ Aucun CVE critique
- ✅ Uptime >99.9%
- ✅ Latency API <50ms p95

**Business (si open source)**
- ✅ 100+ stars GitHub en 3 mois
- ✅ 10+ installations production
- ✅ 5+ contributors

## Notes d'implémentation

### Priorités de développement
1. **SMTP receiver** : C'est la base, commencer ici
2. **Storage + Maildir** : Nécessaire pour receiver
3. **SMTP sender** : Peut envoyer des mails
4. **IMAP** : Accès aux mails reçus
5. **API** : Interface pour MCP
6. **Sécurité** : Hardening

### Pièges à éviter
- ❌ Ne pas implémenter POP3 (legacy, pas nécessaire)
- ❌ Ne pas faire de webmail dans mail-rs (séparation concerns)
- ❌ Ne pas gérer multi-domain dans MVP (un seul domain suffit)
- ❌ Ne pas implémenter antispam avancé dans MVP (futur)

### Décisions architecturales

**Pourquoi Maildir et pas mbox ?**
- Pas de file lock issues
- Un mail = un fichier (simple, atomique)
- Compatible avec autres outils (offlineimap, etc.)

**Pourquoi SQLite et pas PostgreSQL ?**
- MVP = single server
- Simplicité déploiement
- Performance suffisante (<10k mails/jour)
- Peut migrer PostgreSQL plus tard si besoin

**Pourquoi REST API et pas gRPC ?**
- Plus simple pour MCP server
- Debug facile (curl)
- Standards HTTP (cache, auth, etc.)

## Références

### RFCs à implémenter
- RFC 5321 : SMTP
- RFC 3501 : IMAP4rev1
- RFC 6376 : DKIM
- RFC 7208 : SPF
- RFC 7489 : DMARC
- RFC 5322 : Internet Message Format

### Ressources
- https://github.com/stalwartlabs/mail-server (inspiration)
- https://doc.dovecot.org/configuration_manual/protocols/imap/
- https://www.postfix.org/documentation.html
- https://cr.yp.to/proto/maildir.html

### Outils de test
- swaks : SMTP testing
- imaptest : IMAP testing  
- mail-tester.com : Deliverability check
- mxtoolbox.com : DNS/SPF/DKIM check
