# ai-runtime - LLM Runtime + MCP Orchestrator

## Vue d'ensemble

`ai-runtime` est le cœur du système AI-native. Il gère l'exécution de modèles de langage locaux (LLM) et orchestre les appels aux différents MCP servers pour permettre à l'assistant conversationnel d'interagir avec tous les services (mail, chat, contacts, etc.).

**C'EST LE COMPOSANT DIFFÉRENCIANT DU PROJET.**

## Contexte du projet global

Le `ai-runtime` transforme une suite d'outils classiques en une expérience conversationnelle unifiée. Au lieu de cliquer dans des interfaces séparées, l'utilisateur dialogue avec un assistant qui pilote tous les services.

### Interfaces avec les autres composants

- **Consomme** : MCP servers (mcp-mail-server, mcp-chat-server, mcp-dav-server)
- **Expose** : WebSocket API pour `web-ui`
- **Utilise** : Modèles LLM locaux (Mistral, Llama)
- **Derrière** : `proxy-rs` pour exposition HTTPS

## Responsabilités

### Primaires
1. **Exécution LLM locale**
   - Chargement modèles quantifiés (GGUF)
   - Inférence avec llama.cpp
   - Context management
   - Streaming responses

2. **MCP Orchestration**
   - Découverte MCP servers
   - Tool calling / function calling
   - Routing vers les bons servers
   - Gestion erreurs/retry

3. **Conversation Management**
   - Historique conversations
   - Context window optimization
   - Multi-turn dialogue
   - Persona/system prompts

4. **WebSocket API**
   - Communication temps réel avec UI
   - Streaming LLM responses
   - Events (typing, thinking, etc.)

### Secondaires
- Caching réponses fréquentes
- Métriques usage (tokens, latency)
- Rate limiting par user
- A/B testing prompts

## Architecture technique

### Stack Rust

```toml
[dependencies]
# LLM inference
llama-cpp-rs = "0.3"     # Bindings llama.cpp
# Ou alternative:
# candle-core = "0.3"    # Hugging Face Candle

# MCP protocol
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# HTTP client (pour MCP servers)
reqwest = { version = "0.11", features = ["json"] }

# WebSocket server
axum = { version = "0.7", features = ["ws"] }
tokio-tungstenite = "0.21"
futures = "0.3"

# Async
tokio = { version = "1", features = ["full"] }

# Database (conversation history)
sqlx = { version = "0.7", features = ["sqlite", "runtime-tokio-rustls"] }

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Config
toml = "0.8"
```

### Structure du projet

```
ai-runtime/
├── Cargo.toml
├── config.example.toml
├── README.md
├── Dockerfile
│
├── models/              # LLM models (GGUF files)
│   └── .gitkeep
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── llm/
│   │   ├── mod.rs
│   │   ├── engine.rs        # LLM inference engine
│   │   ├── loader.rs        # Model loading
│   │   ├── context.rs       # Context management
│   │   ├── prompts.rs       # System prompts
│   │   └── streaming.rs     # Token streaming
│   │
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── protocol.rs      # MCP protocol types
│   │   ├── client.rs        # MCP client (calls servers)
│   │   ├── registry.rs      # MCP server registry
│   │   ├── router.rs        # Tool call routing
│   │   └── executor.rs      # Tool execution
│   │
│   ├── conversation/
│   │   ├── mod.rs
│   │   ├── manager.rs       # Conversation lifecycle
│   │   ├── history.rs       # Message history
│   │   ├── context.rs       # Context window optimization
│   │   └── storage.rs       # SQLite persistence
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── websocket.rs     # WebSocket handler
│   │   ├── handlers.rs      # Request handlers
│   │   └── events.rs        # Event types
│   │
│   └── utils/
│       ├── mod.rs
│       ├── metrics.rs       # Token usage, latency
│       └── cache.rs         # Response caching
│
├── tests/
│   ├── integration/
│   │   ├── llm_test.rs
│   │   ├── mcp_test.rs
│   │   └── conversation_test.rs
│   └── fixtures/
│
├── prompts/             # System prompts templates
│   ├── system.txt
│   ├── mail_assistant.txt
│   └── chat_assistant.txt
│
└── docs/
    ├── ARCHITECTURE.md
    ├── MCP_PROTOCOL.md
    └── PROMPTS.md
```

