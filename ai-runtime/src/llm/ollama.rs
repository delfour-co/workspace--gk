//! Ollama LLM implementation
//!
//! This implementation uses Ollama's HTTP API with function calling support.

use super::{LlmEngine, LlmResponse, Message, MessageRole, ToolCall};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{debug, warn};

/// Ollama LLM implementation
pub struct OllamaLlm {
    model_name: String,
    base_url: String,
    client: reqwest::Client,
}

impl OllamaLlm {
    pub fn new(model_name: String) -> Self {
        Self {
            model_name,
            base_url: "http://localhost:11434".to_string(),
            client: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }
}

/// Ollama chat request
#[derive(Debug, Serialize)]
struct OllamaChatRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    stream: bool,
}

/// Ollama message
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaMessage {
    role: String,
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

/// Ollama tool call
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaToolCall {
    function: OllamaFunction,
}

/// Ollama function
#[derive(Debug, Serialize, Deserialize, Clone)]
struct OllamaFunction {
    name: String,
    arguments: HashMap<String, serde_json::Value>,
}

/// Ollama chat response
#[derive(Debug, Deserialize)]
struct OllamaChatResponse {
    message: OllamaMessage,
    done: bool,
}

#[async_trait::async_trait]
impl LlmEngine for OllamaLlm {
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
    ) -> Result<LlmResponse> {
        debug!("OllamaLLM: Processing {} messages with model {}", messages.len(), self.model_name);

        // Convert messages to Ollama format
        let ollama_messages: Vec<OllamaMessage> = messages
            .into_iter()
            .map(|m| OllamaMessage {
                role: match m.role {
                    MessageRole::User => "user".to_string(),
                    MessageRole::Assistant => "assistant".to_string(),
                    MessageRole::System => "system".to_string(),
                    MessageRole::Tool => "tool".to_string(),
                },
                content: m.content,
                tool_calls: None,
            })
            .collect();

        // Build request
        let request = OllamaChatRequest {
            model: self.model_name.clone(),
            messages: ollama_messages,
            tools,
            stream: false,
        };

        debug!("OllamaLLM: Sending request to {}/api/chat", self.base_url);

        // Send request
        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await.unwrap_or_default();
            warn!("OllamaLLM: Request failed with status {}: {}", status, error_text);
            anyhow::bail!("Ollama request failed: {} - {}", status, error_text);
        }

        let ollama_response: OllamaChatResponse = response.json().await?;

        debug!("OllamaLLM: Received response, done={}", ollama_response.done);

        // Convert tool calls
        let tool_calls = if let Some(ollama_tool_calls) = ollama_response.message.tool_calls {
            debug!("OllamaLLM: Detected {} tool calls", ollama_tool_calls.len());
            ollama_tool_calls
                .into_iter()
                .map(|tc| ToolCall {
                    name: tc.function.name,
                    arguments: tc.function.arguments,
                })
                .collect()
        } else {
            Vec::new()
        };

        let finish_reason = if !tool_calls.is_empty() {
            "tool_calls"
        } else {
            "completed"
        };

        Ok(LlmResponse {
            text: ollama_response.message.content,
            tool_calls,
            finish_reason: finish_reason.to_string(),
        })
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Only run when Ollama is available
    async fn test_ollama_simple_generation() {
        let llm = OllamaLlm::new("mistral:latest".to_string());

        let messages = vec![Message {
            role: MessageRole::User,
            content: "Dis bonjour en une phrase courte.".to_string(),
        }];

        let response = llm.generate(messages, None).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        assert!(!response.text.is_empty());
        println!("Response: {}", response.text);
    }

    #[tokio::test]
    #[ignore] // Only run when Ollama is available
    async fn test_ollama_with_tools() {
        let llm = OllamaLlm::new("mistral:latest".to_string());

        let messages = vec![Message {
            role: MessageRole::User,
            content: "Envoie un email à john@example.com pour dire bonjour".to_string(),
        }];

        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "send_email",
                "description": "Send an email via SMTP",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "to": {
                            "type": "string",
                            "description": "Recipient email address"
                        },
                        "subject": {
                            "type": "string",
                            "description": "Email subject"
                        },
                        "body": {
                            "type": "string",
                            "description": "Email body"
                        }
                    },
                    "required": ["to", "subject", "body"]
                }
            }
        })];

        let response = llm.generate(messages, Some(tools)).await;
        assert!(response.is_ok());

        let response = response.unwrap();
        println!("Response: {:?}", response);

        if !response.tool_calls.is_empty() {
            println!("Tool calls detected: {:?}", response.tool_calls);
        }
    }
}
