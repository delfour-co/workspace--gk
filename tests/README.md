# E2E Tests - GK Mail Suite

Tests end-to-end complets pour valider l'ensemble du système GK Mail.

## 📋 Vue d'Ensemble

Les tests E2E couvrent 5 scénarios principaux :

1. **Test 1: Envoi Email Simple** - Envoi via AI assistant → MCP → SMTP
2. **Test 2: Réception Email Externe** - SMTP → Maildir → IMAP → AI
3. **Test 3: Conversation Email** - Envoi → Réponse → Thread complet
4. **Test 4: Assistant IA** - 6 types de requêtes différentes
5. **Test 5: Multi-utilisateurs** - Isolation et concurrence

## 🚀 Prérequis

### Services Docker Required

Les tests nécessitent que tous les services Docker soient en cours d'exécution :

```bash
# Démarrer tous les services
docker compose -f docker-compose.dev.yml up -d

# Vérifier que tous les services sont UP
docker compose -f docker-compose.dev.yml ps
```

Services requis :
- ✅ `gk-mail-rs-dev` (SMTP + IMAP sur ports 2525, 1993)
- ✅ `gk-mcp-mail-dev` (MCP Mail Server sur port 8090)
- ✅ `gk-ollama-dev` (Ollama avec llama3.1:8b sur port 11434)
- ✅ `gk-ai-runtime-dev` (AI Runtime sur port 8888)
- ✅ `gk-web-ui-dev` (Web UI sur port 5173)

### Dépendances Rust

Les dépendances nécessaires sont déjà dans le workspace `Cargo.toml` :
- `tokio` (async runtime)
- `reqwest` (HTTP client)
- `tokio-tungstenite` (WebSocket client)
- `futures-util` (async utilities)
- `serde` / `serde_json` (serialization)

## 🧪 Exécuter les Tests

### Tous les Tests

```bash
# Depuis la racine du projet
cargo test --test 'e2e_*' -- --nocapture

# Ou plus verbeux
RUST_LOG=debug cargo test --test 'e2e_*' -- --nocapture --test-threads=1
```

### Test Individuel

```bash
# Test 1: Envoi email simple
cargo test --test e2e_test_1_send_email -- --nocapture

# Test 2: Réception email
cargo test --test e2e_test_2_receive_email -- --nocapture

# Test 3: Conversation
cargo test --test e2e_test_3_conversation -- --nocapture

# Test 4: AI Assistant
cargo test --test e2e_test_4_ai_assistant -- --nocapture

# Test 5: Multi-utilisateurs
cargo test --test e2e_test_5_multi_users -- --nocapture
```

### Options Utiles

```bash
# Exécuter en séquentiel (recommandé pour E2E)
cargo test --test 'e2e_*' -- --test-threads=1 --nocapture

# Afficher la sortie même si test réussi
cargo test --test e2e_test_1_send_email -- --nocapture --show-output

# Avec logs de debug
RUST_LOG=debug cargo test --test e2e_test_1_send_email -- --nocapture
```

## 📊 Structure des Tests

```
tests/
├── README.md                          # Ce fichier
├── e2e/                               # Modules helper
│   ├── mod.rs                         # Module exports
│   ├── helpers.rs                     # TestEnv, TestResult, utilities
│   ├── websocket_client.rs            # WebSocket test client
│   ├── smtp_client.rs                 # SMTP test client
│   └── imap_client.rs                 # IMAP test client
├── e2e_test_1_send_email.rs           # Test 1: Envoi simple
├── e2e_test_2_receive_email.rs        # Test 2: Réception externe
├── e2e_test_3_conversation.rs         # Test 3: Conversation
├── e2e_test_4_ai_assistant.rs         # Test 4: AI queries
└── e2e_test_5_multi_users.rs          # Test 5: Multi-users
```

## 🔧 Configuration

### Variables d'Environnement

Les tests utilisent les URLs par défaut :
- AI Runtime: `http://localhost:8888`
- MCP Mail: `http://localhost:8090`
- SMTP: `localhost:2525`
- IMAP: `localhost:1993`

Pour modifier, éditer `tests/e2e/helpers.rs` :

```rust
impl Default for TestEnv {
    fn default() -> Self {
        Self {
            mail_rs_url: "localhost:2525".to_string(),
            ai_runtime_url: "http://localhost:8888".to_string(),
            // ...
        }
    }
}
```

### Utilisateur de Test

Par défaut :
- Email: `test@localhost`
- Password: `testpass123`

Les tests créent automatiquement cet utilisateur s'il n'existe pas.

## 📝 Détails des Tests

### Test 1: Envoi Email Simple

**Objectif**: Tester le flow complet d'envoi via AI assistant

**Étapes**:
1. ✅ Vérifier services running
2. ✅ Créer utilisateur test
3. ✅ Nettoyer mailbox
4. ✅ Connecter WebSocket
5. ✅ Authentifier
6. ✅ Envoyer email via prompt AI
7. ✅ Vérifier tool `send_email` appelé
8. ✅ Vérifier succès dans réponse

**Durée estimée**: ~5-10 secondes

### Test 2: Réception Email Externe

**Objectif**: Tester réception depuis client SMTP externe

**Étapes**:
1. ✅ Setup environnement
2. ✅ Compter emails initiaux
3. ✅ Envoyer email via SMTP direct
4. ✅ Vérifier augmentation count
5. ✅ Vérifier via IMAP SELECT
6. ✅ Vérifier via IMAP SEARCH
7. ✅ Vérifier via AI assistant

