# web-ui - Interface Conversationnelle

## Vue d'ensemble

`web-ui` est l'interface utilisateur web qui expose l'expérience conversationnelle AI-native du système. Au lieu d'interfaces classiques avec boutons et formulaires, l'utilisateur dialogue avec un assistant qui pilote tous les services.

**C'EST L'INTERFACE UTILISATEUR PRINCIPALE ET LE POINT D'ENTRÉE DE L'EXPÉRIENCE.**

## Contexte du projet global

Cette interface est la **vitrine** du projet. Elle matérialise la vision AI-native et différencie complètement le système d'une suite mail/chat classique.

### Interfaces avec les autres composants

- **Consomme** : `ai-runtime` WebSocket API (ws://ai-runtime:8888/ws)
- **Optionnel** : APIs directes des services (mail-rs, chat-rs) pour features avancées
- **Derrière** : `proxy-rs` (https://app.example.com)

## Responsabilités

### Primaires
1. **Interface Conversationnelle**
   - Chat interface (message input, history)
   - Streaming responses du LLM
   - Typing indicators, thinking states
   - Markdown rendering
   - Code syntax highlighting

2. **Authentication**
   - Login/logout
   - JWT token management
   - Session persistence

3. **Conversation Management**
   - Liste conversations
   - Créer/supprimer conversations
   - Export conversations

4. **Real-time Updates**
   - WebSocket connection au ai-runtime
   - Reconnection automatique
   - Offline handling

### Secondaires (optionnel)
- Settings/preferences
- Dark mode
- Notifications
- Shortcuts clavier
- Voice input (speech-to-text)

## Architecture technique

### Stack Tech

**Option 1 : Rust Full-Stack (Leptos)**

```toml
[dependencies]
leptos = { version = "0.5", features = ["csr"] }
leptos_router = "0.5"
leptos_meta = "0.5"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "WebSocket",
    "MessageEvent",
    "CloseEvent",
] }
gloo-net = "0.4"  # HTTP requests
gloo-timers = "0.3"
```

**Option 2 : Rust Backend + HTMX Frontend (Plus simple MVP)**

Backend (Axum):
```toml
[dependencies]
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["fs", "trace"] }
askama = "0.12"  # Templates
tokio = { version = "1", features = ["full"] }
```

Frontend (HTMX + Alpine.js):
- HTMX pour interactions serveur
- Alpine.js pour state local minimal
- Tailwind CSS pour styling
- Markdown-it pour rendering markdown

**Recommandation MVP : Option 2 (Rust + HTMX)**
- Plus rapide à développer
- Moins de complexité
- Progressive enhancement
- Peut migrer vers Leptos plus tard si besoin

### Structure du projet (Option HTMX)

```
web-ui/
├── Cargo.toml
├── README.md
├── Dockerfile
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── routes/
│   │   ├── mod.rs
│   │   ├── auth.rs           # Login, logout
│   │   ├── conversations.rs  # List, create, view
│   │   └── websocket.rs      # WS proxy to ai-runtime
│   │
│   ├── templates/
│   │   ├── base.html         # Layout de base
│   │   ├── login.html
│   │   ├── chat.html         # Interface conversationnelle
│   │   └── partials/
│   │       ├── message.html
│   │       ├── thinking.html
│   │       └── sidebar.html
│   │
│   ├── middleware/
│   │   ├── mod.rs
│   │   └── auth.rs           # JWT auth middleware
│   │
│   └── ai_client/
│       ├── mod.rs
│       └── client.rs         # Client vers ai-runtime
│
├── static/
│   ├── css/
│   │   └── styles.css        # Tailwind build output
│   ├── js/
│   │   ├── app.js            # Alpine.js app
│   │   ├── websocket.js      # WS connection management
│   │   └── markdown.js       # Markdown rendering
│   └── icons/
│
├── tailwind.config.js
├── package.json              # Pour build CSS/JS
│
└── docs/
    └── DESIGN.md
```

## Spécifications fonctionnelles

### 1. Interface Conversationnelle

**Layout**

```
┌─────────────────────────────────────────────────────┐
│  [Logo]  Personal AI Assistant         [Settings] ⚙ │
├───────────┬─────────────────────────────────────────┤
│           │                                         │
│ Sidebar   │         Conversation Area              │
│           │                                         │
│ New Chat  │  ┌──────────────────────────────────┐ │
│ ───────   │  │ User: Check my emails            │ │
│           │  └──────────────────────────────────┘ │
│ Today     │                                         │
│  Chat 1   │  ┌──────────────────────────────────┐ │
│  Chat 2   │  │ AI: [Thinking...]                │ │
│           │  └──────────────────────────────────┘ │
│ Yesterday │                                         │
│  Chat 3   │  ┌──────────────────────────────────┐ │
│           │  │ AI: You have 3 unread emails...  │ │
│           │  └──────────────────────────────────┘ │
│           │                                         │
│           │  ┌──────────────────────────────────┐ │
│           │  │ User: Reply to Alice saying...   │ │
│           │  └──────────────────────────────────┘ │
│           │                                         │
├───────────┴─────────────────────────────────────────┤
│  [Type a message...]                      [Send] → │
└─────────────────────────────────────────────────────┘
```

**HTML Structure (HTMX)**

```html
<!-- templates/chat.html -->
{% extends "base.html" %}

{% block content %}
<div class="flex h-screen">
  <!-- Sidebar -->
  <aside class="w-64 bg-gray-100 border-r">
    <div class="p-4">
      <button 
        hx-post="/conversations/new"
        hx-target="#conversation-area"
        class="w-full btn btn-primary"
      >
        + New Chat
      </button>
    </div>
    
    <div id="conversations-list">
      {% include "partials/sidebar.html" %}
    </div>
  </aside>
  
  <!-- Main conversation area -->
  <main class="flex-1 flex flex-col">
    <div id="messages" class="flex-1 overflow-y-auto p-4 space-y-4">
      <!-- Messages dynamically loaded -->
    </div>
    
    <!-- Input area -->
    <div class="border-t p-4">
      <form 
        hx-post="/message/send"
        hx-target="#messages"
        hx-swap="beforeend"
        hx-on::after-request="this.reset()"
        class="flex gap-2"
      >
        <input 
          type="text" 
          name="content"
          placeholder="Type a message..."
          class="flex-1 input"
          autofocus
        />
        <button type="submit" class="btn btn-primary">
          Send →
        </button>
      </form>
    </div>
  </main>
</div>

<script src="/js/websocket.js"></script>
{% endblock %}
```

**Message Templates**

```html
<!-- templates/partials/message.html -->

<!-- User message -->
<div class="flex justify-end">
  <div class="max-w-2xl bg-blue-500 text-white rounded-lg p-4">
    <div class="prose prose-invert">
      {{ content | safe }}
    </div>
    <div class="text-xs text-blue-100 mt-2">
      {{ timestamp }}
    </div>
  </div>
</div>

<!-- AI message -->
<div class="flex justify-start">
  <div class="max-w-2xl bg-gray-200 rounded-lg p-4">
    <div class="prose">
      {{ content | safe }}
    </div>
    <div class="text-xs text-gray-500 mt-2">
      {{ timestamp }}
    </div>
  </div>
</div>

<!-- Thinking indicator -->
<div class="flex justify-start">
  <div class="bg-gray-200 rounded-lg p-4">
    <div class="flex items-center gap-2">
      <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-900"></div>
      <span class="text-gray-600">Thinking...</span>
    </div>
  </div>
</div>

<!-- Tool execution indicator -->
<div class="flex justify-start">
  <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
    <div class="flex items-center gap-2">
      <svg class="w-4 h-4 text-blue-500" fill="none" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
      </svg>
      <span class="text-sm text-blue-700">Searching emails...</span>
    </div>
  </div>
</div>
```

### 2. WebSocket Connection

**JavaScript (static/js/websocket.js)**

```javascript
class ConversationWebSocket {
  constructor(conversationId) {
    this.conversationId = conversationId;
    this.ws = null;
    this.reconnectAttempts = 0;
    this.maxReconnectAttempts = 5;
    this.connect();
  }
  
  connect() {
    const protocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
    const url = `${protocol}//${window.location.host}/ws`;
    
    this.ws = new WebSocket(url);
    
    this.ws.onopen = () => {
      console.log('WebSocket connected');
      this.reconnectAttempts = 0;
      this.showConnectionStatus('connected');
    };
    
    this.ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      this.handleMessage(data);
    };
    
    this.ws.onerror = (error) => {
      console.error('WebSocket error:', error);
      this.showConnectionStatus('error');
    };
    
    this.ws.onclose = () => {
      console.log('WebSocket closed');
      this.showConnectionStatus('disconnected');
      this.reconnect();
    };
  }
  
  reconnect() {
    if (this.reconnectAttempts < this.maxReconnectAttempts) {
      this.reconnectAttempts++;
      const delay = Math.min(1000 * Math.pow(2, this.reconnectAttempts), 30000);
      console.log(`Reconnecting in ${delay}ms (attempt ${this.reconnectAttempts})...`);
      setTimeout(() => this.connect(), delay);
    } else {
      this.showConnectionStatus('failed');
    }
  }
  
  handleMessage(data) {
    switch (data.type) {
      case 'thinking':
        this.showThinkingIndicator();
        break;
        
      case 'tool_call':
        this.showToolExecution(data.tool, data.description);
        break;
        
      case 'token':
        this.appendToken(data.content);
        break;
        
      case 'done':
        this.hideThinkingIndicator();
        this.finalizeMessage(data.message);
        break;
        
      case 'error':
        this.showError(data.message);
        break;
        
      case 'pong':
        // Heartbeat response
        break;
    }
  }
  
  sendMessage(content) {
    if (this.ws.readyState === WebSocket.OPEN) {
      this.ws.send(JSON.stringify({
        type: 'message',
        conversation_id: this.conversationId,
        content: content
      }));
    } else {
      this.showError('Connection lost. Trying to reconnect...');
    }
  }
  
  showThinkingIndicator() {
    const messagesDiv = document.getElementById('messages');
    const thinkingHtml = `
      <div id="thinking-indicator" class="flex justify-start">
        <div class="bg-gray-200 rounded-lg p-4">
          <div class="flex items-center gap-2">
            <div class="animate-spin rounded-full h-4 w-4 border-b-2 border-gray-900"></div>
            <span class="text-gray-600">Thinking...</span>
          </div>
        </div>
      </div>
    `;
    messagesDiv.insertAdjacentHTML('beforeend', thinkingHtml);
    messagesDiv.scrollTop = messagesDiv.scrollHeight;
  }
  
  hideThinkingIndicator() {
    const indicator = document.getElementById('thinking-indicator');
    if (indicator) indicator.remove();
  }
  
  showToolExecution(tool, description) {
    // Replace thinking with tool execution
    this.hideThinkingIndicator();
    
    const messagesDiv = document.getElementById('messages');
    const toolHtml = `
      <div id="tool-indicator" class="flex justify-start">
        <div class="bg-blue-50 border border-blue-200 rounded-lg p-3">
          <div class="flex items-center gap-2">
            <svg class="w-4 h-4 text-blue-500" fill="none" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 10V3L4 14h7v7l9-11h-7z"/>
            </svg>
            <span class="text-sm text-blue-700">${description}</span>
          </div>
        </div>
      </div>
    `;
    messagesDiv.insertAdjacentHTML('beforeend', toolHtml);
    messagesDiv.scrollTop = messagesDiv.scrollHeight;
  }
  
  appendToken(token) {
    let streamingMsg = document.getElementById('streaming-message');
    
    if (!streamingMsg) {
      // Create streaming message div
      const indicator = document.getElementById('tool-indicator');
      if (indicator) indicator.remove();
      
      const messagesDiv = document.getElementById('messages');
      const msgHtml = `
        <div id="streaming-message" class="flex justify-start">
          <div class="max-w-2xl bg-gray-200 rounded-lg p-4">
            <div class="prose" id="streaming-content"></div>
          </div>
        </div>
      `;
      messagesDiv.insertAdjacentHTML('beforeend', msgHtml);
      streamingMsg = document.getElementById('streaming-message');
    }
    
    const content = document.getElementById('streaming-content');
    content.textContent += token;
    
    // Render markdown
    content.innerHTML = marked.parse(content.textContent);
    
    // Scroll
    const messagesDiv = document.getElementById('messages');
    messagesDiv.scrollTop = messagesDiv.scrollHeight;
  }
  
  finalizeMessage(message) {
    const streamingMsg = document.getElementById('streaming-message');
    if (streamingMsg) {
      streamingMsg.id = ''; // Remove temporary ID
    }
  }
  
  showConnectionStatus(status) {
    // Show connection status indicator
    const statusDiv = document.getElementById('connection-status');
    if (statusDiv) {
      statusDiv.textContent = status;
      statusDiv.className = `status-${status}`;
    }
  }
  
  showError(message) {
    // Show error message
    const messagesDiv = document.getElementById('messages');
    const errorHtml = `
      <div class="flex justify-center">
        <div class="bg-red-100 border border-red-300 rounded-lg p-4">
          <span class="text-red-700">${message}</span>
        </div>
      </div>
    `;
    messagesDiv.insertAdjacentHTML('beforeend', errorHtml);
  }
}

