# mcp-mail-server - MCP Server pour Mail

## Vue d'ensemble

`mcp-mail-server` expose les fonctionnalités du serveur mail (`mail-rs`) via le protocole MCP (Model Context Protocol). Il sert d'interface entre le LLM (dans `ai-runtime`) et le système mail, permettant à l'assistant de lire, envoyer, chercher des emails via des tool calls.

## Contexte du projet global

Ce composant est le **pont** entre l'IA et le système mail. Il traduit les intentions du LLM en actions concrètes sur les emails.

### Interfaces avec les autres composants

- **Consomme** : `mail-rs` REST API (http://mail-rs:8080/api)
- **Expose** : MCP JSON-RPC API sur HTTP (port 8090)
- **Appelé par** : `ai-runtime` (MCP client)

## Responsabilités

### Primaires
1. **Implémentation protocole MCP**
   - Endpoint `tools/list` : Liste tools disponibles
   - Endpoint `tools/call` : Exécute tool calls

2. **Tools Mail**
   - `list_emails` : Lister emails d'une mailbox
   - `read_email` : Lire un email complet
   - `send_email` : Envoyer un email
   - `search_emails` : Rechercher emails
   - `delete_email` : Supprimer email
   - `move_email` : Déplacer email vers dossier
   - `mark_email` : Marquer comme lu/important/etc.

3. **Transformation données**
   - REST API responses → MCP tool results
   - MCP tool arguments → REST API requests
   - Formatage user-friendly pour LLM

### Secondaires
- Validation arguments tool calls
- Gestion erreurs mail API
- Rate limiting
- Logs/traces

## Architecture technique

### Stack Rust

```toml
[dependencies]
# HTTP server
axum = "0.7"
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "trace"] }

# HTTP client (to mail-rs API)
reqwest = { version = "0.11", features = ["json"] }

# MCP protocol
serde = { version = "1", features = ["derive"] }
serde_json = "1"
schemars = "0.8"  # JSON Schema generation

# Async
tokio = { version = "1", features = ["full"] }

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Utils
chrono = { version = "0.4", features = ["serde"] }
uuid = { version = "1", features = ["v4"] }

# Config
toml = "0.8"
```

### Structure du projet

```
mcp-mail-server/
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
│   ├── mcp/
│   │   ├── mod.rs
│   │   ├── protocol.rs     # MCP types (Request, Response, Tool, etc.)
│   │   ├── server.rs       # MCP JSON-RPC handler
│   │   └── tools.rs        # Tool definitions (schemas)
│   │
│   ├── mail/
│   │   ├── mod.rs
│   │   ├── client.rs       # HTTP client to mail-rs API
│   │   └── types.rs        # Mail API types
│   │
│   ├── handlers/
│   │   ├── mod.rs
│   │   ├── list_emails.rs  # Tool: list_emails
│   │   ├── read_email.rs   # Tool: read_email
│   │   ├── send_email.rs   # Tool: send_email
│   │   ├── search_emails.rs
│   │   ├── delete_email.rs
│   │   ├── move_email.rs
│   │   └── mark_email.rs
│   │
│   └── utils/
│       ├── mod.rs
│       └── formatting.rs   # Format data for LLM consumption
│
├── tests/
│   └── integration_test.rs
│
└── docs/
    └── TOOLS.md
```

## Spécifications fonctionnelles

### 1. MCP Protocol

**Endpoints**

```
POST /mcp
Content-Type: application/json

Request body: MCPRequest
Response body: MCPResponse
```

**Protocol Types**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPRequest {
    pub jsonrpc: String,  // "2.0"
    pub method: String,   // "tools/list" or "tools/call"
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MCPResponse {
    pub jsonrpc: String,  // "2.0"
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub inputSchema: serde_json::Value,  // JSON Schema
}
```

**Method: `tools/list`**

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "tools/list",
  "params": {},
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "tools": [
      {
        "name": "list_emails",
        "description": "List emails from a mailbox",
        "inputSchema": {
          "type": "object",
          "properties": {
            "mailbox": {
              "type": "string",
              "description": "Mailbox name (e.g., 'INBOX', 'Sent')",
              "default": "INBOX"
            },
            "limit": {
              "type": "integer",
              "description": "Maximum number of emails to return",
              "default": 20
            },
            "unread_only": {
              "type": "boolean",
              "description": "Only return unread emails",
              "default": false
            }
          }
        }
      },
      // ... more tools
    ]
  },
  "id": 1
}
```

**Method: `tools/call`**

```json
// Request
{
  "jsonrpc": "2.0",
  "method": "tools/call",
  "params": {
    "name": "list_emails",
    "arguments": {
      "mailbox": "INBOX",
      "limit": 10,
      "unread_only": true
    }
  },
  "id": 2
}

// Response (success)
{
  "jsonrpc": "2.0",
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Found 3 unread emails in INBOX:\n\n1. From: alice@example.com\n   Subject: Q4 Planning\n   Date: 2024-11-18 10:30\n   Preview: Let's schedule a meeting to discuss...\n\n2. From: bob@example.com\n   Subject: Bug report\n   Date: 2024-11-18 09:15\n   Preview: I found an issue with the login...\n\n3. From: team@company.com\n   Subject: Newsletter November\n   Date: 2024-11-17 14:00\n   Preview: This month's updates include..."
      }
    ]
  },
  "id": 2
}

// Response (error)
{
  "jsonrpc": "2.0",
  "error": {
    "code": -32603,
    "message": "Failed to fetch emails: Connection refused"
  },
  "id": 2
}
```

### 2. Tool Definitions

#### Tool: `list_emails`

```rust
pub const LIST_EMAILS_TOOL: Tool = Tool {
    name: "list_emails",
    description: "List emails from a specific mailbox. Returns a summary of recent emails with sender, subject, date, and preview.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "mailbox": {
                "type": "string",
                "description": "Mailbox name (INBOX, Sent, Drafts, Trash, or custom folder)",
                "default": "INBOX"
            },
            "limit": {
                "type": "integer",
                "description": "Maximum number of emails to return (1-100)",
                "default": 20,
                "minimum": 1,
                "maximum": 100
            },
            "unread_only": {
                "type": "boolean",
                "description": "If true, only return unread emails",
                "default": false
            }
        }
    }),
};

pub async fn handle_list_emails(
    mail_client: &MailClient,
    args: ListEmailsArgs,
) -> Result<MCPToolResult> {
    // 1. Call mail-rs API
    let emails = mail_client.list_emails(
        &args.mailbox,
        args.limit,
        args.unread_only,
    ).await?;
    
    // 2. Format for LLM
    let formatted = format_emails_summary(&emails);
    
    // 3. Return MCP result
    Ok(MCPToolResult {
        content: vec![
            MCPContent::Text {
                text: formatted,
            }
        ],
    })
}

fn format_emails_summary(emails: &[EmailSummary]) -> String {
    if emails.is_empty() {
        return "No emails found.".to_string();
    }
    
    let mut output = format!("Found {} email(s):\n\n", emails.len());
    
    for (i, email) in emails.iter().enumerate() {
        output.push_str(&format!(
            "{}. From: {}\n   Subject: {}\n   Date: {}\n   Preview: {}\n   ID: {}\n\n",
            i + 1,
            email.from,
            email.subject,
            email.date.format("%Y-%m-%d %H:%M"),
            &email.preview[..email.preview.len().min(100)],
            email.id,
        ));
    }
    
    output
}
```

#### Tool: `read_email`

```rust
pub const READ_EMAIL_TOOL: Tool = Tool {
    name: "read_email",
    description: "Read the full content of a specific email by its ID. Returns the complete email body.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "email_id": {
                "type": "string",
                "description": "The unique ID of the email to read"
            }
        },
        "required": ["email_id"]
    }),
};

pub async fn handle_read_email(
    mail_client: &MailClient,
    args: ReadEmailArgs,
) -> Result<MCPToolResult> {
    let email = mail_client.get_email(&args.email_id).await?;
    
    let formatted = format!(
        "From: {}\nTo: {}\nSubject: {}\nDate: {}\n\n{}",
        email.from,
        email.to.join(", "),
        email.subject,
        email.date.format("%Y-%m-%d %H:%M"),
        email.body_text.as_ref()
            .unwrap_or(&email.body_html.clone().unwrap_or_default())
    );
    
    Ok(MCPToolResult {
        content: vec![
            MCPContent::Text { text: formatted }
        ],
    })
}
```

#### Tool: `send_email`

```rust
pub const SEND_EMAIL_TOOL: Tool = Tool {
    name: "send_email",
    description: "Send a new email. IMPORTANT: Always confirm with the user before sending.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "to": {
                "type": "array",
                "items": { "type": "string" },
                "description": "Recipient email addresses"
            },
            "subject": {
                "type": "string",
                "description": "Email subject line"
            },
            "body": {
                "type": "string",
                "description": "Email body (plain text)"
            },
            "cc": {
                "type": "array",
                "items": { "type": "string" },
                "description": "CC recipients (optional)"
            },
            "bcc": {
                "type": "array",
                "items": { "type": "string" },
                "description": "BCC recipients (optional)"
            }
        },
        "required": ["to", "subject", "body"]
    }),
};

pub async fn handle_send_email(
    mail_client: &MailClient,
    args: SendEmailArgs,
) -> Result<MCPToolResult> {
    // Validate email addresses
    for addr in &args.to {
        if !is_valid_email(addr) {
            return Err(Error::InvalidEmail(addr.clone()));
        }
    }
    
    // Send via mail-rs API
    let result = mail_client.send_email(SendEmailRequest {
        to: args.to,
        cc: args.cc.unwrap_or_default(),
        bcc: args.bcc.unwrap_or_default(),
        subject: args.subject,
        body_text: args.body,
        body_html: None,
        attachments: vec![],
    }).await?;
    
    Ok(MCPToolResult {
        content: vec![
            MCPContent::Text {
                text: format!(
                    "Email sent successfully. Message ID: {}",
                    result.message_id
                ),
            }
        ],
    })
}
```

#### Tool: `search_emails`

```rust
pub const SEARCH_EMAILS_TOOL: Tool = Tool {
    name: "search_emails",
    description: "Search emails by keywords. Searches in subject, sender, and body.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "string",
                "description": "Search query (keywords)"
            },
            "mailbox": {
                "type": "string",
                "description": "Mailbox to search in (optional, searches all if not specified)"
            },
            "limit": {
                "type": "integer",
                "description": "Maximum results to return",
                "default": 20
            }
        },
        "required": ["query"]
    }),
};

pub async fn handle_search_emails(
    mail_client: &MailClient,
    args: SearchEmailsArgs,
) -> Result<MCPToolResult> {
    let results = mail_client.search_emails(
        &args.query,
        args.mailbox.as_deref(),
        args.limit.unwrap_or(20),
    ).await?;
    
    let formatted = format_emails_summary(&results);
    
    Ok(MCPToolResult {
        content: vec![
            MCPContent::Text { text: formatted }
        ],
    })
}
```

#### Tool: `delete_email`

```rust
pub const DELETE_EMAIL_TOOL: Tool = Tool {
    name: "delete_email",
    description: "Delete an email by moving it to Trash. IMPORTANT: Confirm with user before deleting.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "email_id": {
                "type": "string",
                "description": "ID of the email to delete"
            }
        },
        "required": ["email_id"]
    }),
};
```

#### Tool: `move_email`

```rust
pub const MOVE_EMAIL_TOOL: Tool = Tool {
    name: "move_email",
    description: "Move an email to a different mailbox/folder.",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "email_id": {
                "type": "string",
                "description": "ID of the email to move"
            },
            "to_mailbox": {
                "type": "string",
                "description": "Destination mailbox name (e.g., 'Archive', 'Work')"
            }
        },
        "required": ["email_id", "to_mailbox"]
    }),
};
```

#### Tool: `mark_email`

```rust
pub const MARK_EMAIL_TOOL: Tool = Tool {
    name: "mark_email",
    description: "Mark an email with flags (read, starred, etc.).",
    inputSchema: json!({
        "type": "object",
        "properties": {
            "email_id": {
                "type": "string",
                "description": "ID of the email to mark"
            },
            "flag": {
                "type": "string",
                "enum": ["read", "unread", "starred", "unstarred"],
                "description": "Flag to set"
            }
        },
        "required": ["email_id", "flag"]
    }),
};
```

### 3. Mail API Client

```rust
// src/mail/client.rs

pub struct MailClient {
    http_client: reqwest::Client,
    base_url: String,
    auth_token: String,
}

impl MailClient {
    pub fn new(base_url: String, auth_token: String) -> Self {
        Self {
            http_client: reqwest::Client::new(),
            base_url,
            auth_token,
        }
    }
    
    pub async fn list_emails(
        &self,
        mailbox: &str,
        limit: usize,
        unread_only: bool,
    ) -> Result<Vec<EmailSummary>> {
        let url = format!(
            "{}/api/mailbox/{}/messages?limit={}&unread={}",
            self.base_url, mailbox, limit, unread_only
        );
        
        let resp = self.http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            return Err(Error::MailAPIError(resp.status()));
        }
        
        let emails: Vec<EmailSummary> = resp.json().await?;
        Ok(emails)
    }
    
    pub async fn get_email(&self, email_id: &str) -> Result<Email> {
        let url = format!("{}/api/message/{}", self.base_url, email_id);
        
        let resp = self.http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            return Err(Error::MailAPIError(resp.status()));
        }
        
        let email: Email = resp.json().await?;
        Ok(email)
    }
    
    pub async fn send_email(
        &self,
        request: SendEmailRequest,
    ) -> Result<SendEmailResponse> {
        let url = format!("{}/api/message/send", self.base_url);
        
        let resp = self.http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&request)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            return Err(Error::MailAPIError(resp.status()));
        }
        
        let result: SendEmailResponse = resp.json().await?;
        Ok(result)
    }
    
    pub async fn search_emails(
        &self,
        query: &str,
        mailbox: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EmailSummary>> {
        let mut url = format!(
            "{}/api/search?q={}&limit={}",
            self.base_url,
            urlencoding::encode(query),
            limit
        );
        
        if let Some(mb) = mailbox {
            url.push_str(&format!("&mailbox={}", mb));
        }
        
        let resp = self.http_client
            .get(&url)
            .bearer_auth(&self.auth_token)
            .send()
            .await?;
        
        if !resp.status().is_success() {
            return Err(Error::MailAPIError(resp.status()));
        }
        
        let emails: Vec<EmailSummary> = resp.json().await?;
        Ok(emails)
    }
}
```

### 4. Configuration

```toml
# config.toml

[server]
bind_addr = "0.0.0.0:8090"

[mail_api]
base_url = "http://mail-rs:8080"
# Auth token for mail API (JWT)
auth_token = "eyJhbGc..."  # À générer/configurer

[logging]
level = "info"
format = "json"
```

## Tests

```rust
#[tokio::test]
async fn test_tools_list() {
    let app = create_test_app().await;
    
    let req = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/list".to_string(),
        params: json!({}),
        id: 1,
    };
    
    let resp = app.handle_request(req).await.unwrap();
    
    assert_eq!(resp.jsonrpc, "2.0");
    assert!(resp.result.is_some());
    
    let tools: Vec<Tool> = serde_json::from_value(
        resp.result.unwrap()
    ).unwrap();
    
    assert!(tools.iter().any(|t| t.name == "list_emails"));
    assert!(tools.iter().any(|t| t.name == "send_email"));
}

#[tokio::test]
async fn test_list_emails_tool() {
    // Mock mail API
    let mock_server = MockServer::start().await;
    mock_server
        .mock_get("/api/mailbox/INBOX/messages")
        .with_json_body(json!([
            {
                "id": "1",
                "from": "test@example.com",
                "subject": "Test",
                "date": "2024-11-18T10:00:00Z",
                "preview": "Hello world"
            }
        ]))
        .create()
        .await;
    
    let app = create_test_app_with_url(mock_server.url()).await;
    
    let req = MCPRequest {
        jsonrpc: "2.0".to_string(),
        method: "tools/call".to_string(),
        params: json!({
            "name": "list_emails",
            "arguments": {
                "mailbox": "INBOX",
                "limit": 10
            }
        }),
        id: 2,
    };
    
    let resp = app.handle_request(req).await.unwrap();
    
    assert!(resp.error.is_none());
    assert!(resp.result.is_some());
}
```

## Déploiement

```dockerfile
FROM rust:1.75-alpine AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM alpine:3.19
COPY --from=builder /app/target/release/mcp-mail-server /usr/local/bin/
EXPOSE 8090
CMD ["mcp-mail-server", "--config", "/etc/mcp-mail/config.toml"]
```

## Roadmap

### MVP - 1 semaine
- [x] MCP protocol implementation
- [x] Basic tools (list, read, send)
- [x] Mail API client

### Post-MVP
- [ ] Attachments support
- [ ] Advanced search filters
- [ ] Batch operations
- [ ] Email templates

## Métriques de succès

- ✅ Tool call success rate >98%
- ✅ Latency <100ms
- ✅ Error messages claires pour LLM

## Notes

**Ce composant est simple mais critique** : il doit fournir des réponses formatées de manière à ce que le LLM puisse les comprendre et les présenter à l'utilisateur de façon naturelle.
