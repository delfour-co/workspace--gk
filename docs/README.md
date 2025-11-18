# Projet Self-Hosted Communication Suite - Architecture Complète

## Vue d'ensemble

Ce projet vise à créer une **alternative self-hosted à Google Workspace** avec une interface **AI-native conversationnelle**. Au lieu de cliquer dans des interfaces séparées, l'utilisateur dialogue avec un assistant qui pilote tous les services.

**Différenciation clé** : Interface conversationnelle propulsée par un LLM local via le protocole MCP (Model Context Protocol).

## Architecture Globale

```
┌─────────────────────────────────────────────────────────────┐
│                        Internet                             │
└──────────────────────┬──────────────────────────────────────┘
                       │
                       ▼
              ┌────────────────┐
              │   proxy-rs     │  Reverse Proxy (SSL/TLS)
              │  (Port 80/443) │
              └────────┬───────┘
                       │
        ┌──────────────┼──────────────┐
        │              │              │
        ▼              ▼              ▼
  ┌──────────┐  ┌──────────┐  ┌──────────┐
  │ web-ui   │  │  mail-rs │  │ chat-rs  │
  │(Port 3000│  │(Port 8080│  │(Port 8081│
  └────┬─────┘  └────┬─────┘  └────┬─────┘
       │             │              │
       │    ┌────────┴────────┐     │
       │    │                 │     │
       ▼    ▼                 ▼     ▼
  ┌────────────────┐    ┌──────────────┐
  │  ai-runtime    │    │   dav-rs     │
  │  (Port 8888)   │    │ (Port 8082)  │
  └────────┬───────┘    └──────────────┘
           │
    ┌──────┴──────┐
    │             │
    ▼             ▼
┌─────────┐  ┌─────────┐
│mcp-mail │  │mcp-chat │  MCP Servers
│ (8090)  │  │ (8091)  │
└─────────┘  └─────────┘
```

## Composants du Système

### 1. **proxy-rs** - Reverse Proxy
- Point d'entrée unique (ports 80/443)
- Gestion SSL/TLS automatique (Let's Encrypt)
- Rate limiting
- Routing vers services backend

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/proxy-rs_CLAUDE.md)

### 2. **mail-rs** - Serveur Mail
- SMTP (envoi/réception)
- IMAP (lecture)
- API REST pour MCP
- SPF, DKIM, DMARC

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/mail-rs_CLAUDE.md)

### 3. **chat-rs** - Messagerie Instantanée
- WebSocket temps réel
- Rooms/channels
- Bridges WhatsApp/Google Chat
- Persistence PostgreSQL

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/chat-rs_CLAUDE.md)

### 4. **dav-rs** - CalDAV/CardDAV
- Calendriers (CalDAV)
- Contacts (CardDAV)
- Sync multi-devices
- API REST pour MCP

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/dav-rs_CLAUDE.md)

### 5. **ai-runtime** ⭐ - LLM + MCP Orchestrator
**C'EST LE CŒUR DIFFÉRENCIANT DU SYSTÈME**

- Exécution LLM local (Mistral/Llama)
- Orchestration MCP servers
- Tool calling
- Conversation management

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/ai-runtime_CLAUDE.md)

### 6. **mcp-mail-server** - MCP Server Mail
- Expose mail-rs via MCP
- Tools : list_emails, read_email, send_email, search_emails
- Bridge LLM ↔ Mail

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/mcp-mail-server_CLAUDE.md)

### 7. **web-ui** - Interface Conversationnelle
- Chat interface
- WebSocket streaming
- Markdown rendering
- HTMX + Alpine.js

