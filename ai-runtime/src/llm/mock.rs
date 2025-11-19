//! Mock LLM for testing
//!
//! This mock LLM uses simple pattern matching to detect intents
//! and generate appropriate tool calls.

use super::{LlmEngine, LlmResponse, Message, ToolCall};
use anyhow::Result;
use std::collections::HashMap;
use tracing::debug;

/// Mock LLM implementation for testing
pub struct MockLlm {
    model_name: String,
}

impl MockLlm {
    pub fn new() -> Self {
        Self {
            model_name: "mock-llm-v1".to_string(),
        }
    }

    /// Parse user message and extract intent
    fn parse_intent(&self, message: &str) -> Option<ToolCall> {
        let message_lower = message.to_lowercase();

        // Intent: Send email
        if message_lower.contains("envoie") && message_lower.contains("email") {
            return self.parse_send_email(message);
        }

        // Intent: List emails
        if (message_lower.contains("liste") || message_lower.contains("montre"))
            && (message_lower.contains("email") || message_lower.contains("mail"))
        {
            return Some(ToolCall {
                name: "list_emails".to_string(),
                arguments: HashMap::from([
                    ("folder".to_string(), serde_json::json!("INBOX")),
                    ("limit".to_string(), serde_json::json!(10)),
                ]),
            });
        }

        // Intent: Search emails
        if message_lower.contains("cherche") || message_lower.contains("recherche") {
            let query = self.extract_search_query(message);
            return Some(ToolCall {
                name: "search_emails".to_string(),
                arguments: HashMap::from([("query".to_string(), serde_json::json!(query))]),
            });
        }

        None
    }

    /// Parse "send email" intent
    fn parse_send_email(&self, message: &str) -> Option<ToolCall> {
        // Try to extract email address
        let email_regex = regex::Regex::new(r"([a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,})").ok()?;
        let email = email_regex.find(message)?.as_str();

        // Extract subject/body from "pour dire X" or "sujet X"
        let body = if message.contains("pour dire") {
            let parts: Vec<&str> = message.split("pour dire").collect();
            parts.get(1).map(|s| s.trim()).unwrap_or("Hello")
        } else if message.contains("pour lui dire") {
            let parts: Vec<&str> = message.split("pour lui dire").collect();
            parts.get(1).map(|s| s.trim()).unwrap_or("Hello")
        } else {
            "Hello from ai-runtime!"
        };

        // Extract subject
        let subject = if message.contains("sujet") {
            let parts: Vec<&str> = message.split("sujet").collect();
            parts.get(1).map(|s| s.trim()).unwrap_or("Message from AI")
        } else {
            "Message from AI"
        };

        Some(ToolCall {
            name: "send_email".to_string(),
            arguments: HashMap::from([
                ("to".to_string(), serde_json::json!(email)),
                ("subject".to_string(), serde_json::json!(subject)),
                ("body".to_string(), serde_json::json!(body)),
            ]),
        })
    }

    /// Extract search query from message
    fn extract_search_query(&self, message: &str) -> String {
        // Simple extraction: words after "cherche" or "recherche"
        if let Some(pos) = message.find("cherche") {
            message[pos + 7..].trim().to_string()
        } else if let Some(pos) = message.find("recherche") {
            message[pos + 9..].trim().to_string()
        } else {
            message.to_string()
        }
    }
}

impl Default for MockLlm {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl LlmEngine for MockLlm {
    async fn generate(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
    ) -> Result<LlmResponse> {
        debug!("MockLLM: Processing {} messages", messages.len());

        // Get last user message
        let user_message = messages
            .iter()
            .rev()
            .find(|m| matches!(m.role, super::MessageRole::User))
            .map(|m| m.content.as_str())
            .unwrap_or("");

        debug!("MockLLM: User message: {}", user_message);

        // Parse intent and generate tool call
        if let Some(tool_call) = self.parse_intent(user_message) {
            debug!("MockLLM: Detected tool call: {}", tool_call.name);

            Ok(LlmResponse {
                text: String::new(),
                tool_calls: vec![tool_call],
                finish_reason: "tool_calls".to_string(),
            })
        } else {
            // No intent detected, generate a default response
            Ok(LlmResponse {
                text: "Je n'ai pas compris votre demande. Essayez 'Envoie un email à john@example.com pour dire bonjour'.".to_string(),
                tool_calls: Vec::new(),
                finish_reason: "completed".to_string(),
            })
        }
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::MessageRole;

    #[tokio::test]
    async fn test_mock_llm_send_email() {
        let llm = MockLlm::new();

        let messages = vec![Message {
            role: MessageRole::User,
            content: "Envoie un email à john@example.com pour dire bonjour".to_string(),
        }];

        let response = llm.generate(messages, None).await.unwrap();

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "send_email");
        assert_eq!(
            response.tool_calls[0].arguments["to"],
            "john@example.com"
        );
    }

    #[tokio::test]
    async fn test_mock_llm_list_emails() {
        let llm = MockLlm::new();

        let messages = vec![Message {
            role: MessageRole::User,
            content: "Liste mes emails".to_string(),
        }];

        let response = llm.generate(messages, None).await.unwrap();

        assert_eq!(response.tool_calls.len(), 1);
        assert_eq!(response.tool_calls[0].name, "list_emails");
    }
}
