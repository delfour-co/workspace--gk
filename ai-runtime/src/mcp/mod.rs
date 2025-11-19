//! MCP (Model Context Protocol) implementation
//!
//! This module implements the MCP protocol for communicating with tool servers.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod registry;
pub mod server;
pub mod tool;

pub use registry::McpRegistry;
pub use server::McpServer;
pub use tool::{Tool, ToolParameter};

/// MCP JSON-RPC request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

impl McpRequest {
    /// Create a new MCP request
    pub fn new(method: String, params: serde_json::Value, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }

    /// Create a tool call request
    pub fn tool_call(tool_name: String, arguments: HashMap<String, serde_json::Value>, id: u64) -> Self {
        Self::new(
            "tools/call".to_string(),
            serde_json::json!({
                "name": tool_name,
                "arguments": arguments
            }),
            id,
        )
    }
}

/// MCP JSON-RPC response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<McpError>,
    pub id: u64,
}

impl McpResponse {
    /// Create a successful response
    pub fn success(result: serde_json::Value, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: McpError, id: u64) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// MCP error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn parse_error(message: String) -> Self {
        Self {
            code: -32700,
            message,
            data: None,
        }
    }

    pub fn invalid_request(message: String) -> Self {
        Self {
            code: -32600,
            message,
            data: None,
        }
    }

    pub fn method_not_found(message: String) -> Self {
        Self {
            code: -32601,
            message,
            data: None,
        }
    }

    pub fn invalid_params(message: String) -> Self {
        Self {
            code: -32602,
            message,
            data: None,
        }
    }

    pub fn internal_error(message: String) -> Self {
        Self {
            code: -32603,
            message,
            data: None,
        }
    }
}
