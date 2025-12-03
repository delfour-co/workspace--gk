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

    #[serde(rename = "email_summaries")]
    EmailSummaries {
        count: usize,
        summaries: Vec<EmailSummaryInfo>,
    },

    #[serde(rename = "new_email_notification")]
    NewEmailNotification {
        from: String,
        subject: String,
        summary: String,
    },

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

#[derive(Debug, Serialize)]
struct EmailSummaryInfo {
    from: String,
    subject: String,
    summary: String,
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
    let mut conversation_history: Vec<Message> = Vec::new();

    // Subscribe to email notifications
    let mut notification_rx = state.email_notifier.subscribe();

    // Handle incoming messages and notifications
    loop {
        tokio::select! {
            // Handle WebSocket messages from client
            Some(Ok(msg)) = receiver.next() => {
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

                        // Send auth success
                        let auth_msg = ServerMessage::AuthSuccess {
                            email: email.clone(),
                        };
                        if let Ok(json) = serde_json::to_string(&auth_msg) {
                            let _ = sender.send(WsMessage::Text(json)).await;
                        }

                        // Load and send unread email summaries
                        match state.summary_store.get_unread_summaries(&email).await {
                            Ok(summaries) => {
                                if !summaries.is_empty() {
                                    info!(
                                        "📬 Sending {} unread email summaries to {}",
                                        summaries.len(),
                                        email
                                    );

                                    let summary_infos: Vec<EmailSummaryInfo> = summaries
                                        .iter()
                                        .map(|s| EmailSummaryInfo {
                                            from: s.from_addr.clone(),
                                            subject: s.subject.clone(),
                                            summary: s.summary.clone(),
                                        })
                                        .collect();

                                    let summaries_msg = ServerMessage::EmailSummaries {
                                        count: summary_infos.len(),
                                        summaries: summary_infos,
                                    };

                                    if let Ok(json) = serde_json::to_string(&summaries_msg) {
                                        let _ = sender.send(WsMessage::Text(json)).await;
                                    }

                                    // Mark all summaries as read after sending
                                    if let Err(e) = state.summary_store.mark_all_as_read(&email).await {
                                        warn!("⚠️  Failed to mark summaries as read: {}", e);
                                    } else {
                                        info!("✅ Marked all summaries as read for {}", email);
                                    }
                                }
                            }
                            Err(e) => {
                                warn!("⚠️  Failed to load summaries: {}", e);
                            }
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
                            &state,
                            &mut conversation_history
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
            },
            // Handle email notifications from broadcast channel
            Ok(notification) = notification_rx.recv() => {
                // Only send notification if it's for the authenticated user
                if let Some(ref user_email) = authenticated_email {
                    if &notification.user_email == user_email {
                        info!("📬 Sending new email notification to {}", user_email);

                        let notification_msg = ServerMessage::NewEmailNotification {
                            from: notification.summary.from_addr,
                            subject: notification.summary.subject,
                            summary: notification.summary.summary,
                        };

                        if let Ok(json) = serde_json::to_string(&notification_msg) {
                            let _ = sender.send(WsMessage::Text(json)).await;
                        }
                    }
                }
            },
            // Handle disconnection
            else => {
                info!("🔌 WebSocket connection ended");
                break;
            }
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
    conversation_history: &mut Vec<Message>,
) -> anyhow::Result<()> {
    info!("💬 Processing chat message from {}: {}", user_email, user_message);

    // Add system prompt if conversation is empty
    if conversation_history.is_empty() {
        conversation_history.push(Message {
            role: MessageRole::System,
            content: format!(
                "Tu es un assistant email intelligent en français pour l'utilisateur: {}.\n\n\
                IMPORTANT - Comment lire les emails:\n\
                1. Appelle list_emails pour voir les emails disponibles\n\
                2. Le résultat contient un champ 'id' pour chaque email (ex: '1763735627.170438.fedora')\n\
                3. Pour lire un email, appelle read_email avec LES DEUX paramètres:\n\
                   - email: '{}'\n\
                   - email_id: la valeur exacte du champ 'id' de list_emails (PAS le sujet!)\n\n\
                IMPORTANT - Mémoire de conversation:\n\
                - Tu te souviens des emails listés précédemment\n\
                - Quand l'utilisateur dit \"le premier\", \"le deuxième\", utilise l'id du dernier list_emails\n\
                - Garde en mémoire les résultats des outils pour répondre aux questions suivantes\n\n\
                Exemple:\n\
                User: \"Ai-je de nouveaux emails?\"\n\
                → Appelle list_emails(email='{}')\n\
                Résultat: [{{id: '123.456.fedora', subject: 'Test', from: 'alice@example.com'}}]\n\
                User: \"Lis le premier\"\n\
                → Appelle read_email(email='{}', email_id='123.456.fedora')\n\n\
                Réponds TOUJOURS en français de façon naturelle et conversationnelle.",
                user_email, user_email, user_email, user_email
            ),
        });
    }

    // Add user message to history
    conversation_history.push(Message {
        role: MessageRole::User,
        content: user_message.clone(),
    });

    // Create messages for LLM (full history)
    let messages = conversation_history.clone();

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

        // Format tool results as a message and add to history
        let tool_results_text = tool_results
            .iter()
            .map(|(name, result)| {
                format!("Outil '{}' a retourné: {}", name, serde_json::to_string_pretty(result).unwrap_or_default())
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        conversation_history.push(Message {
            role: MessageRole::Tool,
            content: tool_results_text,
        });

        // Call LLM again with full history including tool results
        let final_llm_response = state
            .llm
            .generate(conversation_history.clone(), None)
            .await?;

        final_llm_response.text
    } else {
        // No tool calls, use LLM's direct response
        llm_response.text
    };

    // Add assistant response to history
    conversation_history.push(Message {
        role: MessageRole::Assistant,
        content: final_response.clone(),
    });

    // Stream response word by word for better UX
    let words: Vec<&str> = final_response.split_whitespace().collect();
    let mut current_chunk = String::new();

    for (i, word) in words.iter().enumerate() {
        current_chunk.push_str(word);

        // Add space after each word except the last
        if i < words.len() - 1 {
            current_chunk.push(' ');
        }

        // Send chunk every 3-5 words or on last word
        if current_chunk.split_whitespace().count() >= 4 || i == words.len() - 1 {
            let chunk_msg = ServerMessage::Chunk {
                content: current_chunk.clone(),
            };
            if let Ok(json) = serde_json::to_string(&chunk_msg) {
                let _ = sender.send(WsMessage::Text(json)).await;
            }

            // Small delay for streaming effect (20ms)
            tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

            current_chunk.clear();
        }
    }

    // Send done message (empty content since we already streamed everything)
    let done_msg = ServerMessage::Done {
        content: String::new(),
    };
    let json = serde_json::to_string(&done_msg)?;
    sender.send(WsMessage::Text(json)).await?;

    info!("✅ Chat message processed");
    Ok(())
}
