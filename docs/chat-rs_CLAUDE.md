# chat-rs - Serveur de Messagerie Instantanée

## Vue d'ensemble

`chat-rs` est un serveur de messagerie instantanée écrit en Rust, avec support des rooms/channels, messages temps réel via WebSocket, et bridges vers WhatsApp/Google Chat via des connecteurs.

## Contexte du projet global

Composant de communication synchrone pour compléter le mail (asynchrone). Permet conversations en temps réel, notifications push, et intégration avec services externes.

### Interfaces avec les autres composants

- **Expose** : WebSocket API pour clients + REST API pour MCP
- **Consommé par** : `web-ui`, `ai-runtime` (via mcp-chat-server), apps mobiles
- **Derrière** : `proxy-rs` pour exposition HTTPS
- **Bridges** : WhatsApp, Google Chat (via APIs externes)

## Responsabilités

### Primaires
1. **Messaging temps réel**
   - Envoi/réception messages
   - Delivery receipts
   - Read receipts
   - Typing indicators
   - Online presence

2. **Rooms/Channels**
   - Conversations 1-to-1
   - Group chats
   - Channels publics/privés
   - Permissions (admin, member)

3. **Persistence**
   - Stockage messages
   - Historique conversations
   - Attachments (files, images)
   - Search full-text

4. **Bridges**
   - WhatsApp (via WhatsApp Business API ou mautrix-whatsapp)
   - Google Chat (via Google Chat API)
   - Messages bidirectionnels
   - Sync statuts

### Secondaires
- Push notifications (optionnel, via FCM/APNS)
- Message encryption E2E
- Voice/video calls (futur, WebRTC)
- Reactions emoji
- Message threading

## Architecture technique

### Stack Rust

```toml
[dependencies]
# WebSocket
axum = { version = "0.7", features = ["ws"] }
tokio-tungstenite = "0.21"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace", "fs"] }

# Async
tokio = { version = "1", features = ["full"] }
futures = "0.3"

# Database
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "uuid", "chrono"] }

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
bytes = "1"

# Crypto (E2E encryption)
ring = "0.17"
base64 = "0.21"

# Full-text search
tantivy = "0.21"  # Optionnel

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Config
toml = "0.8"

# HTTP client (pour bridges)
reqwest = { version = "0.11", features = ["json", "multipart"] }
```

### Structure du projet

```
chat-rs/
├── Cargo.toml
├── config.example.toml
├── README.md
├── Dockerfile
│
├── src/
│   ├── main.rs
│   ├── config.rs
│   ├── error.rs
│   │
│   ├── websocket/
│   │   ├── mod.rs
│   │   ├── server.rs         # WebSocket server
│   │   ├── handler.rs        # Connection handler
│   │   ├── events.rs         # Event types (Message, Typing, etc.)
│   │   └── broadcast.rs      # Message broadcasting
│   │
│   ├── api/
│   │   ├── mod.rs
│   │   ├── routes.rs         # REST endpoints
│   │   ├── auth.rs           # JWT authentication
│   │   └── handlers/
│   │       ├── rooms.rs
│   │       ├── messages.rs
│   │       └── users.rs
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   ├── user.rs
│   │   ├── room.rs
│   │   ├── message.rs
│   │   └── attachment.rs
│   │
│   ├── storage/
│   │   ├── mod.rs
│   │   ├── db.rs             # PostgreSQL operations
│   │   ├── migrations/
│   │   │   ├── 001_init.sql
│   │   │   ├── 002_rooms.sql
│   │   │   └── 003_messages.sql
│   │   └── files.rs          # Attachment storage
│   │
│   ├── presence/
│   │   ├── mod.rs
│   │   └── tracker.rs        # Online/offline tracking
│   │
│   ├── bridges/
│   │   ├── mod.rs
│   │   ├── whatsapp.rs       # WhatsApp bridge
│   │   ├── googlechat.rs     # Google Chat bridge
│   │   └── bridge_manager.rs
│   │
│   ├── search/
│   │   ├── mod.rs
│   │   └── indexer.rs        # Full-text search (Tantivy)
│   │
│   └── utils/
│       ├── mod.rs
│       └── crypto.rs         # E2E encryption helpers
│
├── tests/
│   ├── integration/
│   │   ├── websocket_test.rs
│   │   ├── api_test.rs
│   │   └── messages_test.rs
│   └── fixtures/
│
└── docs/
    ├── ARCHITECTURE.md
    ├── WEBSOCKET_API.md
    └── BRIDGES.md
```

