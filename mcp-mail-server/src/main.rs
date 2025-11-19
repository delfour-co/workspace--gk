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
        .write_all(b"MAIL FROM:<ai@localhost>\r\n")
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
        "From: AI Assistant <ai@localhost>\r\n\
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
