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
    let smtp_host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let smtp_port = std::env::var("SMTP_PORT").unwrap_or_else(|_| "2525".to_string());
    let smtp_server = format!("{}:{}", smtp_host, smtp_port);

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
            description: "List recent emails from Maildir. Returns email metadata including 'email_id' field which is required to read the full email content with read_email tool.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address to list emails for (optional - if omitted, lists all emails)".to_string(),
                    param_type: "string".to_string(),
                    required: false,
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
            description: "Read the full content of a specific email. You MUST call list_emails first to get the email_id value for the email you want to read.".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address (e.g. test@example.com)".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "email_id".to_string(),
                    description: "Email ID from list_emails result (e.g. '1763735627.170438.fedora'). This is NOT the email subject - use the exact 'email_id' value returned by list_emails.".to_string(),
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
        Tool {
            name: "mark_as_read".to_string(),
            description: "Mark an email as read by moving it from new/ to cur/".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address (e.g. test@example.com)".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "email_id".to_string(),
                    description: "Email ID from list_emails result".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            server: "mail".to_string(),
        },
        Tool {
            name: "delete_email".to_string(),
            description: "Delete an email permanently".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address (e.g. test@example.com)".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
                ToolParameter {
                    name: "email_id".to_string(),
                    description: "Email ID from list_emails result".to_string(),
                    param_type: "string".to_string(),
                    required: true,
                },
            ],
            server: "mail".to_string(),
        },
        Tool {
            name: "get_email_count".to_string(),
            description: "Get the count of unread emails in new/ folder".to_string(),
            parameters: vec![
                ToolParameter {
                    name: "email".to_string(),
                    description: "Email address (e.g. test@example.com)".to_string(),
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
        "mark_as_read" => mark_as_read_tool(arguments, request.id).await,
        "delete_email" => delete_email_tool(arguments, request.id).await,
        "get_email_count" => get_email_count_tool(arguments, request.id).await,
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

    // Extract arguments - email is now optional
    let email_filter = arguments
        .get("email")
        .and_then(|v| v.as_str());

    let limit = arguments
        .get("limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(10) as usize;

    if let Some(email) = email_filter {
        info!("📬 Listing emails for: {}", email);
    } else {
        info!("📬 Listing all emails across all mailboxes");
    }

    let mut all_emails = Vec::new();

    if let Some(email) = email_filter {
        // List emails for specific address
        let maildir_path = format!("mail-rs/data/maildir/{}/new", email);
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

        all_emails = read_maildir_emails(path, email, limit)?;
    } else {
        // List emails from ALL mailboxes
        let maildir_root = Path::new("mail-rs/data/maildir");

        if maildir_root.exists() {
            if let Ok(entries) = fs::read_dir(maildir_root) {
                for entry in entries.filter_map(|e| e.ok()) {
                    if !entry.path().is_dir() {
                        continue;
                    }

                    let email_addr = entry.file_name().to_string_lossy().to_string();
                    let new_path = entry.path().join("new");

                    if new_path.exists() {
                        if let Ok(mut emails) = read_maildir_emails(&new_path, &email_addr, usize::MAX) {
                            all_emails.append(&mut emails);
                        }
                    }
                }
            }
        }

        // Sort all emails by modification time (newest first)
        all_emails.sort_by(|a, b| {
            let a_date = a.get("date").and_then(|d| d.as_str()).unwrap_or("");
            let b_date = b.get("date").and_then(|d| d.as_str()).unwrap_or("");
            b_date.cmp(a_date)
        });

        // Apply limit
        all_emails.truncate(limit);
    }

    info!("✅ Listed {} emails", all_emails.len());

    Ok(Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "emails": all_emails,
            "count": all_emails.len(),
        })),
        error: None,
        id,
    }))
}