## Spécifications fonctionnelles

### 1. WebSocket Protocol

**Connection**

```
WS /ws
Headers:
  Authorization: Bearer <jwt_token>
```

**Client → Server Events**

```json
// Send message
{
  "type": "message.send",
  "room_id": "uuid",
  "content": "Hello world",
  "reply_to": "message_id"  // Optional
}

// Typing indicator
{
  "type": "typing.start",
  "room_id": "uuid"
}

{
  "type": "typing.stop",
  "room_id": "uuid"
}

// Mark as read
{
  "type": "message.read",
  "room_id": "uuid",
  "message_id": "uuid"
}

// Presence
{
  "type": "presence.update",
  "status": "online" | "away" | "busy" | "offline"
}

// Ping
{
  "type": "ping"
}
```

**Server → Client Events**

```json
// New message
{
  "type": "message.new",
  "message": {
    "id": "uuid",
    "room_id": "uuid",
    "user_id": "uuid",
    "user_name": "Alice",
    "content": "Hello world",
    "created_at": "2024-11-18T10:30:00Z",
    "reply_to": null,
    "attachments": []
  }
}

// Message delivered
{
  "type": "message.delivered",
  "message_id": "uuid",
  "delivered_at": "2024-11-18T10:30:01Z"
}

// Message read
{
  "type": "message.read",
  "message_id": "uuid",
  "user_id": "uuid",
  "read_at": "2024-11-18T10:30:05Z"
}

// Typing indicator
{
  "type": "typing",
  "room_id": "uuid",
  "user_id": "uuid",
  "user_name": "Bob"
}

// Presence update
{
  "type": "presence",
  "user_id": "uuid",
  "status": "online",
  "last_seen": "2024-11-18T10:30:00Z"
}

// Room updated
{
  "type": "room.updated",
  "room": {
    "id": "uuid",
    "name": "Team Chat",
    "members": ["uuid1", "uuid2"]
  }
}

// Pong
{
  "type": "pong"
}

// Error
{
  "type": "error",
  "code": "INVALID_ROOM",
  "message": "Room not found"
}
```

### 2. REST API

**Endpoints**

```yaml
# Authentication
POST /api/auth/login
  Request: { email, password }
  Response: { token, user }

# Rooms
GET /api/rooms
  Response: [{ id, name, type, last_message, unread_count }]

POST /api/rooms
  Request: { name?, type: "direct" | "group", members: [user_ids] }
  Response: { id, name, type, members }

GET /api/rooms/:id
  Response: { id, name, type, members, created_at }

PATCH /api/rooms/:id
  Request: { name?, members? }

DELETE /api/rooms/:id

# Messages
GET /api/rooms/:room_id/messages?limit=50&before=<message_id>
  Response: [{
    id, user_id, user_name, content,
    created_at, reply_to, attachments
  }]

POST /api/rooms/:room_id/messages
  Request: { content, reply_to?, attachments? }
  Response: { id, created_at }

DELETE /api/messages/:id

# Attachments
POST /api/attachments
  Content-Type: multipart/form-data
  File: file
  Response: { id, url, filename, size, mime_type }

# Users
GET /api/users/:id
  Response: { id, name, email, avatar_url, status }

GET /api/users/search?q=alice
  Response: [{ id, name, email }]

# Presence
GET /api/presence/:user_id
  Response: { user_id, status, last_seen }
```

### 3. Database Schema

```sql
-- migrations/001_init.sql

CREATE TABLE users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    name VARCHAR(255) NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    avatar_url VARCHAR(500),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE rooms (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255),
    type VARCHAR(50) NOT NULL, -- 'direct', 'group', 'channel'
    created_by UUID REFERENCES users(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE room_members (
    room_id UUID REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(50) DEFAULT 'member', -- 'admin', 'member'
    joined_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (room_id, user_id)
);

CREATE TABLE messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    room_id UUID REFERENCES rooms(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    content TEXT NOT NULL,
    reply_to UUID REFERENCES messages(id),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    deleted_at TIMESTAMP WITH TIME ZONE
);

CREATE INDEX idx_messages_room_id ON messages(room_id, created_at DESC);
CREATE INDEX idx_messages_user_id ON messages(user_id);

CREATE TABLE attachments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id UUID REFERENCES messages(id) ON DELETE CASCADE,
    filename VARCHAR(255) NOT NULL,
    url VARCHAR(500) NOT NULL,
    size BIGINT NOT NULL,
    mime_type VARCHAR(100) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

CREATE TABLE message_receipts (
    message_id UUID REFERENCES messages(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    delivered_at TIMESTAMP WITH TIME ZONE,
    read_at TIMESTAMP WITH TIME ZONE,
    PRIMARY KEY (message_id, user_id)
);

CREATE TABLE presence (
    user_id UUID PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    status VARCHAR(50) NOT NULL, -- 'online', 'away', 'busy', 'offline'
    last_seen TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Full-text search (PostgreSQL)
CREATE INDEX idx_messages_content_fts ON messages
USING GIN (to_tsvector('english', content));
```

