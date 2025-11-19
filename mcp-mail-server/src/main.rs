//! mcp-mail-server - MCP Server for mail-rs
//!
//! Exposes mail-rs functionality via the Model Context Protocol (MCP)

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
struct McpRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: u64,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
    id: u64,
}

/// MCP error
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

/// Tool definition
#[derive(Debug, Serialize)]
struct Tool {
    name: String,
    description: String,
    parameters: Vec<ToolParameter>,
    server: String,
}

#[derive(Debug, Serialize)]
struct ToolParameter {
    name: String,
    description: String,
    #[serde(rename = "type")]
    param_type: String,
    required: bool,
}

/// Application state
struct AppState {
    smtp_server: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting mcp-mail-server...");

    // Configuration
    let smtp_server = std::env::var("SMTP_SERVER").unwrap_or_else(|_| "127.0.0.1:2525".to_string());

    let state = Arc::new(AppState {
        smtp_server: smtp_server.clone(),
    });

    info!("📧 Using SMTP server: {}", smtp_server);

    // Build router
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/mcp", post(mcp_handler))
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:8090";
    info!("🌐 MCP server listening on http://{}", addr);
    info!("📋 Available tools: send_email");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "mcp-mail-server",
        "version": "0.1.0"
    }))
}

/// MCP endpoint handler
async fn mcp_handler(
    State(state): State<Arc<AppState>>,
    Json(request): Json<McpRequest>,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    debug!("📥 MCP request: method={}", request.method);

    match request.method.as_str() {
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(state, request).await,
        _ => Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
            id: request.id,
        })),
    }
}

/// Handle tools/list
fn handle_tools_list(id: u64) -> Result<Json<McpResponse>, (StatusCode, String)> {
    debug!("📋 Listing available tools");

    let tools = vec![
        Tool {
            name: "send_email".to_string(),
            description: "Send an email via SMTP".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "to".to_string(),
                    description: "Recipient email address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "subject".to_string(),
                    description: "Email subject".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "body".to_string(),
                    description: "Email body".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            server: "mail".to_string(),
        },
        Tool {
            name: "list_emails".to_string(),
            description: "List recent emails from Maildir".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address to list emails for".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "limit".to_string(),
                    description: "Maximum number of emails to return (default: 10)".to_string(),
                    param_type: "number".to_string(),
                    required: false,
                },
            ],
            server: "mail".to_string(),
        },
        Tool {
            name: "read_email".to_string(),
            description: "Read a specific email by ID".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "email_id".to_string(),
                    description: "Email ID (filename)".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            server: "mail".to_string(),
        },
        Tool {
            name: "search_emails".to_string(),
            description: "Search emails by keyword".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address to search in".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "query".to_string(),
                    description: "Search query (searches in subject and body)".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            server: "mail".to_string(),
        },
    ];

    Ok(Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::to_value(tools).unwrap()),
        error: None,
        id,
    }))
}

/// Handle tools/call
async fn handle_tools_call(
    state: Arc<AppState>,
    request: McpRequest,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    // Parse params
    let params: HashMap<String, serde_json::Value> = serde_json::from_value(request.params)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let tool_name = params
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing tool name".to_string()))?;

    let arguments: HashMap<String, serde_json::Value> = params
        .get("arguments")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing arguments".to_string()))?;

    debug!("🔧 Calling tool: {} with args: {:?}", tool_name, arguments);

    match tool_name {
        "send_email" => send_email_tool(state, arguments, request.id).await,
        "list_emails" => list_emails_tool(arguments, request.id).await,
        "read_email" => read_email_tool(arguments, request.id).await,
        "search_emails" => search_emails_tool(arguments, request.id).await,
        _ => Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(McpError {
                code: -32601,
                message: format!("Tool not found: {}", tool_name),
            }),
            id: request.id,
        })),
    }
}

/// Send email tool implementation
async fn send_email_tool(
    state: Arc<AppState>,
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    // Extract arguments
    let to = arguments
        .get("to")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'to' argument".to_string()))?;

    let subject = arguments
        .get("subject")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'subject' argument".to_string()))?;

    let body = arguments
        .get("body")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'body' argument".to_string()))?;

    info!("📧 Sending email to: {}, subject: {}", to, subject);

    // Use mail-rs SMTP client to send email
    match send_email_smtp(&state.smtp_server, to, subject, body).await {
        Ok(_) => {
            info!("✅ Email sent successfully");
            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "success": true,
                    "message": format!("Email sent to {}", to)
                })),
                error: None,
                id,
            }))
        }
        Err(e) => {
            warn!("❌ Failed to send email: {}", e);
            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(McpError {
                    code: -32000,
                    message: format!("Failed to send email: {}", e),
                }),
                id,
            }))
        }
    }
}

/// Send email via SMTP
async fn send_email_smtp(
    smtp_server: &str,
    to: &str,
    subject: &str,
    body: &str,
) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::TcpStream;

    debug!("🔌 Connecting to SMTP server: {}", smtp_server);

    // Connect to SMTP server
    let mut stream = TcpStream::connect(smtp_server).await?;
    let (reader, mut writer) = stream.split();
    let mut reader = BufReader::new(reader);

    // Read greeting
    let mut line = String::new();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    // EHLO
    writer.write_all(b"EHLO mcp-mail-server\r\n").await?;
    loop {
        line.clear();
        reader.read_line(&mut line).await?;
        debug!("← {}", line.trim());
        if line.starts_with("250 ") {
            break;
        }
    }

    // MAIL FROM
    writer
        .write_all(b"MAIL FROM:<ai@example.com>\r\n")
        .await?;
    line.clear();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    // RCPT TO
    writer
        .write_all(format!("RCPT TO:<{}>\r\n", to).as_bytes())
        .await?;
    line.clear();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    // DATA
    writer.write_all(b"DATA\r\n").await?;
    line.clear();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    // Email content
    let email_data = format!(
        "From: AI Assistant <ai@example.com>\r\n\
         To: <{}>\r\n\
         Subject: {}\r\n\
         \r\n\
         {}\r\n\
         .\r\n",
        to, subject, body
    );

    writer.write_all(email_data.as_bytes()).await?;
    line.clear();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    // QUIT
    writer.write_all(b"QUIT\r\n").await?;
    line.clear();
    reader.read_line(&mut line).await?;
    debug!("← {}", line.trim());

    Ok(())
}

