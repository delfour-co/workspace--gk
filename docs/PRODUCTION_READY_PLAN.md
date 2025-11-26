# 🚀 Plan Production-Ready - GK Mail

**Date de début**: 2025-11-25
**Durée estimée**: 2-3 semaines
**Objectif**: Rendre le système stable, sécurisé et prêt pour un usage quotidien

---

## 📋 Vue d'ensemble

### Statut Actuel
- ✅ MVP fonctionnel (mail-rs + ai-runtime + mcp-mail-server + web-ui)
- ✅ Stack Docker complète avec Ollama
- ✅ WebSocket et communication temps réel
- ✅ Authentification basique

### Objectifs
- 🎯 Système stable et fiable
- 🎯 Sécurité renforcée
- 🎯 Monitoring et observabilité
- 🎯 Documentation complète
- 🎯 Prêt pour usage quotidien

---

## 🧪 1. Tests End-to-End (Priorité: P0)

### 1.1 Scénarios Utilisateur Réels
**Durée**: 3-4 jours

**Tâches**:
- [ ] **Test E2E 1**: Envoi d'email simple
  - Envoyer email via web-ui
  - Vérifier réception dans Maildir
  - Vérifier visibilité via IMAP

- [ ] **Test E2E 2**: Réception d'email externe
  - Configurer relay SMTP test (MailHog ou similaire)
  - Envoyer email depuis client externe
  - Vérifier notification temps réel

- [ ] **Test E2E 3**: Conversation email complète
  - Envoi → Réception → Réponse → Thread
  - Vérifier flags (\Seen, \Answered)

- [ ] **Test E2E 4**: Assistant IA
  - "Liste mes emails" avec emails présents
  - "Recherche emails de bob"
  - "Envoie un email à alice"
  - Vérifier parsing et exécution correcte

- [ ] **Test E2E 5**: Gestion multi-utilisateurs
  - Créer 3 utilisateurs
  - Envoyer emails croisés
  - Vérifier isolation des boîtes mail

### 1.2 Tests de Charge
**Durée**: 2 jours

- [ ] Test: 100 emails/minute
- [ ] Test: 10 connexions IMAP simultanées
- [ ] Test: 50 requêtes WebSocket/seconde
- [ ] Identifier les goulots d'étranglement
- [ ] Documenter les limites du système

### 1.3 Tests d'Intégration
**Durée**: 2 jours

- [ ] Test avec Thunderbird
- [ ] Test avec Apple Mail
- [ ] Test avec Gmail (envoi/réception)
- [ ] Test avec Outlook.com
- [ ] Documenter la compatibilité

---

## 🔒 2. Sécurité Renforcée (Priorité: P0)

### 2.1 SPF/DKIM/DMARC Complets
**Durée**: 3 jours

**SPF (mail-rs/src/utils/spf.rs)**:
- [ ] Implémenter mécanisme `a` complet
- [ ] Implémenter mécanisme `mx` complet
- [ ] Implémenter mécanisme `include` avec récursion
- [ ] Support CIDR notation (ip4:192.168.1.0/24)
- [ ] Tests avec vrais records SPF
- [ ] Documentation configuration DNS

**DKIM (mail-rs/src/utils/dkim.rs)**:
- [ ] Vérification signature complète (RSA-SHA256)
- [ ] Support ed25519-sha256
- [ ] Génération clés DKIM (CLI tool)
- [ ] Signature emails sortants
- [ ] Tests avec vrais headers DKIM
- [ ] Documentation génération/rotation clés

**DMARC (nouveau)**:
- [ ] Parser records DMARC
- [ ] Implémenter politique (none/quarantine/reject)
- [ ] Reporting (rua/ruf)
- [ ] Tests alignement SPF/DKIM
- [ ] Documentation configuration

### 2.2 TLS/STARTTLS Robuste
**Durée**: 2 jours

- [ ] Compléter upgrade TLS stream (mail-rs/src/smtp/session.rs)
- [ ] Forcer TLS pour AUTH (option configurable)
- [ ] Support TLS 1.3 uniquement (option)
- [ ] Vérification certificats clients (optionnel)
- [ ] Tests avec différents clients TLS
- [ ] Documentation certificats

### 2.3 Rate Limiting & Anti-Abuse
**Durée**: 2 jours

- [ ] Rate limiting par IP (SMTP)
- [ ] Rate limiting par utilisateur (API)
- [ ] Détection bruteforce AUTH
- [ ] Blacklist IPs temporaire
- [ ] Protection contre spam:
  - Limite taille message
  - Limite recipients par message
  - Greylist optionnel
- [ ] Logs événements sécurité