📄 [Voir specs détaillées](computer:///mnt/user-data/outputs/web-ui_CLAUDE.md)

## Ordre de Développement (Roadmap)

### Phase 1 : Mail (10-12 semaines)
**Priorité absolue** - Sans mail fonctionnel, le reste n'a pas de sens.

1. SMTP receiver (2 semaines)
2. SMTP sender + queue (2 semaines)
3. IMAP read-only (2 semaines)
4. IMAP complete + API (2 semaines)
5. Security hardening (1 semaine)
6. Docker + docs (1 semaine)

📊 **Milestone** : Pouvoir envoyer/recevoir des mails avec Gmail/Outlook

### Phase 2 : Proxy (2 semaines)
**En parallèle de Phase 1**

1. HTTP reverse proxy basique (1 semaine)
2. TLS + Let's Encrypt (1 semaine)

### Phase 3 : AI Runtime + MCP Mail (3 semaines)
**Le différenciateur**

1. LLM loading & inference (1 semaine)
2. MCP protocol + registry (1 semaine)
3. mcp-mail-server implementation (1 semaine)

📊 **Milestone** : Pouvoir demander "liste mes mails" et obtenir une réponse

### Phase 4 : Web UI (4 semaines)
**L'interface utilisateur**

1. Auth + layout (1 semaine)
2. Chat interface (1 semaine)
3. WebSocket streaming (1 semaine)
4. Polish + responsive (1 semaine)

📊 **Milestone** : MVP complet utilisable

### Phase 5 : Chat (4 semaines)
**Extension communication**

1. WebSocket messaging (2 semaines)
2. Rooms + persistence (1 semaine)
3. MCP server chat (1 semaine)

### Phase 6 : DAV (3 semaines)
**Calendriers + Contacts**

1. CalDAV core (1.5 semaines)
2. CardDAV core (1 semaine)
3. MCP server dav (0.5 semaine)

## Timeline Globale

**MVP Complet** : ~26-30 semaines (6-7 mois) en solo à temps partiel

**MVP Minimal** (mail + AI + UI) : ~20 semaines (5 mois)

## Stack Technique

- **Langage** : Rust 🦀 (100% propriétaire)
- **LLM** : Mistral 7B ou Llama 3.1 8B (quantifié)
- **Databases** : 
  - SQLite (mail, ai-runtime, dav)
  - PostgreSQL (chat)
- **Protocols** :
  - SMTP/IMAP (mail)
  - WebSocket (chat, web-ui)
  - CalDAV/CardDAV (dav)
  - MCP (AI orchestration)
- **Frontend** : HTMX + Alpine.js + Tailwind CSS
- **Deployment** : Docker + docker-compose

## Utilisation des Fichiers CLAUDE.md

Chaque fichier `*_CLAUDE.md` contient :

1. **Vue d'ensemble** du composant
2. **Architecture technique** détaillée
3. **Stack Rust** avec dépendances
4. **Structure de fichiers** complète
5. **Spécifications fonctionnelles** précises
6. **Exemples de code** Rust
7. **Configuration**
8. **Tests**
9. **Déploiement**
10. **Roadmap** du composant

### Comment utiliser ces fichiers

**Pour démarrer un nouveau composant** :

1. Lire le fichier CLAUDE.md correspondant
2. Créer le projet Cargo
3. Copier les dépendances du Cargo.toml
4. Créer la structure de fichiers
5. Implémenter en suivant les specs

**Pour déléguer/paralléliser** :

- Chaque fichier est autonome
- Peut être donné à un développeur différent
- Interfaces clairement définies entre composants

**Pour générer du code avec Claude** :

- Charger le fichier CLAUDE.md dans le contexte
- Demander l'implémentation d'un module spécifique
- Claude aura tout le contexte nécessaire

## Stratégie Open Source

### MVP Gratuit

- Tous les composants open source (AGPL v3 ou BSL)
- Repo GitHub public après MVP fonctionnel
- Documentation complète
- Quick start guide

### Validation

**Objectifs 3 mois post-lancement** :
- ⭐ 100+ stars GitHub
- 🔧 10+ installations production
- 👥 5+ contributeurs

**Si validation positive** → Continuer développement
**Si validation négative** → Pivot ou stop (2 mois investis, pas 12)

### Monétisation (si succès)

- Support commercial
- Hosted version managée
- Fonctionnalités entreprise (SSO, audit logs, etc.)
- Formation/consulting

## Différenciation vs Concurrents

| Proton | Nextcloud | **Ce Projet** |
|--------|-----------|---------------|
| Cloud centralisé | Self-hosted | Self-hosted ✅ |
| UI classique | UI classique | **AI-native UX** ⭐ |
| Multi-services | Monolithique | Microservices |
| Freemium SaaS | Open source | Open source ✅ |
| E2E chiffrement | Chiffrement serveur | E2E + **LLM local** ✅ |

**Proposition de valeur unique** :

> "La première suite de communication self-hosted pilotée par une interface conversationnelle AI, où tu dialogues avec ton assistant au lieu de cliquer dans des menus."

## Prochaines Étapes

### Immédiat (Cette semaine)

1. ✅ Créer les fichiers CLAUDE.md (FAIT)
2. ⬜ Setup repo Git
3. ⬜ Décider du nom du projet
4. ⬜ Créer projet `mail-rs`
5. ⬜ Commencer SMTP receiver

### Court terme (1 mois)

- SMTP receiver fonctionnel
- SMTP sender + queue
- Tests avec Gmail/Outlook

### Moyen terme (3 mois)

- Mail complet
- AI runtime + MCP
- Web UI basique
- **MVP testable**

## Notes Importantes

1. **Tout le code sera propriétaire** - Pas de fork de projets existants
2. **Focus sur le différenciateur** - L'UX AI-native est la clé
3. **Approche lean** - MVP pour valider avant d'investir 1 an
4. **Rust everywhere** - Performance, sécurité, contrôle total
5. **Start simple** - HTMX avant Leptos, SQLite avant PostgreSQL

## Contact / Support

Pour questions sur l'architecture ou les specs :
- Relire les fichiers CLAUDE.md correspondants
- Chaque composant a sa section "Notes d'implémentation"
- Les décisions architecturales sont documentées

---

**Version** : 1.0  
**Date** : 2024-11-18  
**Auteur** : Kevin (CTO Shiroo.ai)  
**Statut** : Spécifications complètes - Prêt pour développement
