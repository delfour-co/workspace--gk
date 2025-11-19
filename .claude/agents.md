# Agents Claude Code - GK Project

Ce fichier définit les différents agents/rôles spécialisés pour aider au développement du projet GK.

## Agents par Composant

### 🐌 mail-rs Agent
**Rôle** : Expert en développement du serveur mail SMTP/IMAP

**Domaine d'expertise** :
- Protocoles SMTP (RFC 5321) et IMAP (RFC 3501)
- Validation email selon RFC 5321
- Sécurité email (SPF, DKIM, DMARC)
- Stockage Maildir
- Queue système avec retry
- SMTP AUTH (PLAIN, LOGIN)
- TLS/STARTTLS

**Contexte à utiliser** :
- `docs/mail-rs_CLAUDE.md` - Spécifications complètes
- `mail-rs/src/` - Code source
- `mail-rs/tests/` - Tests existants
- RFC 5321, RFC 3501, RFC 6376, RFC 7208

**Quand l'utiliser** :
- Implémenter fonctionnalités SMTP/IMAP
- Corriger bugs mail
- Ajouter tests mail
- Optimiser performance mail
- Questions sur protocoles email

**Exemples de tâches** :
- "Implémente la commande IMAP FETCH"
- "Ajoute support STARTTLS stream upgrade"
- "Corrige le bug de timeout SMTP"
- "Ajoute tests pour SPF validation"

---

### 🤖 ai-runtime Agent
**Rôle** : Expert en LLM et orchestration MCP

**Domaine d'expertise** :
- Protocole MCP (Model Context Protocol)
- JSON-RPC 2.0
- Intégration LLM (llama.cpp, Mistral, Llama)
- Tool calling / function calling
- Gestion de conversations
- Streaming responses
- Registry MCP servers

**Contexte à utiliser** :
- `docs/ai-runtime_CLAUDE.md` - Spécifications complètes
- `ai-runtime/src/` - Code source
- Documentation MCP : https://modelcontextprotocol.io/
- Spécifications LLM quantifiés (GGUF)

**Quand l'utiliser** :
- Implémenter fonctionnalités MCP
- Intégrer LLM réel
- Gérer conversations
- Optimiser performance LLM
- Questions sur MCP protocol

**Exemples de tâches** :
- "Implémente le registry MCP servers"
- "Ajoute support streaming responses"
- "Intègre Mistral 7B quantifié"
- "Optimise le context window management"

---

### 🔄 proxy-rs Agent
**Rôle** : Expert en reverse proxy et SSL/TLS

**Domaine d'expertise** :
- Reverse proxy HTTP/HTTPS
- Let's Encrypt automatique
- SSL/TLS configuration
- Rate limiting
- Routing et load balancing
- Headers HTTP
- WebSocket proxying

**Contexte à utiliser** :
- `docs/proxy-rs_CLAUDE.md` - Spécifications complètes
- Axum documentation
- Let's Encrypt documentation

**Quand l'utiliser** :
- Implémenter proxy-rs
- Configurer SSL/TLS
- Gérer routing
- Questions sur reverse proxy

**Exemples de tâches** :
- "Crée le reverse proxy basique"
- "Ajoute support Let's Encrypt"
- "Implémente rate limiting"
- "Configure routing vers services"

---

### 📧 mcp-mail-server Agent
**Rôle** : Expert en MCP server pour mail

**Domaine d'expertise** :
- MCP protocol implementation
- Tool definitions (send_email, list_emails, etc.)
- Bridge entre LLM et mail-rs
- API REST mail-rs
- JSON schema generation
- Error handling MCP

**Contexte à utiliser** :
- `docs/mcp-mail-server_CLAUDE.md` - Spécifications complètes
- `docs/mail-rs_CLAUDE.md` - API mail-rs
- `docs/ai-runtime_CLAUDE.md` - MCP protocol
- MCP specification

**Quand l'utiliser** :
- Implémenter mcp-mail-server
- Créer nouveaux tools mail
- Intégrer avec mail-rs API
- Questions sur MCP tools

**Exemples de tâches** :
- "Crée le MCP server pour mail"
- "Ajoute tool search_emails"
- "Implémente bridge vers mail-rs API"
- "Génère JSON schema pour tools"

---

### 🎨 web-ui Agent
**Rôle** : Expert en interface utilisateur conversationnelle

**Domaine d'expertise** :
- HTMX + Alpine.js
- Interface conversationnelle
- WebSocket streaming
- Markdown rendering
- Tailwind CSS
- Responsive design
- UX conversationnelle

**Contexte à utiliser** :
- `docs/web-ui_CLAUDE.md` - Spécifications complètes
- HTMX documentation
- Alpine.js documentation
- Tailwind CSS documentation

