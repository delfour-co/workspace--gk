# GK - AI-Native Communication Suite

## Vue d'ensemble du projet

**GK** est une suite de communication self-hosted avec interface AI-native conversationnelle. Alternative à Google Workspace où l'utilisateur dialogue avec un assistant IA local (via MCP) pour gérer tous les services au lieu de naviguer entre plusieurs interfaces.

**Différenciateur clé** : Interface conversationnelle propulsée par un LLM local via le protocole MCP (Model Context Protocol).

## Architecture

```
Internet → proxy-rs → [web-ui, mail-rs, chat-rs] → ai-runtime → [mcp-mail, mcp-chat]
                                                      ↓
                                                   dav-rs
```

## Composants (7 total)

| Composant | Statut | Priorité | Description |
|-----------|--------|----------|-------------|
| **mail-rs** | 🟢 80% | P0 | SMTP/IMAP mail server |
| **ai-runtime** | 🟡 Début | P0 | LLM runtime + MCP orchestrator |
| **proxy-rs** | ⚪ 0% | P1 | Reverse proxy SSL/TLS |
| **mcp-mail-server** | ⚪ 0% | P0 | MCP server pour mail |
| **web-ui** | ⚪ 0% | P1 | Interface conversationnelle |
| **chat-rs** | ⚪ 0% | P2 | Messagerie temps réel |
| **dav-rs** | ⚪ 0% | P2 | CalDAV/CardDAV |

## État actuel

### mail-rs (80% complet)
- ✅ SMTP Receiver (Sprint 1)
- ✅ SMTP Sender + Queue (Sprint 2)
- ✅ TLS + SMTP AUTH (Sprint 3)
- ✅ SPF/DKIM validation (Sprint 4)
- ⏳ IMAP (Sprint 5-6)
- ⏳ API REST (Sprint 7)

**Tests** : 78/78 ✅

### ai-runtime (Début)
- ✅ Structure MCP (JSON-RPC 2.0)
- ✅ Mock LLM avec détection d'intentions
- ✅ HTTP API /chat
- ⏳ Intégration LLM réel
- ⏳ Registry MCP servers

## Stack technique

- **Langage** : 100% Rust
- **Runtime** : Tokio (async/await)
- **HTTP** : Axum
- **LLM** : Mistral 7B ou Llama 3.1 8B (quantifié)
- **Bases de données** : SQLite (mail, ai, dav) + PostgreSQL (chat)
- **Frontend** : HTMX + Alpine.js + Tailwind CSS
- **Protocoles** : SMTP, IMAP, WebSocket, CalDAV, CardDAV, MCP
- **Déploiement** : Docker + docker-compose

## Documentation

Toute la documentation est dans `docs/` :

- `README.md` - Vue d'ensemble et architecture
- `INDEX.md` - Index de toutes les spécifications
- `QUICK_START.md` - Guide de démarrage rapide
- `PROJECT_STATUS.md` - État d'avancement détaillé
- `*_CLAUDE.md` - Spécifications techniques de chaque composant

## Agents Claude Code

Le fichier `.claude/agents.md` définit des agents spécialisés pour aider au développement :

### Agents par composant
- **mail-rs Agent** - Expert SMTP/IMAP
- **ai-runtime Agent** - Expert LLM et MCP
- **proxy-rs Agent** - Expert reverse proxy
- **mcp-mail-server Agent** - Expert MCP mail
- **web-ui Agent** - Expert interface utilisateur
- **chat-rs Agent** - Expert messagerie
- **dav-rs Agent** - Expert CalDAV/CardDAV

### Agents par domaine
- **Security Agent** - Expert sécurité
- **Testing Agent** - Expert tests
- **Documentation Agent** - Expert documentation
- **Architecture Agent** - Expert architecture
- **Debug Agent** - Expert debugging
- **Refactoring Agent** - Expert refactoring

### Agents par tâche
- **Development Agent** - Assistant développement général
- **Integration Agent** - Expert intégration
- **DevOps Agent** - Expert déploiement

**Utilisation** : Référencer un agent dans vos prompts pour obtenir une expertise spécialisée.
Exemple : "En tant que mail-rs Agent, implémente la commande IMAP FETCH"

## Structure du workspace

```
gk/
├── mail-rs/          # Serveur mail SMTP/IMAP
├── ai-runtime/       # LLM + MCP orchestrator
├── proxy-rs/         # Reverse proxy (à créer)
├── mcp-mail-server/  # MCP server mail (à créer)
├── web-ui/           # Interface web (à créer)
├── chat-rs/          # Messagerie (à créer)
├── dav-rs/           # CalDAV/CardDAV (à créer)
├── docs/             # Documentation complète
└── Cargo.toml        # Workspace Rust
```

## Conventions

### Commits Git
Format : `type(scope): description`

Types : `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `chore`

Exemples :
- `feat(mail-rs): add IMAP server`
- `fix(smtp): handle timeout correctly`
- `docs: update architecture diagram`

### Code Rust
- Format avec `cargo fmt`
- Lint avec `cargo clippy -- -D warnings`
- Documentation rustdoc pour toutes les APIs publiques
- Tests unitaires + intégration
- Gestion d'erreurs avec `thiserror`

### Sécurité
- Validation de tous les inputs externes
- Timeouts sur toutes les opérations I/O
- Limites de ressources (taille, nombre)
- Pas de `unsafe` sauf si absolument nécessaire

## Roadmap

### MVP Minimal (Mail + AI + UI) - ~20 semaines
1. ✅ Mail SMTP (Sprint 1-4) - 7 semaines
2. ⏳ Mail IMAP (Sprint 5-6) - 4 semaines
3. ⏳ AI Runtime + MCP (Sprint 7-9) - 3 semaines
4. ⏳ Web UI (Sprint 10-13) - 4 semaines

### MVP Complet (+ Chat + DAV) - ~26-30 semaines
+ Chat (4 semaines)
+ DAV (3 semaines)

## Points importants pour Claude

1. **Tout le code est en Rust** - Pas de code dans d'autres langages
2. **Focus sécurité** - Validation, timeouts, limites partout
3. **Tests obligatoires** - Chaque fonctionnalité doit avoir des tests
4. **Documentation complète** - Les specs détaillées sont dans `docs/*_CLAUDE.md`
5. **Architecture modulaire** - Chaque composant est indépendant
6. **AI-native est le différenciateur** - L'interface conversationnelle est la clé

## Fichiers de référence

Pour comprendre un composant, lire :
1. `docs/[composant]_CLAUDE.md` - Spécifications complètes
2. Code source dans `[composant]/src/`
3. Tests dans `[composant]/tests/`

## Liens utiles

- RFC 5321 (SMTP) : https://www.rfc-editor.org/rfc/rfc5321
- RFC 3501 (IMAP) : https://www.rfc-editor.org/rfc/rfc3501
- MCP Protocol : https://modelcontextprotocol.io/
- Tokio : https://tokio.rs/
- Axum : https://github.com/tokio-rs/axum