// Initialize
let ws;
document.addEventListener('DOMContentLoaded', () => {
  const conversationId = document.body.dataset.conversationId;
  if (conversationId) {
    ws = new ConversationWebSocket(conversationId);
  }
  
  // Override form submission to use WebSocket
  const form = document.querySelector('form');
  form.addEventListener('submit', (e) => {
    e.preventDefault();
    const input = form.querySelector('input[name="content"]');
    const content = input.value.trim();
    
    if (content && ws) {
      // Add user message to UI immediately
      addUserMessage(content);
      
      // Send via WebSocket
      ws.sendMessage(content);
      
      // Clear input
      input.value = '';
    }
  });
});

function addUserMessage(content) {
  const messagesDiv = document.getElementById('messages');
  const msgHtml = `
    <div class="flex justify-end">
      <div class="max-w-2xl bg-blue-500 text-white rounded-lg p-4">
        <div class="prose prose-invert">
          ${content}
        </div>
        <div class="text-xs text-blue-100 mt-2">
          Just now
        </div>
      </div>
    </div>
  `;
  messagesDiv.insertAdjacentHTML('beforeend', msgHtml);
  messagesDiv.scrollTop = messagesDiv.scrollHeight;
}
```

### 3. Backend Routes (Rust/Axum)

```rust
// src/routes/mod.rs