### 4. WebSocket Handler

```rust
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Extension(user): Extension<User>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state, user))
}

async fn handle_socket(
    mut socket: WebSocket,
    state: AppState,
    user: User,
) {
    // Subscribe to user's rooms
    let room_ids = get_user_rooms(&state.db, user.id).await.unwrap();
    let mut broadcast_rx = state.broadcaster.subscribe(&room_ids);
    
    // Update presence
    update_presence(&state.db, user.id, "online").await.ok();
    
    loop {
        tokio::select! {
            // Client → Server
            Some(Ok(msg)) = socket.recv() => {
                if let Message::Text(text) = msg {
                    match serde_json::from_str::<ClientEvent>(&text) {
                        Ok(event) => {
                            handle_client_event(
                                event,
                                &user,
                                &state,
                            ).await.ok();
                        }
                        Err(e) => {
                            error!("Failed to parse client event: {}", e);
                        }
                    }
                }
            }
            
            // Server → Client (broadcasts)
            Ok(server_event) = broadcast_rx.recv() => {
                let json = serde_json::to_string(&server_event).unwrap();
                socket.send(Message::Text(json)).await.ok();
            }
            
            else => break,
        }
    }
    
    // Cleanup
    update_presence(&state.db, user.id, "offline").await.ok();
}

async fn handle_client_event(
    event: ClientEvent,
    user: &User,
    state: &AppState,
) -> Result<()> {
    match event {
        ClientEvent::MessageSend { room_id, content, reply_to } => {
            // Validate user is member
            if !is_room_member(&state.db, &room_id, user.id).await? {
                return Err(Error::NotRoomMember);
            }
            
            // Store message
            let message = create_message(
                &state.db,
                &room_id,
                user.id,
                &content,
                reply_to,
            ).await?;
            
            // Broadcast to room
            state.broadcaster.broadcast(
                &room_id,
                ServerEvent::MessageNew { message },
            ).await;
            
            Ok(())
        }
        
        ClientEvent::TypingStart { room_id } => {
            state.broadcaster.broadcast(
                &room_id,
                ServerEvent::Typing {
                    room_id,
                    user_id: user.id,
                    user_name: user.name.clone(),
                },
            ).await;
            Ok(())
        }
        
        ClientEvent::MessageRead { room_id, message_id } => {
            mark_message_read(&state.db, &message_id, user.id).await?;
            
            state.broadcaster.broadcast(
                &room_id,
                ServerEvent::MessageRead {
                    message_id,
                    user_id: user.id,
                    read_at: Utc::now(),
                },
            ).await;
            
            Ok(())
        }
        
        _ => Ok(()),
    }
}
```

### 5. Message Broadcasting

```rust
use tokio::sync::broadcast;

pub struct Broadcaster {
    channels: Arc<Mutex<HashMap<Uuid, broadcast::Sender<ServerEvent>>>>,
}

impl Broadcaster {
    pub fn new() -> Self {
        Self {
            channels: Arc::new(Mutex::new(HashMap::new())),
        }
    }
    
    pub fn subscribe(&self, room_ids: &[Uuid]) -> MultiRoomReceiver {
        let mut receivers = Vec::new();
        let mut channels = self.channels.lock().unwrap();
        
        for room_id in room_ids {
            let tx = channels.entry(*room_id)
                .or_insert_with(|| broadcast::channel(100).0);
            
            receivers.push((*room_id, tx.subscribe()));
        }
        
        MultiRoomReceiver { receivers }
    }
    
    pub async fn broadcast(&self, room_id: &Uuid, event: ServerEvent) {
        let channels = self.channels.lock().unwrap();
        
        if let Some(tx) = channels.get(room_id) {
            tx.send(event).ok();
        }
    }
}

pub struct MultiRoomReceiver {
    receivers: Vec<(Uuid, broadcast::Receiver<ServerEvent>)>,
}

impl MultiRoomReceiver {
    pub async fn recv(&mut self) -> Result<ServerEvent> {
        // Try to receive from any room
        loop {
            for (_, rx) in &mut self.receivers {
                match rx.try_recv() {
                    Ok(event) => return Ok(event),
                    Err(broadcast::error::TryRecvError::Empty) => continue,
                    Err(_) => continue,
                }
            }
            
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }
}
```

