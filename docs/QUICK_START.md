# Quick Start Guide - Démarrage mail-rs

## Objectif

Ce guide te permet de démarrer le développement de `mail-rs` immédiatement, en suivant une approche incrémentale sprint par sprint.

## Prérequis

- Rust 1.75+ installé (`rustup`)
- Git
- Un domaine de test (optionnel au début)
- Docker (pour tests d'intégration)

## Setup Initial (Jour 1)

### 1. Créer le projet

```bash
# Créer workspace Rust
mkdir communication-suite
cd communication-suite

# Créer projet mail-rs
cargo new mail-rs --bin
cd mail-rs
```

### 2. Setup Cargo.toml

Copier les dépendances depuis `mail-rs_CLAUDE.md` section "Stack Rust" :

```toml
[package]
name = "mail-rs"
version = "0.1.0"
edition = "2021"

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

# ... (voir mail-rs_CLAUDE.md pour liste complète)
```

### 3. Créer structure de dossiers

```bash
mkdir -p src/{smtp,imap,storage,security,api,utils}
mkdir -p tests/integration
mkdir -p docs
```

### 4. Configuration de base

```bash
# Créer fichier config
cat > config.yaml << EOF
server:
  domain: test.local
  hostname: mail.test.local

smtp:
  listen_addr: 0.0.0.0:2525  # Port non-privilégié pour tests
  
logging:
  level: debug
  format: pretty
EOF
```

## Sprint 1 : SMTP Receiver Minimal (Semaine 1-2)

### Objectif

Accepter une connexion SMTP et recevoir un mail basique (sans TLS, sans auth).

### Étapes

**Jour 1-2 : Setup serveur TCP**

```rust
// src/main.rs
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let listener = TcpListener::bind("0.0.0.0:2525").await?;
    println!("SMTP server listening on port 2525");
    
    loop {
        let (socket, addr) = listener.accept().await?;
        println!("New connection from {}", addr);
        
        tokio::spawn(async move {
            handle_connection(socket).await;
        });
    }
}

async fn handle_connection(socket: TcpStream) {
    // TODO: Implémenter
}
```

**Jour 3-5 : Parser SMTP commands**

```rust
// src/smtp/commands.rs
pub enum SmtpCommand {
    Helo(String),
    Ehlo(String),
    MailFrom(String),
    RcptTo(String),
    Data,
    Quit,
}

pub fn parse_command(line: &str) -> Result<SmtpCommand> {
    // TODO: Implémenter parsing
}
```

**Jour 6-8 : State machine SMTP**

```rust
// src/smtp/session.rs
enum SmtpState {
    Fresh,
    Greeted,
    MailFrom,
    RcptTo,
    Data,
}

pub struct SmtpSession {
    state: SmtpState,
    from: Option<String>,
    to: Vec<String>,
    data: Vec<u8>,
}

impl SmtpSession {
    pub async fn handle_command(&mut self, cmd: SmtpCommand) -> String {
        match (self.state, cmd) {
            (SmtpState::Fresh, SmtpCommand::Ehlo(domain)) => {
                self.state = SmtpState::Greeted;
                format!("250-mail.test.local\r\n250 HELP\r\n")
            }
            // ... autres transitions
        }
    }
}
```

**Jour 9-10 : Stocker mail en Maildir**

```rust
// src/storage/maildir.rs
pub async fn store_mail(
    recipient: &str,
    data: &[u8],
) -> Result<String> {
    let path = format!("/var/mail/maildir/{}/new", recipient);
    let filename = generate_maildir_filename();
    
    tokio::fs::write(
        format!("{}/{}", path, filename),
        data
    ).await?;
    
    Ok(filename)
}
```

### Test Sprint 1

```bash
# Tester avec swaks
swaks --to test@test.local \
      --from sender@example.com \
      --server localhost:2525 \
      --body "Hello from swaks"

# Vérifier fichier créé
ls -la /var/mail/maildir/test@test.local/new/
```

### Critère de succès ✅

- [ ] Serveur accepte connexions TCP
- [ ] Parse commandes SMTP basiques
- [ ] Reçoit mail complet (MAIL FROM → RCPT TO → DATA)
- [ ] Stocke mail en Maildir
- [ ] Tests swaks passent

## Sprint 2 : SMTP Sender (Semaine 3-4)

### Objectif

Envoyer un mail sortant vers un serveur SMTP externe.

### Points clés

```rust
// src/smtp/client.rs
pub async fn send_mail(
    from: &str,
    to: &str,
    data: &[u8],
) -> Result<()> {
    // 1. MX lookup
    let mx_records = lookup_mx(to.split('@').nth(1).unwrap()).await?;
    
    // 2. Connect to best MX
    let stream = TcpStream::connect(&mx_records[0]).await?;
    
    // 3. SMTP dialogue
    // EHLO → MAIL FROM → RCPT TO → DATA
    
    // 4. Close
    Ok(())
}
```

### Test Sprint 2

```bash
# Créer fichier de test
cat > /tmp/test.eml << EOF
From: test@yourdomain.com
To: your-real-email@gmail.com
Subject: Test from mail-rs

This is a test email.
EOF

# Envoyer via mail-rs
cargo run -- send /tmp/test.eml

# Vérifier réception dans Gmail
```

### Critère de succès ✅

- [ ] MX lookup fonctionne
- [ ] Connexion SMTP sortante OK
- [ ] Mail reçu dans Gmail (inbox ou spam)

## Sprint 3 : IMAP Read-only (Semaine 5-6)

### Objectif

Lire ses mails via un client IMAP (Thunderbird).

### Points clés

```rust
// src/imap/server.rs
pub async fn handle_imap_connection(socket: TcpStream) {
    // State machine IMAP
    // * OK Server ready
    // a1 LOGIN user pass
    // a2 SELECT INBOX
    // a3 FETCH 1:* (FLAGS BODY[])
}
```

### Test Sprint 3

```bash
# Tester avec openssl
openssl s_client -connect localhost:1993 -crlf
a1 LOGIN test@test.local password
a2 SELECT INBOX
a3 FETCH 1 BODY[]
a4 LOGOUT
```

### Critère de succès ✅

- [ ] Thunderbird peut se connecter
- [ ] Liste les mails dans INBOX
- [ ] Peut lire un mail complet

## Sprint 4-6 : Complétion MVP

- Sprint 4 : IMAP STORE, DELETE, SEARCH
- Sprint 5 : SPF, DKIM, API REST
- Sprint 6 : Docker, docs, tests

## Conseils Pratiques

### Debug

```rust
// Activer logs détaillés
RUST_LOG=debug cargo run
```

### Tests unitaires

```bash
# Lancer tests
cargo test

# Lancer tests spécifiques
cargo test smtp_parser
```

### Structure commits Git

```
feat(smtp): add EHLO command parsing
fix(imap): handle empty mailbox
docs: update SMTP protocol documentation
test: add integration test for mail reception
```

### Resources

- RFC 5321 (SMTP) : https://www.rfc-editor.org/rfc/rfc5321
- RFC 3501 (IMAP) : https://www.rfc-editor.org/rfc/rfc3501
- Maildir spec : https://cr.yp.to/proto/maildir.html

## Checklist Globale mail-rs

### Phase 1 : SMTP
- [ ] Receiver (port 25/587)
- [ ] Sender (client SMTP)
- [ ] Queue + retry
- [ ] TLS support
- [ ] AUTH support

### Phase 2 : IMAP
- [ ] Basic commands (LOGIN, SELECT, FETCH)
- [ ] STORE (flags)
- [ ] SEARCH
- [ ] IDLE (push)

### Phase 3 : Sécurité
- [ ] SPF checking
- [ ] DKIM signing
- [ ] Rate limiting
- [ ] Fail2ban logs

### Phase 4 : Production
- [ ] REST API
- [ ] Docker image
- [ ] Documentation
- [ ] Tests d'intégration

## Points de Blocage Fréquents

### "Je ne reçois pas de mails depuis l'extérieur"

**Causes** :
- Port 25 bloqué par hébergeur → Utiliser port 587 avec auth
- Pas de DNS MX record → Ajouter MX record
- Firewall → Ouvrir ports 25/587/993

### "Mes mails finissent en spam"

**Solutions** :
- Configurer SPF record
- Signer avec DKIM
- DMARC policy
- PTR record (reverse DNS)
- Warmup IP (envoyer graduellement)

### "Performance dégradée"

**Optimisations** :
- Async I/O (tokio)
- Connection pooling
- Caching DNS lookups
- Index database (SQLite)

## Next Steps

Une fois `mail-rs` MVP terminé :

1. Lire `ai-runtime_CLAUDE.md`
2. Lire `mcp-mail-server_CLAUDE.md`
3. Commencer intégration AI

## Questions ?

Relire les sections pertinentes dans `mail-rs_CLAUDE.md` :
- Architecture technique
- Spécifications fonctionnelles
- Notes d'implémentation

---

**Bon courage ! 🚀**

Premier objectif : Sprint 1 terminé en 2 semaines.