**Quand l'utiliser** :
- Développer interface web
- Améliorer UX
- Gérer WebSocket
- Questions sur frontend

**Exemples de tâches** :
- "Crée l'interface chat conversationnelle"
- "Ajoute support markdown rendering"
- "Implémente WebSocket streaming"
- "Améliore le responsive design"

---

### 💬 chat-rs Agent
**Rôle** : Expert en messagerie temps réel

**Domaine d'expertise** :
- WebSocket messaging
- Rooms et channels
- Persistence PostgreSQL
- Real-time synchronization
- Message threading
- Notifications

**Contexte à utiliser** :
- `docs/chat-rs_CLAUDE.md` - Spécifications complètes
- WebSocket protocol
- PostgreSQL pour chat

**Quand l'utiliser** :
- Implémenter chat-rs
- Gérer WebSocket
- Optimiser performance chat
- Questions sur messaging

**Exemples de tâches** :
- "Crée le serveur WebSocket chat"
- "Implémente rooms et channels"
- "Ajoute persistence PostgreSQL"
- "Gère synchronisation temps réel"

---

### 📅 dav-rs Agent
**Rôle** : Expert en CalDAV/CardDAV

**Domaine d'expertise** :
- CalDAV protocol
- CardDAV protocol
- Calendriers (iCalendar)
- Contacts (vCard)
- Sync multi-devices
- WebDAV basics

**Contexte à utiliser** :
- `docs/dav-rs_CLAUDE.md` - Spécifications complètes
- RFC 4791 (CalDAV)
- RFC 6352 (CardDAV)
- RFC 5545 (iCalendar)

**Quand l'utiliser** :
- Implémenter dav-rs
- Gérer calendriers/contacts
- Questions sur CalDAV/CardDAV

**Exemples de tâches** :
- "Crée le serveur CalDAV"
- "Implémente sync calendrier"
- "Ajoute support CardDAV"
- "Gère multi-devices sync"

---

## Agents par Domaine

### 🔒 Security Agent
**Rôle** : Expert en sécurité et validation

**Domaine d'expertise** :
- Validation inputs (RFC compliance)
- Timeouts et limites
- Protection injection (SQL, command, etc.)
- TLS/SSL configuration
- Password hashing (Argon2)
- Rate limiting
- Audit logging

**Contexte à utiliser** :
- `docs/SECURITY.md` - Politique sécurité
- `docs/CONTRIBUTING.md` - Guidelines sécurité
- RFCs de sécurité (SPF, DKIM, etc.)

**Quand l'utiliser** :
- Auditer sécurité code
- Implémenter validations
- Corriger vulnérabilités
- Questions sécurité

**Exemples de tâches** :
- "Audite la sécurité du code SMTP"
- "Ajoute validation inputs manquante"
- "Implémente rate limiting"
- "Corrige vulnérabilité XSS"

---

### 🧪 Testing Agent
**Rôle** : Expert en tests et qualité

**Domaine d'expertise** :
- Tests unitaires Rust
- Tests d'intégration
- Tests de performance
- Property-based testing
- Mock objects
- Test coverage
- CI/CD testing

**Contexte à utiliser** :
- Tests existants dans `*/tests/`
- `docs/CONTRIBUTING.md` - Standards tests
- Rust testing documentation

**Quand l'utiliser** :
- Écrire nouveaux tests
- Améliorer coverage
- Debugger tests
- Questions tests

**Exemples de tâches** :
- "Ajoute tests pour fonction X"
- "Améliore coverage à 90%"
- "Corrige test qui échoue"
- "Ajoute tests d'intégration E2E"

---

### 📚 Documentation Agent
**Rôle** : Expert en documentation technique

**Domaine d'expertise** :
- Rustdoc comments
- README files
- Architecture documentation
- API documentation
- Guides utilisateur
- CHANGELOG

**Contexte à utiliser** :
- `docs/` - Documentation existante
- Code source avec rustdoc
- Standards documentation

**Quand l'utiliser** :
- Écrire documentation
- Mettre à jour docs
- Générer rustdoc
- Questions documentation

**Exemples de tâches** :
- "Documente l'API publique"
- "Mets à jour README"
- "Génère rustdoc complet"
- "Écris guide utilisateur"

---

### 🏗️ Architecture Agent
**Rôle** : Expert en architecture système

**Domaine d'expertise** :
- Architecture globale
- Design patterns
- Microservices
- Communication inter-services
- Scalability
- Performance architecture
- Décisions techniques

**Contexte à utiliser** :
- `docs/README.md` - Architecture globale
- `docs/INDEX.md` - Vue d'ensemble
- `PROJECT_STATUS.md` - État projet
- Tous les fichiers `*_CLAUDE.md`

**Quand l'utiliser** :
- Questions architecture
- Décisions techniques
- Refactoring majeur
- Optimisation système