### 6. Bridges

#### WhatsApp Bridge

```rust
// Utiliser mautrix-whatsapp ou WhatsApp Business API
pub struct WhatsAppBridge {
    api_url: String,
    api_token: String,
}

impl WhatsAppBridge {
    pub async fn send_message(
        &self,
        to: &str,
        message: &str,
    ) -> Result<()> {
        // WhatsApp Business API
        self.client
            .post(&format!("{}/messages", self.api_url))
            .bearer_auth(&self.api_token)
            .json(&json!({
                "messaging_product": "whatsapp",
                "to": to,
                "type": "text",
                "text": { "body": message }
            }))
            .send()
            .await?;
        
        Ok(())
    }
    
    pub async fn handle_webhook(
        &self,
        payload: WhatsAppWebhook,
    ) -> Result<()> {
        // Recevoir messages WhatsApp
        // Les convertir en messages internes
        for msg in payload.messages {
            let internal_msg = InternalMessage {
                room_id: get_or_create_whatsapp_room(&msg.from).await?,
                content: msg.text.body,
                user_id: get_or_create_whatsapp_user(&msg.from).await?,
                external_id: Some(msg.id),
            };
            
            create_message_from_bridge(internal_msg).await?;
        }
        
        Ok(())
    }
}
```

#### Google Chat Bridge

```rust
pub struct GoogleChatBridge {
    credentials: ServiceAccountCredentials,
}

impl GoogleChatBridge {
    pub async fn send_message(
        &self,
        space: &str,
        message: &str,
    ) -> Result<()> {
        let token = self.get_access_token().await?;
        
        self.client
            .post(&format!(
                "https://chat.googleapis.com/v1/{}/messages",
                space
            ))
            .bearer_auth(&token)
            .json(&json!({
                "text": message
            }))
            .send()
            .await?;
        
        Ok(())
    }
}
```

### 7. Configuration

```toml
# config.toml

[server]
bind_addr = "0.0.0.0:8081"
cors_origins = ["https://app.example.com"]

[database]
url = "postgres://user:pass@localhost/chat"
max_connections = 20

[storage]
attachments_path = "/var/chat/attachments"
max_attachment_size_mb = 10

[websocket]
ping_interval_seconds = 30
max_connections_per_user = 5

[bridges.whatsapp]
enabled = false
api_url = "https://graph.facebook.com/v18.0/YOUR_PHONE_NUMBER_ID"
api_token = "YOUR_WHATSAPP_TOKEN"
webhook_secret = "YOUR_WEBHOOK_SECRET"

[bridges.googlechat]
enabled = false
credentials_path = "/etc/chat/google-credentials.json"

[logging]
level = "info"
format = "json"
```

## Tests

```rust
#[tokio::test]
async fn test_send_message() {
    let state = create_test_state().await;
    let user = create_test_user(&state.db).await;
    let room = create_test_room(&state.db, &[user.id]).await;
    
    let event = ClientEvent::MessageSend {
        room_id: room.id,
        content: "Hello".to_string(),
        reply_to: None,
    };
    
    handle_client_event(event, &user, &state).await.unwrap();
    
    let messages = get_messages(&state.db, &room.id, 10).await.unwrap();
    assert_eq!(messages.len(), 1);
    assert_eq!(messages[0].content, "Hello");
}
```

## Déploiement

```dockerfile
FROM rust:1.75 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates libpq5 && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/chat-rs /usr/local/bin/
EXPOSE 8081
CMD ["chat-rs", "--config", "/etc/chat/config.toml"]
```

## Roadmap

### MVP - 4 semaines
- [x] WebSocket real-time messaging
- [x] Rooms (direct, group)
- [x] Persistence PostgreSQL
- [x] REST API
- [x] Presence tracking

### Post-MVP
- [ ] WhatsApp bridge
- [ ] Google Chat bridge
- [ ] E2E encryption
- [ ] Voice/video calls (WebRTC)
- [ ] Push notifications

## Métriques de succès

- ✅ Message latency <100ms (p95)
- ✅ Support 1000+ concurrent connections
- ✅ Message delivery >99.9%
- ✅ Uptime >99.9%