**Durée estimée**: ~8-12 secondes

### Test 3: Conversation Email

**Objectif**: Tester une conversation complète

**Étapes**:
1. ✅ Envoyer email initial (externe → user)
2. ✅ Lister emails via AI
3. ✅ Vérifier email présent
4. ✅ Répondre via AI
5. ✅ Rechercher emails de l'expéditeur

**Durée estimée**: ~10-15 secondes

### Test 4: AI Assistant Requests

**Objectif**: Tester 6 types de requêtes IA différentes

**Tests inclus**:
1. ✅ 4.1: Liste tous les emails
2. ✅ 4.2: Recherche par expéditeur
3. ✅ 4.3: Recherche par mot-clé
4. ✅ 4.4: Composer et envoyer
5. ✅ 4.5: Compter les emails (NL query)
6. ✅ 4.6: Résultats vides (graceful)

**Durée estimée**: ~20-30 secondes

### Test 5: Multi-Utilisateurs

**Objectif**: Tester isolation et concurrence

**Tests inclus**:
1. ✅ Créer 3 utilisateurs (alice, bob, charlie)
2. ✅ Envoyer emails croisés
3. ✅ Vérifier isolation mailbox
4. ✅ Tester accès concurrent

**Durée estimée**: ~15-20 secondes

## ✅ Interprétation des Résultats

### Test Réussi

```
================================================================================
📝 Test: E2E Test 1: Send Email Simple
⏱️  Duration: 8.234s
✅ Test passed
================================================================================

🎉 Test completed successfully!
```

### Test Échoué

```
================================================================================
📝 Test: E2E Test 1: Send Email Simple
⏱️  Duration: 3.456s
❌ Test failed: WebSocket connection failed: Connection refused
================================================================================

thread 'test_e2e_1_send_email_simple' panicked at 'WebSocket connection failed'
```

## 🐛 Troubleshooting

### Services pas démarrés

**Erreur**: `Services check failed: ai-runtime not ready`

**Solution**:
```bash
docker compose -f docker-compose.dev.yml up -d
docker compose -f docker-compose.dev.yml ps  # Vérifier status
```

### Timeout WebSocket

**Erreur**: `Timeout after 60 seconds`

**Causes possibles**:
- Ollama charge le modèle (première fois)
- Réseau lent
- Services surchargés

**Solution**:
```bash
# Vérifier logs ai-runtime
docker logs gk-ai-runtime-dev --tail 50

# Vérifier logs ollama
docker logs gk-ollama-dev --tail 50
```

### Email non trouvé

**Erreur**: `Email not found in mailbox`

**Solution**:
```bash
# Vérifier maildir
docker exec gk-mail-rs-dev ls -la /data/maildir/test_at_localhost/new

# Vérifier logs mail-rs
docker logs gk-mail-rs-dev --tail 50
```

### Utilisateur existe déjà

**Warning**: `Failed to create test user: User already exists`

Ce n'est pas une erreur - le test gère ce cas automatiquement.

## 📈 Métriques de Couverture

### Composants Testés

- ✅ **mail-rs** (SMTP receiver + sender + IMAP)
- ✅ **mcp-mail-server** (4 tools MCP)
- ✅ **ai-runtime** (LLM + WebSocket + tool calling)
- ✅ **Ollama** (function calling llama3.1:8b)
- ✅ **Integration** (end-to-end flows)

### Scénarios Couverts

- ✅ Envoi email via AI
- ✅ Réception email SMTP
- ✅ Lecture via IMAP
- ✅ Recherche emails
- ✅ Conversation thread
- ✅ Multi-utilisateurs
- ✅ Isolation données
- ✅ Accès concurrent

### Tools MCP Testés

- ✅ `send_email` - Envoi d'email
- ✅ `list_emails` - Liste des emails
- ✅ `search_emails` - Recherche
- ✅ `read_email` - Lecture (implicite)

## 🚦 CI/CD Integration

### GitHub Actions Example

```yaml
name: E2E Tests

on: [push, pull_request]

jobs:
  e2e-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Setup Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable

      - name: Start Docker services
        run: |
          docker compose -f docker-compose.dev.yml up -d
          sleep 30  # Wait for services to be ready

      - name: Run E2E tests
        run: |
          cargo test --test 'e2e_*' -- --test-threads=1 --nocapture

      - name: Stop Docker services
        if: always()
        run: docker compose -f docker-compose.dev.yml down
```

## 📚 Références

- [Test Module Helpers](./e2e/helpers.rs) - Utilities et TestEnv
- [WebSocket Client](./e2e/websocket_client.rs) - Client WebSocket pour tests
- [SMTP Client](./e2e/smtp_client.rs) - Client SMTP pour tests
- [IMAP Client](./e2e/imap_client.rs) - Client IMAP pour tests

## ✨ Améliorations Futures

- [ ] Tests de performance (load testing)
- [ ] Tests de sécurité (injection, auth bypass)
- [ ] Tests de résilience (network failures, timeouts)
- [ ] Tests de migration de données
- [ ] Tests d'upgrade (compatibility)
- [ ] Visual regression tests (web-ui)

---

**Questions?** Voir [docs/PRODUCTION_READY_PLAN.md](../docs/PRODUCTION_READY_PLAN.md)