## Spécifications fonctionnelles

### 1. LLM Engine

**Modèles supportés**

```toml
# config.toml
[llm]
model_path = "/app/models/mistral-7b-instruct-v0.2.Q4_K_M.gguf"
# Alternatives:
# - llama-3.1-8b-instruct.Q4_K_M.gguf
# - phi-3-mini-4k-instruct.Q4_K_M.gguf

context_size = 4096      # Token context window
n_gpu_layers = 0         # 0 = CPU only, -1 = all GPU
threads = 4              # CPU threads
temperature = 0.7
top_p = 0.9
top_k = 40
max_tokens = 2048        # Max response length
```

**Loading & Inference**

```rust
use llama_cpp_rs::{LlamaModel, LlamaContext, LlamaSession};

pub struct LLMEngine {
    model: LlamaModel,
    context: LlamaContext,
}

impl LLMEngine {
    pub async fn load(config: &LLMConfig) -> Result<Self> {
        info!("Loading model: {}", config.model_path);
        
        let model = LlamaModel::load_from_file(
            &config.model_path,
            LlamaModelParams {
                n_gpu_layers: config.n_gpu_layers,
                ..Default::default()
            }
        )?;
        
        let context = model.create_context(LlamaContextParams {
            n_ctx: config.context_size,
            n_threads: config.threads,
            ..Default::default()
        })?;
        
        Ok(Self { model, context })
    }
    
    pub async fn generate(
        &mut self,
        prompt: &str,
        params: GenerationParams,
    ) -> Result<String> {
        let tokens = self.context.tokenize(prompt, true)?;
        
        let mut output = String::new();
        
        self.context.generate(
            &tokens,
            |token| {
                let text = self.model.token_to_str(token)?;
                output.push_str(&text);
                
                // Stop conditions
                if output.contains("</response>") 
                    || output.len() > params.max_tokens * 4 {
                    return Ok(false); // Stop
                }
                
                Ok(true) // Continue
            },
            params.temperature,
            params.top_p,
            params.top_k,
        )?;
        
        Ok(output)
    }
    
    pub async fn generate_stream(
        &mut self,
        prompt: &str,
        params: GenerationParams,
        mut callback: impl FnMut(String) -> Result<()>,
    ) -> Result<String> {
        let tokens = self.context.tokenize(prompt, true)?;
        let mut output = String::new();
        
        self.context.generate(
            &tokens,
            |token| {
                let text = self.model.token_to_str(token)?;
                output.push_str(&text);
                callback(text)?;
                
                Ok(output.len() < params.max_tokens * 4)
            },
            params.temperature,
            params.top_p,
            params.top_k,
        )?;
        
        Ok(output)
    }
}
```

**System Prompt Engineering**

```rust
// prompts/system.txt
pub const SYSTEM_PROMPT: &str = r#"
You are an AI assistant with access to a personal communication system.
You can help users manage their emails, chat messages, and contacts.

You have access to the following tools:
- list_emails: List emails from a mailbox
- read_email: Read a specific email
- send_email: Send a new email
- search_emails: Search emails by query
- list_chats: List recent chat conversations
- send_message: Send a chat message
- list_contacts: List contacts

When the user asks you to do something, use the appropriate tool.
Always confirm actions before executing them (especially sends).

Be concise and helpful. Use natural language.
"#;

fn build_prompt(
    system: &str,
    history: &[Message],
    user_input: &str,
    available_tools: &[Tool],
) -> String {
    format!(
        r#"<|system|>
{}

Available tools:
{}
<|end|>

{}

<|user|>
{}
<|end|>

<|assistant|>
"#,
        system,
        serde_json::to_string_pretty(available_tools).unwrap(),
        format_history(history),
        user_input,
    )
}
```