use axum::{
    routing::{get, post},
    Router,
};

pub fn routes() -> Router {
    Router::new()
        // Auth
        .route("/login", get(auth::login_page).post(auth::login))
        .route("/logout", post(auth::logout))
        
        // Conversations
        .route("/", get(conversations::index))
        .route("/conversations/new", post(conversations::create))
        .route("/conversations/:id", get(conversations::view))
        .route("/conversations/:id/delete", post(conversations::delete))
        
        // WebSocket
        .route("/ws", get(websocket::handler))
        
        // Static files
        .nest_service("/static", ServeDir::new("static"))
}
```

```rust
// src/routes/websocket.rs

pub async fn handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_websocket(socket, state, user))
}

async fn handle_websocket(
    mut client_socket: WebSocket,
    state: AppState,
    user: User,
) {
    // Connect to ai-runtime WebSocket
    let ai_ws_url = format!("{}/ws", state.config.ai_runtime_url);
    
    let (mut ai_socket, _) = match tokio_tungstenite::connect_async(&ai_ws_url).await {
        Ok(s) => s,
        Err(e) => {
            error!("Failed to connect to ai-runtime: {}", e);
            return;
        }
    };
    
    // Proxy messages bidirectionally
    loop {
        tokio::select! {
            // Client → AI Runtime
            Some(Ok(msg)) = client_socket.recv() => {
                if let Message::Text(text) = msg {
                    ai_socket.send(Message::Text(text)).await.ok();
                }
            }
            
            // AI Runtime → Client
            Some(Ok(msg)) = ai_socket.next() => {
                if let Message::Text(text) = msg {
                    client_socket.send(Message::Text(text)).await.ok();
                }
            }
            
            else => break,
        }
    }
}
```

### 4. Authentication

```rust
// src/routes/auth.rs

