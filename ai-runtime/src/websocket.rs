//! WebSocket handler for real-time chat streaming

use axum::{
    extract::{
        ws::{Message as WsMessage, WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
};
use futures::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{debug, error, info, warn};

use crate::{
    llm::{LlmEngine, Message, MessageRole},
    mcp::McpRegistry,
    AppState,
};

/// WebSocket message from client
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { email: String },

    #[serde(rename = "chat")]
    Chat { message: String },
}

/// WebSocket message to client
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess { email: String },

    #[serde(rename = "chunk")]
    Chunk { content: String },

    #[serde(rename = "tool_call")]
    ToolCall {
        tool: String,
        arguments: serde_json::Value,
    },

    #[serde(rename = "tool_result")]
    ToolResult {
        tool: String,
        result: serde_json::Value,
    },

    #[serde(rename = "done")]
    Done { content: String },

    #[serde(rename = "error")]
    Error { message: String },
}

/// WebSocket upgrade handler
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

/// Handle WebSocket connection
async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    info!("🔌 New WebSocket connection");

    let (mut sender, mut receiver) = socket.split();
    let mut authenticated_email: Option<String> = None;

    // Handle incoming messages
    while let Some(Ok(msg)) = receiver.next().await {
        if let WsMessage::Text(text) = msg {
            debug!("📥 Received message: {}", text);

            // Parse client message
            let client_msg: ClientMessage = match serde_json::from_str(&text) {
                Ok(msg) => msg,
                Err(e) => {
                    error!("Failed to parse message: {}", e);
                    let error_msg = ServerMessage::Error {
                        message: format!("Invalid message format: {}", e),
                    };
                    if let Ok(json) = serde_json::to_string(&error_msg) {
                        let _ = sender.send(WsMessage::Text(json)).await;
                    }
                    continue;
                }
            };

            // Handle messages
            match client_msg {
                ClientMessage::Auth { email } => {
                    info!("🔐 Authentication request for: {}", email);

                    // Simple email validation
                    if email.contains('@') && !email.is_empty() {
                        authenticated_email = Some(email.clone());
                        info!("✅ User authenticated: {}", email);

                        let auth_msg = ServerMessage::AuthSuccess { email };
                        if let Ok(json) = serde_json::to_string(&auth_msg) {
                            let _ = sender.send(WsMessage::Text(json)).await;
                        }
                    } else {
                        let error_msg = ServerMessage::Error {
                            message: "Invalid email format".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let _ = sender.send(WsMessage::Text(json)).await;
                        }
                    }
                }

                ClientMessage::Chat { message } => {
                    // Check authentication
                    if let Some(ref user_email) = authenticated_email {
                        if let Err(e) = handle_chat_message(
                            message,
                            user_email.clone(),
                            &mut sender,
                            &state
                        ).await {
                            error!("Error handling chat: {}", e);
                            let error_msg = ServerMessage::Error {
                                message: format!("Chat error: {}", e),
                            };
                            if let Ok(json) = serde_json::to_string(&error_msg) {
                                let _ = sender.send(WsMessage::Text(json)).await;
                            }
                        }
                    } else {
                        let error_msg = ServerMessage::Error {
                            message: "Not authenticated. Please send auth message first.".to_string(),
                        };
                        if let Ok(json) = serde_json::to_string(&error_msg) {
                            let _ = sender.send(WsMessage::Text(json)).await;
                        }
                    }
                }
            }
        } else if let WsMessage::Close(_) = msg {
            info!("🔌 WebSocket closed by client");
            break;
        }
    }

    info!("🔌 WebSocket connection closed");
}

