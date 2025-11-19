# ai-runtime - AI Orchestration Engine

**Le cœur du système GK** - Orchestre un LLM local avec des MCP servers pour créer une interface conversationnelle.

## 🎯 Vision

Permettre à l'utilisateur d'interagir avec tous les services (mail, chat, calendrier) via une interface conversationnelle naturelle:

```
User: "Envoie un email à john@example.com pour lui dire bonjour"
AI: "✅ Email envoyé à john@example.com avec le sujet 'Bonjour'"

User: "Quels sont mes mails importants aujourd'hui?"
AI: "Tu as 3 emails importants:
     1. De alice@example.com: Réunion demain
     2. De bob@company.com: Contrat à signer
     3. De team@startup.io: Release notes v2.0"
```

## 🏗️ Architecture

```
┌─────────────────────────────────────────┐
│           web-ui / CLI                   │
│        (WebSocket / HTTP)                │
└──────────────────┬──────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────┐
│           ai-runtime                     │
│  ┌─────────────────────────────────┐   │
│  │   Conversation Manager          │   │
│  │   - Context history             │   │
│  │   - Message queue               │   │
│  └──────────────┬──────────────────┘   │
│                 │                        │
│  ┌──────────────▼──────────────────┐   │
│  │      LLM Engine                 │   │
│  │   - Model loading               │   │
│  │   - Inference                   │   │
│  │   - Tool calling                │   │
│  └──────────────┬──────────────────┘   │
│                 │                        │
│  ┌──────────────▼──────────────────┐   │
│  │    MCP Orchestrator             │   │
│  │   - Server registry             │   │
│  │   - Tool discovery              │   │
│  │   - Request routing             │   │
│  └──────────────┬──────────────────┘   │
└─────────────────┼──────────────────────┘
                  │
        ┌─────────┼─────────┐
        │         │         │
        ▼         ▼         ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │ mcp-   │ │ mcp-   │ │ mcp-   │
   │ mail   │ │ chat   │ │ cal    │
   └────────┘ └────────┘ └────────┘
        │         │         │
        ▼         ▼         ▼
   ┌────────┐ ┌────────┐ ┌────────┐
   │mail-rs │ │chat-rs │ │dav-rs  │
   └────────┘ └────────┘ └────────┘
```

## 🚀 Features

### Phase 1: MVP (Cette semaine)

- ✅ LLM local loading (mock pour tests)
- ✅ MCP protocol basique (JSON-RPC)
- ✅ MCP server registry
- ✅ Tool calling vers mcp-mail-server
- ✅ API HTTP simple (/chat endpoint)
- ✅ Test E2E: Envoyer email via conversation

### Phase 2: Production

- ⚪ LLM réel (Mistral 7B / Llama 3.1 8B)
- ⚪ Streaming responses (WebSocket)
- ⚪ Context window management
- ⚪ Multiple MCP servers
- ⚪ Tool chaining (multi-step tasks)
- ⚪ Error recovery
- ⚪ Rate limiting

## 📋 MCP Protocol

Le **Model Context Protocol** (MCP) est un protocole standard pour connecter des LLMs à des outils externes.

### Format Message

```json
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "send_email",
    "arguments": {
      "to": "john@example.com",
      "subject": "Hello",
      "body": "Hi John!"
    }
  },
  "id": 1
}
```

### Tools Disponibles

#### mcp-mail-server

- `send_email(to, subject, body)` - Envoyer un email
- `list_emails(folder, limit)` - Lister les emails
- `read_email(id)` - Lire un email
- `search_emails(query)` - Rechercher emails

## 🔧 Stack Technique

- **Rust** (async/await avec Tokio)
- **LLM Engine**: llama.cpp bindings ou candle
- **MCP**: JSON-RPC over HTTP/WebSocket
- **API**: Axum framework
- **Config**: TOML

## 📦 Structure

```
ai-runtime/
├── src/
│   ├── main.rs              # Entry point + HTTP server
│   ├── llm/
│   │   ├── mod.rs           # LLM engine trait
│   │   ├── mock.rs          # Mock LLM for testing
│   │   └── mistral.rs       # Real LLM (future)
│   ├── mcp/
│   │   ├── mod.rs           # MCP protocol types
│   │   ├── server.rs        # MCP server client
│   │   ├── registry.rs      # Server registry
│   │   └── tool.rs          # Tool definition
│   ├── conversation/
│   │   ├── mod.rs           # Conversation manager
│   │   ├── context.rs       # Context window
│   │   └── message.rs       # Message types
│   └── config.rs            # Configuration
├── Cargo.toml
└── config.toml              # Runtime config
```

## 🎯 Quick Start

### 1. Démarrer mail-rs

```bash
cd mail-rs
cargo run --release
```

### 2. Démarrer mcp-mail-server

```bash
cd mcp-mail-server
cargo run
```

### 3. Démarrer ai-runtime

```bash
cd ai-runtime
cargo run
```

### 4. Tester

```bash
curl -X POST http://localhost:8888/chat \
  -H "Content-Type: application/json" \
  -d '{
    "message": "Envoie un email à john@example.com pour dire bonjour"
  }'
```

## 📝 Configuration

```toml
[server]
listen_addr = "0.0.0.0:8888"

[llm]
model = "mock"  # or "mistral-7b", "llama-3.1-8b"
model_path = "./models/mistral-7b-instruct.gguf"
context_size = 4096

[mcp]
servers = [
  { name = "mail", url = "http://localhost:8090" }
]
```

## 🔐 Sécurité

- ⚠️ **Pas d'authentification** pour MVP
- Production: JWT tokens
- Rate limiting sur API
- Validation inputs LLM

## 🧪 Tests

```bash
# Tests unitaires
cargo test

# Test avec mock LLM
cargo run --features mock

# Test E2E
./scripts/test-e2e.sh
```

## 📚 Ressources

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [llama.cpp](https://github.com/ggerganov/llama.cpp)
- [Candle](https://github.com/huggingface/candle)
- [Mistral Models](https://mistral.ai/technology/#models)

## 🎯 Milestone

**MVP validé quand**: On peut envoyer un email via une commande en langage naturel.

**Success criteria**:
- ✅ API `/chat` répond
- ✅ LLM parse l'intent "envoyer email"
- ✅ Tool calling vers mcp-mail-server
- ✅ Email effectivement envoyé via mail-rs
- ✅ Réponse confirmative à l'utilisateur