/// Helper function to read emails from a maildir path
fn read_maildir_emails(
    path: &std::path::Path,
    email_addr: &str,
    limit: usize,
) -> Result<Vec<serde_json::Value>, (StatusCode, String)> {
    use std::fs;

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
                        "to": email_addr,
                        "from": from,
                        "subject": subject,
                        "date": date,
                    }));
                }
            }
        }
        Err(e) => {
            warn!("Failed to read maildir {}: {}", path.display(), e);
        }
    }

    Ok(emails)
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
    let email_path = format!("mail-rs/data/maildir/{}/new/{}", email, email_id);

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

    let maildir_path = format!("mail-rs/data/maildir/{}/new", email);
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

/// Mark email as read tool implementation
async fn mark_as_read_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;
    use std::path::Path;

    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    let email_id = arguments
        .get("email_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email_id' argument".to_string()))?;

    info!("📖 Marking email as read: {} for {}", email_id, email);

    // Move email from new/ to cur/
    let new_path = format!("mail-rs/data/maildir/{}/new/{}", email, email_id);
    let cur_path = format!("mail-rs/data/maildir/{}/cur/{}", email, email_id);

    match fs::rename(&new_path, &cur_path) {
        Ok(_) => {
            info!("✅ Email marked as read: {}", email_id);
            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "success": true,
                    "message": format!("Email {} marked as read", email_id)
                })),
                error: None,
                id,
            }))
        }
        Err(e) => {
            warn!("⚠️  Failed to mark email as read: {}", e);
            Ok(Json(McpResponse {
                jsonrpc: "2.0".to_string(),
                result: Some(serde_json::json!({
                    "success": false,
                    "error": format!("Failed to mark email as read: {}", e)
                })),
                error: None,
                id,
            }))
        }
    }
}

/// Delete email tool implementation
async fn delete_email_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;
    use std::path::Path;

    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    let email_id = arguments
        .get("email_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email_id' argument".to_string()))?;

    info!("🗑️  Deleting email: {} for {}", email_id, email);

    // Try to delete from new/ or cur/
    let new_path = format!("mail-rs/data/maildir/{}/new/{}", email, email_id);
    let cur_path = format!("mail-rs/data/maildir/{}/cur/{}", email, email_id);

    let deleted = if Path::new(&new_path).exists() {
        fs::remove_file(&new_path).is_ok()
    } else if Path::new(&cur_path).exists() {
        fs::remove_file(&cur_path).is_ok()
    } else {
        false
    };

    if deleted {
        info!("✅ Email deleted: {}", email_id);
        Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "success": true,
                "message": format!("Email {} deleted", email_id)
            })),
            error: None,
            id,
        }))
    } else {
        warn!("⚠️  Failed to delete email: not found");
        Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "success": false,
                "error": "Email not found or already deleted"
            })),
            error: None,
            id,
        }))
    }
}

/// Get email count tool implementation
async fn get_email_count_tool(
    arguments: HashMap<String, serde_json::Value>,
    id: u64,
) -> Result<Json<McpResponse>, (StatusCode, String)> {
    use std::fs;
    use std::path::Path;

    let email = arguments
        .get("email")
        .and_then(|v| v.as_str())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "Missing 'email' argument".to_string()))?;

    info!("📊 Getting email count for: {}", email);

    let maildir_path = format!("mail-rs/data/maildir/{}/new", email);
    let path = Path::new(&maildir_path);

    if !path.exists() {
        return Ok(Json(McpResponse {
            jsonrpc: "2.0".to_string(),
            result: Some(serde_json::json!({
                "count": 0,
                "email": email
            })),
            error: None,
            id,
        }));
    }

    let count = match fs::read_dir(path) {
        Ok(entries) => entries.filter(|e| e.is_ok()).count(),
        Err(_) => 0,
    };

    info!("📧 Found {} unread emails for {}", count, email);

    Ok(Json(McpResponse {
        jsonrpc: "2.0".to_string(),
        result: Some(serde_json::json!({
            "count": count,
            "email": email
        })),
        error: None,
        id,
    }))
}