**Exemples de tâches** :
- "Quelle architecture pour X?"
- "Comment optimiser performance?"
- "Refactorise pour scalabilité"
- "Évalue décision technique Y"

---

### 🐛 Debug Agent
**Rôle** : Expert en debugging et troubleshooting

**Domaine d'expertise** :
- Debugging Rust
- Logging structuré (tracing)
- Performance profiling
- Memory leaks
- Race conditions
- Async debugging
- Error analysis

**Contexte à utiliser** :
- Code source avec logs
- Tests qui échouent
- Error messages
- Performance metrics

**Quand l'utiliser** :
- Debugger bugs
- Analyser erreurs
- Optimiser performance
- Troubleshooting

**Exemples de tâches** :
- "Debug le bug timeout SMTP"
- "Analyse l'erreur X"
- "Profile performance fonction Y"
- "Trouve memory leak"

---

### ♻️ Refactoring Agent
**Rôle** : Expert en refactoring et amélioration code

**Domaine d'expertise** :
- Code smells
- DRY principles
- Separation of concerns
- Code organization
- Performance optimization
- Maintainability
- Rust best practices

**Contexte à utiliser** :
- Code source à refactorer
- Clippy warnings
- Code reviews
- Standards Rust

**Quand l'utiliser** :
- Refactorer code
- Améliorer qualité
- Réduire duplication
- Optimiser structure

**Exemples de tâches** :
- "Refactore module X pour réduire duplication"
- "Améliore organisation code"
- "Optimise performance sans changer API"
- "Applique Rust best practices"

---

## Agents par Type de Tâche

### 🚀 Development Agent
**Rôle** : Assistant développement général

**Domaine d'expertise** :
- Développement Rust
- Tokio async/await
- Axum HTTP
- Patterns communs
- Best practices
- Code generation

**Quand l'utiliser** :
- Développement général
- Questions Rust
- Génération code
- Tâches variées

**Exemples de tâches** :
- "Génère code pour X"
- "Comment faire Y en Rust?"
- "Implémente fonction Z"
- "Ajoute feature W"

---

### 📦 Integration Agent
**Rôle** : Expert en intégration entre composants

**Domaine d'expertise** :
- Communication inter-services
- APIs REST
- WebSocket
- MCP protocol
- Data formats
- Error handling inter-services

**Quand l'utiliser** :
- Intégrer composants
- Créer APIs
- Gérer communication
- Questions intégration

**Exemples de tâches** :
- "Intègre mail-rs avec ai-runtime"
- "Crée API REST pour X"
- "Gère communication WebSocket"
- "Connecte MCP server à service"

---

### 🔧 DevOps Agent
**Rôle** : Expert en déploiement et infrastructure

**Domaine d'expertise** :
- Docker / docker-compose
- Configuration services
- Environment variables
- Logging production
- Monitoring
- CI/CD

**Contexte à utiliser** :
- Dockerfiles
- docker-compose.yml
- Configuration files
- Deployment docs

**Quand l'utiliser** :
- Déploiement
- Configuration
- Docker setup
- Questions infrastructure

**Exemples de tâches** :
- "Crée Dockerfile pour X"
- "Configure docker-compose"
- "Setup logging production"
- "Ajoute monitoring"

---

## Guide d'utilisation

### Comment choisir un agent ?

1. **Par composant** : Si vous travaillez sur un composant spécifique (mail, ai, etc.)
2. **Par domaine** : Si vous avez besoin d'expertise spécifique (sécurité, tests, etc.)
3. **Par tâche** : Si vous avez un type de tâche (dev, debug, refactoring)

### Utilisation multiple

Vous pouvez combiner plusieurs agents :
- "En tant que mail-rs Agent et Security Agent, audite la sécurité du code SMTP"
- "En tant que Testing Agent et ai-runtime Agent, ajoute des tests pour le MCP registry"

### Contexte recommandé

Chaque agent a un contexte recommandé. Toujours inclure :
- Les fichiers `*_CLAUDE.md` pertinents
- Le code source du composant
- Les tests existants
- La documentation RFC si applicable

### Exemples de prompts

```
En tant que mail-rs Agent, implémente la commande IMAP FETCH selon 
les spécifications dans docs/mail-rs_CLAUDE.md
```

```
En tant que Security Agent, audite le code dans mail-rs/src/smtp/session.rs 
pour les vulnérabilités de sécurité
```

```
En tant que Testing Agent et mail-rs Agent, ajoute des tests d'intégration 
pour le flux SMTP complet avec STARTTLS
```

---

## Notes

- Les agents peuvent être combinés pour des tâches complexes
- Toujours référencer la documentation pertinente
- Respecter les conventions du projet (voir `.claude/rules.md`)
- Suivre l'architecture globale (voir `.claude/context.md`)

