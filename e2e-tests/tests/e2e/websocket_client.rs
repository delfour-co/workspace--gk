use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { email: String },
    #[serde(rename = "chat")]
    Chat { message: String },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "auth_success")]
    AuthSuccess { email: String },
    #[serde(rename = "chunk")]
    Chunk { content: String },
    #[serde(rename = "tool_call")]
    ToolCall {
        tool: String,
        arguments: Value,
    },
    #[serde(rename = "tool_result")]
    ToolResult { tool: String, result: Value },
    #[serde(rename = "done")]
    Done { content: String },
    #[serde(rename = "error")]
    Error { message: String },
}

pub struct WebSocketTestClient {
    write: futures_util::stream::SplitSink<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        Message,
    >,
    read: futures_util::stream::SplitStream<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
}

impl WebSocketTestClient {
    /// Connect to WebSocket server
    pub async fn connect(url: &str) -> Result<Self, String> {
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| format!("Failed to connect to WebSocket: {}", e))?;

        let (write, read) = ws_stream.split();

        Ok(Self { write, read })
    }

    /// Send a message
    pub async fn send(&mut self, msg: ClientMessage) -> Result<(), String> {
        let json = serde_json::to_string(&msg)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;

        self.write
            .send(Message::Text(json))
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;

        Ok(())
    }

    /// Receive a message with timeout
    pub async fn receive(&mut self, timeout_secs: u64) -> Result<ServerMessage, String> {
        let result = timeout(Duration::from_secs(timeout_secs), self.read.next()).await;

        match result {
            Ok(Some(Ok(Message::Text(text)))) => {
                serde_json::from_str(&text)
                    .map_err(|e| format!("Failed to parse message: {}", e))
            }
            Ok(Some(Ok(_))) => Err("Received non-text message".to_string()),
            Ok(Some(Err(e))) => Err(format!("WebSocket error: {}", e)),
            Ok(None) => Err("WebSocket closed".to_string()),
            Err(_) => Err(format!("Timeout after {} seconds", timeout_secs)),
        }
    }

    /// Authenticate
    pub async fn authenticate(&mut self, email: &str) -> Result<(), String> {
        self.send(ClientMessage::Auth {
            email: email.to_string(),
        })
        .await?;

        // Wait for auth_success
        match self.receive(10).await? {
            ServerMessage::AuthSuccess { email: _ } => Ok(()),
            ServerMessage::Error { message } => {
                Err(format!("Authentication failed: {}", message))
            }
            _ => Err("Unexpected response to auth".to_string()),
        }
    }

    /// Send chat message and collect all responses until done
    pub async fn chat(&mut self, message: &str) -> Result<ChatResponse, String> {
        self.send(ClientMessage::Chat {
            message: message.to_string(),
        })
        .await?;

        let mut response = ChatResponse::default();
        let max_messages = 50; // Prevent infinite loops
        let mut count = 0;

        loop {
            count += 1;
            if count > max_messages {
                return Err("Too many messages received".to_string());
            }

            match self.receive(60).await? {
                ServerMessage::Chunk { content } => {
                    response.chunks.push(content);
                }
                ServerMessage::ToolCall { tool, arguments } => {
                    response.tool_calls.push((tool, arguments));
                }
                ServerMessage::ToolResult { tool, result } => {
                    response.tool_results.push((tool, result));
                }
                ServerMessage::Done { content } => {
                    response.final_content = Some(content);
                    break;
                }
                ServerMessage::Error { message } => {
                    return Err(format!("Chat error: {}", message));
                }
                _ => {
                    return Err("Unexpected message type".to_string());
                }
            }
        }

        Ok(response)
    }

    /// Close connection
    pub async fn close(mut self) -> Result<(), String> {
        self.write
            .close()
            .await
            .map_err(|e| format!("Failed to close connection: {}", e))
    }
}

#[derive(Debug, Default)]
pub struct ChatResponse {
    pub chunks: Vec<String>,
    pub tool_calls: Vec<(String, Value)>,
    pub tool_results: Vec<(String, Value)>,
    pub final_content: Option<String>,
}

impl ChatResponse {
    /// Check if a specific tool was called
    pub fn has_tool_call(&self, tool_name: &str) -> bool {
        self.tool_calls.iter().any(|(name, _)| name == tool_name)
    }

    /// Get tool result by name
    pub fn get_tool_result(&self, tool_name: &str) -> Option<&Value> {
        self.tool_results
            .iter()
            .find(|(name, _)| name == tool_name)
            .map(|(_, result)| result)
    }

    /// Get final content or panic
    pub fn final_content(&self) -> &str {
        self.final_content
            .as_ref()
            .expect("No final content received")
    }
}