### 2. MCP Protocol

**MCP Types**

```rust
// src/mcp/protocol.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value, // JSON Schema
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub tool: String,
    pub arguments: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub call_id: String,
    pub result: serde_json::Value,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String, // "2.0"
    pub method: String,  // "tools/list" or "tools/call"
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<MCPError>,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
```

**MCP Client**

```rust
// src/mcp/client.rs

pub struct MCPClient {
    http_client: reqwest::Client,
    base_url: String,
}

impl MCPClient {
    pub fn new(base_url: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
        }
    }
    
    pub async fn list_tools(&self) -> Result<Vec<Tool>> {
        let req = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/list".to_string(),
            params: serde_json::json!({}),
            id: 1,
        };
        
        let resp: MCPResponse = self.http_client
            .post(&format!("{}/mcp", self.base_url))
            .json(&req)
            .send()
            .await?
            .json()
            .await?;
        
        if let Some(error) = resp.error {
            return Err(Error::MCPError(error.message));
        }
        
        let tools: Vec<Tool> = serde_json::from_value(
            resp.result.ok_or(Error::NoResult)?
        )?;
        
        Ok(tools)
    }
    
    pub async fn call_tool(
        &self,
        tool_call: &ToolCall,
    ) -> Result<ToolResult> {
        let req = MCPRequest {
            jsonrpc: "2.0".to_string(),
            method: "tools/call".to_string(),
            params: serde_json::json!({
                "name": tool_call.tool,
                "arguments": tool_call.arguments,
            }),
            id: 2,
        };
        
        let resp: MCPResponse = self.http_client
            .post(&format!("{}/mcp", self.base_url))
            .json(&req)
            .send()
            .await?
            .json()
            .await?;
        
        if let Some(error) = resp.error {
            return Ok(ToolResult {
                call_id: tool_call.id.clone(),
                result: serde_json::json!(null),
                error: Some(error.message),
            });
        }
        
        Ok(ToolResult {
            call_id: tool_call.id.clone(),
            result: resp.result.ok_or(Error::NoResult)?,
            error: None,
        })
    }
}
```

**MCP Registry**

```rust
// src/mcp/registry.rs

pub struct MCPRegistry {
    servers: HashMap<String, MCPClient>,
    tools: HashMap<String, (String, Tool)>, // tool_name -> (server_name, tool)
}

impl MCPRegistry {
    pub async fn discover(&mut self, config: &Config) -> Result<()> {
        for server_config in &config.mcp_servers {
            let client = MCPClient::new(server_config.url.clone());
            
            info!("Discovering tools from {}", server_config.name);
            let tools = client.list_tools().await?;
            
            for tool in tools {
                info!("  - {}: {}", tool.name, tool.description);
                self.tools.insert(
                    tool.name.clone(),
                    (server_config.name.clone(), tool),
                );
            }
            
            self.servers.insert(server_config.name.clone(), client);
        }
        
        Ok(())
    }
    
    pub fn get_all_tools(&self) -> Vec<Tool> {
        self.tools.values()
            .map(|(_, tool)| tool.clone())
            .collect()
    }
    
    pub async fn execute_tool(
        &self,
        tool_call: &ToolCall,
    ) -> Result<ToolResult> {
        let (server_name, _) = self.tools
            .get(&tool_call.tool)
            .ok_or(Error::ToolNotFound(tool_call.tool.clone()))?;
        
        let client = self.servers
            .get(server_name)
            .ok_or(Error::ServerNotFound(server_name.clone()))?;
        
        client.call_tool(tool_call).await
    }
}
```

**Tool Calling Flow**