### 2.4 Audit Sécurité
**Durée**: 1 jour

- [ ] Revue code sécurité (injection, XSS, etc.)
- [ ] Scan dépendances (cargo audit)
- [ ] Vérifier gestion secrets (pas de hardcode)
- [ ] Vérifier permissions fichiers Maildir
- [ ] Documentation best practices

---

## 📊 3. Observabilité & Monitoring (Priorité: P1)

### 3.1 Logs Structurés
**Durée**: 2 jours

**mail-rs**:
- [ ] Remplacer `tracing::info!` par logs structurés (JSON)
- [ ] Contexte enrichi (user, session_id, request_id)
- [ ] Niveaux appropriés (ERROR, WARN, INFO, DEBUG)
- [ ] Rotation logs (logrotate ou similar)

**ai-runtime**:
- [ ] Logs JSON pour production
- [ ] Tracer latences LLM
- [ ] Logger tool calls et résultats

**Configuration**:
- [ ] Support RUST_LOG avancé
- [ ] Format logs configurable (text/json)
- [ ] Output configurable (stdout/file)

### 3.2 Métriques Prometheus
**Durée**: 2 jours

**Métriques mail-rs** (enrichir l'existant):
- [ ] `smtp_sessions_total` (success/failed)
- [ ] `smtp_commands_duration_seconds` (histogram)
- [ ] `imap_connections_active` (gauge)
- [ ] `maildir_size_bytes` par user
- [ ] `queue_size` (pending emails)
- [ ] `auth_attempts_total` par méthode

**Métriques ai-runtime**:
- [ ] `llm_requests_total`
- [ ] `llm_latency_seconds` (histogram)
- [ ] `mcp_tool_calls_total` par tool
- [ ] `websocket_connections_active`
- [ ] `websocket_messages_total`

**Dashboard Grafana**:
- [ ] Créer dashboard Grafana template
- [ ] Panel santé système
- [ ] Panel performance LLM
- [ ] Panel activité email

### 3.3 Health Checks Robustes
**Durée**: 1 jour

- [ ] Endpoint `/health` détaillé:
  - SMTP listener ok
  - IMAP listener ok
  - Ollama accessible
  - Disk space suffisant
  - Database accessible
- [ ] Endpoint `/ready` (readiness probe)
- [ ] Endpoint `/metrics` (prometheus)
- [ ] Tests health checks

### 3.4 Alerting
**Durée**: 1 jour

- [ ] Configuration Alertmanager
- [ ] Alertes critiques:
  - Service down > 1min
  - Erreur rate > 10%
  - Disk usage > 80%
  - Auth failures spike
- [ ] Documentation alerting

---

## 🐳 4. Docker Production (Priorité: P1)

### 4.1 Multi-stage Builds Optimisés
**Durée**: 2 jours

- [ ] Optimiser Dockerfile:
  - Cache dependencies séparément
  - Utiliser cargo-chef pour deps
  - Images slim (alpine/distroless)
  - Taille finale < 50MB par service
- [ ] Dockerfile.prod distinct
- [ ] Build avec BuildKit
- [ ] Tests images optimisées

### 4.2 Docker Compose Production
**Durée**: 2 jours

**Créer docker-compose.prod.yml**:
- [ ] Volumes persistants nommés
- [ ] Secrets via Docker secrets
- [ ] Resource limits (CPU/Memory)
- [ ] Restart policies robustes
- [ ] Logging drivers configurés
- [ ] Network isolation

**Services additionnels**:
- [ ] Prometheus
- [ ] Grafana
- [ ] Alertmanager (optionnel)
- [ ] Nginx reverse proxy (optionnel)

### 4.3 Healthchecks & Orchestration
**Durée**: 1 jour

- [ ] Healthchecks pour tous les services
- [ ] depends_on avec conditions
- [ ] Startup probes appropriés
- [ ] Grace periods configurés
- [ ] Tests failover

### 4.4 Images Registry
**Durée**: 1 jour

- [ ] Pipeline CI/CD (GitHub Actions ou GitLab CI)
- [ ] Build automatique images
- [ ] Push vers registry (Docker Hub / GHCR)
- [ ] Tags versionnés (semver)
- [ ] Scan sécurité images

---

## 📖 5. Documentation (Priorité: P1)

### 5.1 Guide Installation
**Durée**: 2 jours

**Créer docs/INSTALLATION.md**:
- [ ] Prérequis système
- [ ] Installation Docker
- [ ] Clonage et configuration
- [ ] Variables d'environnement
- [ ] Premier démarrage
- [ ] Vérification santé
- [ ] Troubleshooting courant

### 5.2 Guide Configuration
**Durée**: 2 jours

**Créer docs/CONFIGURATION.md**:
- [ ] Configuration mail-rs (TOML)
- [ ] Configuration DNS (MX, SPF, DKIM, DMARC)
- [ ] Configuration TLS/certificats
- [ ] Configuration Ollama (modèles)
- [ ] Configuration reverse proxy
- [ ] Exemples complets

### 5.3 Guide Utilisation
**Durée**: 2 jours

**Créer docs/USER_GUIDE.md**:
- [ ] Créer un utilisateur
- [ ] Se connecter via web-ui
- [ ] Utiliser l'assistant IA
- [ ] Configurer client mail (Thunderbird, etc.)
- [ ] Exemples requêtes IA
- [ ] FAQ

### 5.4 Guide Administration
**Durée**: 2 jours

**Créer docs/ADMIN_GUIDE.md**:
- [ ] Gestion utilisateurs (CLI)
- [ ] Backup/restore
- [ ] Monitoring (Grafana)
- [ ] Rotation logs
- [ ] Mise à jour système
- [ ] Procédures incident

### 5.5 Guide Développement
**Durée**: 1 jour

**Mettre à jour docs/DEVELOPMENT.md**:
- [ ] Architecture détaillée
- [ ] Setup environnement dev
- [ ] Tests (unit, integ, e2e)
- [ ] Conventions code
- [ ] Contribution guidelines

---

## 🔧 6. Améliorations Système (Priorité: P2)

### 6.1 Gestion Erreurs
**Durée**: 2 jours

- [ ] Error types unifiés (thiserror)
- [ ] Error context enrichi (anyhow)
- [ ] Messages d'erreur clairs utilisateur
- [ ] Récupération erreurs gracieuse
- [ ] Logging erreurs structuré

### 6.2 Configuration Centralisée
**Durée**: 1 jour

- [ ] Fichier config.toml global
- [ ] Variables environnement override
- [ ] Validation config au démarrage
- [ ] Reload config sans redémarrage (SIGHUP)
- [ ] Config par défaut sensible

### 6.3 Performance
**Durée**: 2 jours

- [ ] Profile CPU (perf, flamegraph)
- [ ] Profile mémoire (valgrind, heaptrack)
- [ ] Optimiser hot paths
- [ ] Connection pooling
- [ ] Cache DNS
- [ ] Benchmarks avant/après

### 6.4 Graceful Shutdown
**Durée**: 1 jour

- [ ] Signal handlers (SIGTERM, SIGINT)
- [ ] Drain connections en cours
- [ ] Flush queue emails
- [ ] Sauvegarder état
- [ ] Tests shutdown propre

---

## 📅 Planning Détaillé

### Semaine 1: Tests & Sécurité
- **Jours 1-2**: Tests E2E scénarios réels
- **Jours 3-4**: Tests charge et intégration
- **Jour 5**: SPF/DKIM complet

### Semaine 2: Observabilité & Docker
- **Jours 1-2**: Logs structurés & métriques
- **Jours 3-4**: Docker production optimisé
- **Jour 5**: Health checks & alerting

### Semaine 3: Documentation & Polish
- **Jours 1-2**: Documentation complète
- **Jours 3-4**: Améliorations système
- **Jour 5**: Tests finaux & release

---

## ✅ Critères de Succès

### Must-Have (P0)
- [ ] ✅ 10+ scénarios E2E passent
- [ ] ✅ SPF/DKIM/DMARC fonctionnels
- [ ] ✅ TLS/STARTTLS robustes
- [ ] ✅ Logs structurés JSON
- [ ] ✅ Dashboard Grafana opérationnel
- [ ] ✅ Docker compose production prêt
- [ ] ✅ Documentation installation complète

### Should-Have (P1)
- [ ] ✅ Tests charge validés (100 emails/min)
- [ ] ✅ Rate limiting actif
- [ ] ✅ CI/CD pipeline fonctionnel
- [ ] ✅ Images < 50MB
- [ ] ✅ Guide admin complet

### Nice-to-Have (P2)
- [ ] ✅ Alerting configuré
- [ ] ✅ Profiling performance fait
- [ ] ✅ Reload config sans downtime
- [ ] ✅ Guide contribution

---

## 🎯 Prochaine Action Immédiate

**Commencer par**: Tests E2E - Scénario 1 (Envoi d'email simple)

```bash
# 1. Créer structure tests E2E
mkdir -p tests/e2e

# 2. Créer premier test
touch tests/e2e/test_send_email.rs

# 3. Setup environnement test
docker compose -f docker-compose.test.yml up -d
```

**Prêt à démarrer ?**