/// List emails tool implementation
async fn list_emails_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;
    use std::path::Path;

    // Extract arguments
    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    info!("📬 Listing emails for: {}", email);

    // Read from Maildir
    let maildir_path = format!("/tmp/maildir/{}/new", email);
    let path = Path::new(&maildir_path);

    if !path.exists() {
        return Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "emails": [],
                "count": 0,
                "message": format!("No mailbox found for {}", email)
            })),
            error: None,
            id,
        }));
    }

    let mut emails = Vec::new();
    match fs::read_dir(path) {
        Ok(entries) => {
            let mut files: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .collect();

            // Sort by modification time (newest first)
            files.sort_by(|a, b| {
                let a_time = a.metadata().ok().and_then(|m| m.modified().ok());
                let b_time = b.metadata().ok().and_then(|m| m.modified().ok());
                b_time.cmp(&a_time)
            });

            for entry in files.iter().take(limit) {
                let filename = entry.file_name().to_string_lossy().to_string();
                let path = entry.path();

                // Read email headers
                if let Ok(content) = fs::read_to_string(&path) {
                    let mut from = String::new();
                    let mut subject = String::new();
                    let mut date = String::new();

                    for line in content.lines() {
                        if line.is_empty() {
                            break; // End of headers
                        }
                        if line.starts_with("From:") {
                            from = line[5..].trim().to_string();
                        } else if line.starts_with("Subject:") {
                            subject = line[8..].trim().to_string();
                        } else if line.starts_with("Date:") {
                            date = line[5..].trim().to_string();
                        }
                    }

                    emails.push(serde_json::json!({
                        "id": filename,
                        "from": from,
                        "subject": subject,
                        "date": date,
                    }));
                }
            }
        }
        Err(e) => {
            warn!("Failed to read maildir: {}", e);
        }
    }

    info!("✅ Listed {} emails", emails.len());

    Ok(Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "emails": emails,
            "count": emails.len(),
        })),
        error: None,
        id,
    }))
}

/// Read email tool implementation
async fn read_email_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;

    // Extract arguments
    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    let email_id = arguments
        .get("email_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email_id' argument".to_string()))?;

    info!("📧 Reading email: {} for {}", email_id, email);

    // Read email file
    let email_path = format!("/tmp/maildir/{}/new/{}", email, email_id);

    match fs::read_to_string(&email_path) {
        Ok(content) => {
            // Parse email
            let mut headers = HashMap::new();
            let mut body = String::new();
            let mut in_body = false;

            for line in content.lines() {
                if in_body {
                    body.push_str(line);
                    body.push('\n');
                } else if line.is_empty() {
                    in_body = true;
                } else if let Some(colon_pos) = line.find(':') {
                    let key = line[..colon_pos].to_string();
                    let value = line[colon_pos + 1..].trim().to_string();
                    headers.insert(key, value);
                }
            }

            info!("✅ Email read successfully");

            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "id": email_id,
                    "headers": headers,
                    "body": body.trim(),
                })),
                error: None,
                id,
            }))
        }
        Err(e) => {
            warn!("❌ Failed to read email: {}", e);
            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: None,
                error: Some(McpError {
                    code: -32000,
                    message: format!("Failed to read email: {}", e),
                }),
                id,
            }))
        }
    }
}

/// Search emails tool implementation
async fn search_emails_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;
    use std::path::Path;

    // Extract arguments
    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    let query = arguments
        .get("query")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'query' argument".to_string()))?;

    info!("🔍 Searching emails for: {} with query: {}", email, query);

    let maildir_path = format!("/tmp/maildir/{}/new", email);
    let path = Path::new(&maildir_path);

    if !path.exists() {
        return Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "emails": [],
                "count": 0,
                "message": format!("No mailbox found for {}", email)
            })),
            error: None,
            id,
        }));
    }

    let query_lower = query.to_lowercase();
    let mut matching_emails = Vec::new();

    match fs::read_dir(path) {
        Ok(entries) => {
            for entry in entries.filter_map(|e| e.ok()) {
                if !entry.path().is_file() {
                    continue;
                }

                let filename = entry.file_name().to_string_lossy().to_string();

                if let Ok(content) = fs::read_to_string(entry.path()) {
                    let content_lower = content.to_lowercase();

                    if content_lower.contains(&query_lower) {
                        // Extract headers
                        let mut from = String::new();
                        let mut subject = String::new();

                        for line in content.lines() {
                            if line.is_empty() {
                                break;
                            }
                            if line.starts_with("From:") {
                                from = line[5..].trim().to_string();
                            } else if line.starts_with("Subject:") {
                                subject = line[8..].trim().to_string();
                            }
                        }

                        matching_emails.push(serde_json::json!({
                            "id": filename,
                            "from": from,
                            "subject": subject,
                        }));
                    }
                }
            }
        }
        Err(e) => {
            warn!("Failed to read maildir: {}", e);
        }
    }

    info!("✅ Found {} matching emails", matching_emails.len());

    Ok(Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "emails": matching_emails,
            "count": matching_emails.len(),
            "query": query,
        })),
        error: None,
        id,
    }))
}