```rust
pub async fn handle_user_message(
    engine: &mut LLMEngine,
    registry: &MCPRegistry,
    history: &mut ConversationHistory,
    user_input: String,
) -> Result<String> {
    history.add_message(Message::User(user_input.clone()));
    
    let tools = registry.get_all_tools();
    let prompt = build_prompt(
        SYSTEM_PROMPT,
        &history.messages,
        &user_input,
        &tools,
    );
    
    // First LLM call - decide what to do
    let response = engine.generate(&prompt, GenerationParams::default()).await?;
    
    // Parse tool calls from response
    let tool_calls = parse_tool_calls(&response)?;
    
    if tool_calls.is_empty() {
        // No tool calls, direct response
        history.add_message(Message::Assistant(response.clone()));
        return Ok(response);
    }
    
    // Execute tool calls
    let mut tool_results = Vec::new();
    for call in &tool_calls {
        info!("Executing tool: {} with {:?}", call.tool, call.arguments);
        let result = registry.execute_tool(call).await?;
        tool_results.push(result);
    }
    
    // Second LLM call - synthesize results
    let final_prompt = build_synthesis_prompt(
        SYSTEM_PROMPT,
        &history.messages,
        &user_input,
        &tool_results,
    );
    
    let final_response = engine.generate(
        &final_prompt,
        GenerationParams::default()
    ).await?;
    
    history.add_message(Message::Assistant(final_response.clone()));
    
    Ok(final_response)
}
```

### 3. Conversation Management

**Message History**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    System(String),
    User(String),
    Assistant(String),
    ToolCall(ToolCall),
    ToolResult(ToolResult),
}

pub struct ConversationHistory {
    conversation_id: Uuid,
    messages: Vec<Message>,
    max_tokens: usize,
}

impl ConversationHistory {
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.trim_to_context();
    }
    
    fn trim_to_context(&mut self) {
        // Simple trimming: keep last N messages
        // TODO: Smarter context window management
        let max_messages = 20;
        if self.messages.len() > max_messages {
            self.messages = self.messages
                .iter()
                .skip(self.messages.len() - max_messages)
                .cloned()
                .collect();
        }
    }
    
    pub async fn save(&self, db: &SqlitePool) -> Result<()> {
        sqlx::query(
            "INSERT INTO conversations (id, messages, updated_at)
             VALUES (?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
             messages = excluded.messages,
             updated_at = excluded.updated_at"
        )
        .bind(self.conversation_id.to_string())
        .bind(serde_json::to_string(&self.messages)?)
        .bind(Utc::now())
        .execute(db)
        .await?;
        
        Ok(())
    }
    
    pub async fn load(
        db: &SqlitePool,
        conversation_id: Uuid,
    ) -> Result<Self> {
        let row: (String,) = sqlx::query_as(
            "SELECT messages FROM conversations WHERE id = ?"
        )
        .bind(conversation_id.to_string())
        .fetch_one(db)
        .await?;
        
        let messages: Vec<Message> = serde_json::from_str(&row.0)?;
        
        Ok(Self {
            conversation_id,
            messages,
            max_tokens: 4096,
        })
    }
}
```

### 4. WebSocket API

**Events**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientEvent {
    #[serde(rename = "message")]
    Message {
        conversation_id: Option<Uuid>,
        content: String,
    },
    
    #[serde(rename = "ping")]
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerEvent {
    #[serde(rename = "thinking")]
    Thinking,
    
    #[serde(rename = "tool_call")]
    ToolCall {
        tool: String,
        description: String,
    },
    
    #[serde(rename = "token")]
    Token {
        content: String,
    },
    
    #[serde(rename = "done")]
    Done {
        conversation_id: Uuid,
        message: String,
    },
    
    #[serde(rename = "error")]
    Error {
        message: String,
    },
    
    #[serde(rename = "pong")]
    Pong,
}
```

**WebSocket Handler**