/// Handle a chat message
async fn handle_chat_message(
    user_message: String,
    user_email: String,
    sender: &mut futures::stream::SplitSink<WebSocket, WsMessage>,
    state: &Arc<AppState>,
) -> anyhow::Result<()> {
    info!("💬 Processing chat message from {}: {}", user_email, user_message);

    // Create LLM messages with system context
    let messages = vec![
        Message {
            role: MessageRole::System,
            content: format!(
                "You are an AI email assistant for user: {}.\n\n\
                IMPORTANT - How to read emails:\n\
                1. First call list_emails to see available emails\n\
                2. The list_emails result includes an 'email_id' field for each email (e.g. '1763735627.170438.fedora')\n\
                3. To read an email, call read_email with BOTH parameters:\n\
                   - email: '{}'\n\
                   - email_id: the exact 'email_id' value from list_emails (NOT the subject!)\n\n\
                Example workflow:\n\
                User: \"Read the E2E Test Email\"\n\
                Step 1: Call list_emails with email='{}'\n\
                Result: [{{'email_id': '1763735627.170438.fedora', 'subject': 'E2E Test Email - 2025-11-21 15:33:47', ...}}]\n\
                Step 2: Call read_email with email='{}' and email_id='1763735627.170438.fedora'\n\n\
                Remember: email_id is the file identifier, NOT the subject line!",
                user_email, user_email, user_email, user_email
            ),
        },
        Message {
            role: MessageRole::User,
            content: user_message.clone(),
        },
    ];

    // Get tools from MCP registry
    let registry = state.mcp_registry.lock().await;
    let tool_schemas = registry.get_tool_schemas();
    drop(registry);

    // Generate LLM response
    debug!("🤖 Calling LLM with {} tools", tool_schemas.len());
    let llm_response = state
        .llm
        .generate(
            messages,
            if tool_schemas.is_empty() {
                None
            } else {
                Some(tool_schemas)
            },
        )
        .await?;

    // Don't send response text as chunk - we'll send it in "done" instead
    // to avoid duplication

    // Execute tool calls if any
    let final_response = if !llm_response.tool_calls.is_empty() {
        info!("🔧 Executing {} tool calls", llm_response.tool_calls.len());

        let registry = state.mcp_registry.lock().await;
        let mut tool_results = Vec::new();

        for tool_call in &llm_response.tool_calls {
            // Send tool call notification
            let tool_call_msg = ServerMessage::ToolCall {
                tool: tool_call.name.clone(),
                arguments: serde_json::to_value(&tool_call.arguments)?,
            };
            let json = serde_json::to_string(&tool_call_msg)?;
            sender.send(WsMessage::Text(json)).await?;

            // Execute tool
            debug!("🔧 Executing tool: {}", tool_call.name);
            let result = registry
                .call_tool(&tool_call.name, tool_call.arguments.clone())
                .await?;

            // Send tool result
            let tool_result_msg = ServerMessage::ToolResult {
                tool: tool_call.name.clone(),
                result: result.clone(),
            };
            let json = serde_json::to_string(&tool_result_msg)?;
            debug!("📤 Sending tool result: {} bytes", json.len());
            sender.send(WsMessage::Text(json)).await?;

            tool_results.push((tool_call.name.clone(), result));
        }
        drop(registry);

        // Second LLM call with tool results to generate natural language response
        info!("🤖 Calling LLM again to generate natural language response");

        // Format tool results as a message
        let tool_results_text = tool_results
            .iter()
            .map(|(name, result)| {
                format!("Tool '{}' returned: {}", name, serde_json::to_string_pretty(result).unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let messages_with_results = vec![
            Message {
                role: MessageRole::System,
                content: format!(
                    "You are an AI email assistant for user: {}. \
                    Answer the user's question in French using the tool results provided. \
                    Be concise and natural. Format the response in a user-friendly way. \
                    If the user asked to read an email and you received the full content, \
                    present the key information (sender, subject, date, body) in a clear format.",
                    user_email
                ),
            },
            Message {
                role: MessageRole::User,
                content: user_message.clone(),
            },
            Message {
                role: MessageRole::Tool,
                content: tool_results_text,
            },
        ];

        // Call LLM again to generate final response
        let final_llm_response = state
            .llm
            .generate(messages_with_results, None)
            .await?;

        final_llm_response.text
    } else {
        // No tool calls, use LLM's direct response
        llm_response.text
    };

    // Send done message
    let done_msg = ServerMessage::Done {
        content: final_response,
    };
    let json = serde_json::to_string(&done_msg)?;
    sender.send(WsMessage::Text(json)).await?;

    info!("✅ Chat message processed");
    Ok(())
}