pub async fn login_page() -> impl IntoResponse {
    Html(include_str!("../templates/login.html"))
}

pub async fn login(
    State(state): State<AppState>,
    Form(credentials): Form<LoginForm>,
) -> Result<impl IntoResponse> {
    // Validate credentials against mail-rs API
    let token = authenticate_user(
        &state.mail_api_client,
        &credentials.email,
        &credentials.password,
    ).await?;
    
    // Set cookie
    let cookie = Cookie::build("token", token)
        .path("/")
        .http_only(true)
        .secure(true)
        .max_age(Duration::days(7))
        .finish();
    
    Ok((
        StatusCode::OK,
        [(SET_COOKIE, cookie.to_string())],
        Redirect::to("/"),
    ))
}
```

### 5. Configuration

```toml
# config.toml

[server]
bind_addr = "0.0.0.0:3000"
base_url = "https://app.example.com"

[ai_runtime]
url = "http://ai-runtime:8888"
websocket_url = "ws://ai-runtime:8888/ws"

[mail_api]
url = "http://mail-rs:8080"

[session]
secret = "changeme"
max_age_days = 7

[logging]
level = "info"
```

## Tests

```rust
#[tokio::test]
async fn test_login() {
    let app = create_test_app().await;
    
    let resp = app.post("/login")
        .form(&LoginForm {
            email: "user@test.com".to_string(),
            password: "password".to_string(),
        })
        .send()
        .await
        .unwrap();
    
    assert_eq!(resp.status(), 200);
    assert!(resp.headers().get("set-cookie").is_some());
}
```

## Déploiement

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

# Build frontend assets
FROM node:20 AS frontend
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tailwind.config.js ./
COPY static/ ./static/
RUN npm run build

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/web-ui /usr/local/bin/
COPY --from=frontend /app/static /app/static
COPY templates /app/templates
EXPOSE 3000
CMD ["web-ui", "--config", "/etc/web-ui/config.toml"]
```

## Roadmap

### MVP - 4 semaines
- [x] Login/auth
- [x] Chat interface
- [x] WebSocket streaming
- [x] Markdown rendering
- [x] Conversations list

### Post-MVP
- [ ] Dark mode
- [ ] Voice input
- [ ] File uploads
- [ ] Export conversations
- [ ] Keyboard shortcuts
- [ ] Mobile responsive

## Métriques de succès

- ✅ Interface intuitive (user feedback)
- ✅ Latency première réponse <1s
- ✅ Streaming fluide sans blocage
- ✅ Support desktop + mobile

## Notes

**L'UX doit être simple et naturelle**. L'utilisateur ne doit pas penser "comment faire X", il doit juste demander en langage naturel.