```rust
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
) {
    let mut engine = state.llm_engine.lock().await;
    let registry = &state.mcp_registry;
    
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let event: ClientEvent = match serde_json::from_str(&text) {
                Ok(e) => e,
                Err(_) => continue,
            };
            
            match event {
                ClientEvent::Message { conversation_id, content } => {
                    // Send thinking event
                    send_event(&mut socket, ServerEvent::Thinking).await;
                    
                    // Load or create conversation
                    let mut history = if let Some(id) = conversation_id {
                        ConversationHistory::load(&state.db, id).await
                            .unwrap_or_else(|_| ConversationHistory::new())
                    } else {
                        ConversationHistory::new()
                    };
                    
                    // Process with streaming
                    match process_streaming(
                        &mut engine,
                        registry,
                        &mut history,
                        content,
                        &mut socket,
                    ).await {
                        Ok(response) => {
                            send_event(&mut socket, ServerEvent::Done {
                                conversation_id: history.conversation_id,
                                message: response,
                            }).await;
                            
                            // Save conversation
                            history.save(&state.db).await.ok();
                        }
                        Err(e) => {
                            send_event(&mut socket, ServerEvent::Error {
                                message: e.to_string(),
                            }).await;
                        }
                    }
                }
                
                ClientEvent::Ping => {
                    send_event(&mut socket, ServerEvent::Pong).await;
                }
            }
        }
    }
}

async fn send_event(socket: &mut WebSocket, event: ServerEvent) {
    let json = serde_json::to_string(&event).unwrap();
    socket.send(Message::Text(json)).await.ok();
}
```

### 5. Configuration

```toml
# config.toml

[server]
bind_addr = "0.0.0.0:8888"
cors_origins = ["https://app.example.com"]

[llm]
model_path = "/app/models/mistral-7b-instruct-v0.2.Q4_K_M.gguf"
context_size = 4096
n_gpu_layers = 0
threads = 4
temperature = 0.7
top_p = 0.9
max_tokens = 2048

[database]
url = "sqlite:///var/ai-runtime/conversations.db"

[mcp_servers]
[[mcp_servers]]
name = "mail"
url = "http://mcp-mail-server:8090"

[[mcp_servers]]
name = "chat"
url = "http://mcp-chat-server:8091"

[[mcp_servers]]
name = "dav"
url = "http://mcp-dav-server:8092"

[logging]
level = "info"
format = "json"

[metrics]
enabled = true
prometheus_port = 9091
```

## Tests

```rust
#[tokio::test]
async fn test_llm_generation() {
    let engine = LLMEngine::load(&test_config()).await.unwrap();
    let response = engine.generate(
        "What is 2+2?",
        GenerationParams::default()
    ).await.unwrap();
    
    assert!(response.contains("4"));
}

#[tokio::test]
async fn test_tool_calling() {
    let mut registry = MCPRegistry::new();
    // Register mock MCP server
    
    let tool_call = ToolCall {
        id: "call_1".to_string(),
        tool: "list_emails".to_string(),
        arguments: json!({ "mailbox": "INBOX", "limit": 10 }),
    };
    
    let result = registry.execute_tool(&tool_call).await.unwrap();
    assert!(result.error.is_none());
}
```

## Déploiement

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/ai-runtime /usr/local/bin/
COPY ./models /app/models
EXPOSE 8888 9091
CMD ["ai-runtime", "--config", "/etc/ai-runtime/config.toml"]
```

## Roadmap

### MVP - 3 semaines
- [x] LLM loading & inference (CPU)
- [x] MCP client + registry
- [x] Basic tool calling
- [x] WebSocket API
- [x] Conversation storage

### Post-MVP
- [ ] GPU support (CUDA)
- [ ] Smart context management
- [ ] Response caching
- [ ] Multi-model support
- [ ] Fine-tuning pipeline

## Métriques de succès

- ✅ Latency first token <500ms
- ✅ Tokens/second >20
- ✅ Tool call success rate >95%
- ✅ Context management sans perte info

## Notes

**C'est LE composant clé qui fait la différence**. Sans lui, le projet est juste un serveur mail de plus.
