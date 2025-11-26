//! MCP Tool definitions

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A tool that can be called by the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    /// Tool name (e.g., "send_email")
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Input parameters
    pub parameters: Vec<ToolParameter>,
    /// Which MCP server provides this tool
    pub server: String,
}

impl Tool {
    pub fn new(name: String, description: String, server: String) -> Self {
        Self {
            name,
            description,
            parameters: Vec::new(),
            server,
        }
    }

    pub fn with_parameter(mut self, param: ToolParameter) -> Self {
        self.parameters.push(param);
        self
    }

    /// Convert to JSON schema for LLM (Ollama function calling format)
    pub fn to_schema(&self) -> serde_json::Value {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        for param in &self.parameters {
            properties.insert(
                param.name.clone(),
                serde_json::json!({
                    "type": param.param_type,
                    "description": param.description,
                }),
            );
            if param.required {
                required.push(param.name.clone());
            }
        }

        // Ollama expects this format with "type": "function"
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": {
                    "type": "object",
                    "properties": properties,
                    "required": required,
                }
            }
        })
    }
}

/// Tool parameter definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolParameter {
    pub name: String,
    pub description: String,
    #[serde(rename = "type")]
    pub param_type: String, // "string", "number", "boolean", etc.
    pub required: bool,
}

impl ToolParameter {
    pub fn new(name: String, description: String, param_type: String, required: bool) -> Self {
        Self {
            name,
            description,
            param_type,
            required,
        }
    }

    pub fn string(name: String, description: String, required: bool) -> Self {
        Self::new(name, description, "string".to_string(), required)
    }

    pub fn number(name: String, description: String, required: bool) -> Self {
        Self::new(name, description, "number".to_string(), required)
    }

    pub fn boolean(name: String, description: String, required: bool) -> Self {
        Self::new(name, description, "boolean".to_string(), required)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_schema() {
        let tool = Tool::new(
            "send_email".to_string(),
            "Send an email".to_string(),
            "mail".to_string(),
        )
        .with_parameter(ToolParameter::string(
            "to".to_string(),
            "Recipient email".to_string(),
            true,
        ))
        .with_parameter(ToolParameter::string(
            "subject".to_string(),
            "Email subject".to_string(),
            true,
        ));

        let schema = tool.to_schema();
        // Check Ollama function calling format
        assert_eq!(schema["type"], "function");
        assert_eq!(schema["function"]["name"], "send_email");
        assert_eq!(schema["function"]["description"], "Send an email");
        assert!(schema["function"]["parameters"]["required"].as_array().unwrap().len() == 2);
    }
}
