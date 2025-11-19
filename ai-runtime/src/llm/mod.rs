//! LLM Engine abstraction

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod mock;

/// LLM response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// Generated text
    pub text: String,
    /// Tool calls requested by the LLM
    pub tool_calls: Vec<ToolCall>,
    /// Finish reason (completed, length, tool_calls, etc.)
    pub finish_reason: String,
}

/// A tool call requested by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Tool name
    pub name: String,
    /// Arguments (key-value pairs)
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Message in conversation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
}

/// Message role
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// LLM Engine trait
#[async_trait::async_trait]
pub trait LlmEngine: Send + Sync {
    /// Generate a response from messages
    async fn generate(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<serde_json::Value>>,
    ) -> Result<LlmResponse>;

    /// Get model name
    fn model_name(&self) -> &str;
}
