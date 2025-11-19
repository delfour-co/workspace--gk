//! MCP Server client

use super::{McpRequest, McpResponse, Tool};
use anyhow::Result;
use tracing::{debug, warn};

/// Client for communicating with an MCP server
#[derive(Debug, Clone)]
pub struct McpServer {
    /// Server name
    pub name: String,
    /// Server base URL
    pub url: String,
    /// HTTP client
    client: reqwest::Client,
}

impl McpServer {
    /// Create a new MCP server client
    pub fn new(name: String, url: String) -> Self {
        Self {
            name,
            url,
            client: reqwest::Client::new(),
        }
    }

    /// Discover available tools from this server
    pub async fn discover_tools(&self) -> Result<Vec<Tool>> {
        debug!("Discovering tools from server: {}", self.name);

        let request = McpRequest::new("tools/list".to_string(), serde_json::json!({}), 1);

        let response = self
            .client
            .post(&format!("{}/mcp", self.url))
            .json(&request)
            .send()
            .await?;

        let mcp_response: McpResponse = response.json().await?;

        if let Some(result) = mcp_response.result {
            let tools: Vec<Tool> = serde_json::from_value(result)?;
            debug!("Discovered {} tools from {}", tools.len(), self.name);
            Ok(tools)
        } else {
            warn!("No tools discovered from {}", self.name);
            Ok(Vec::new())
        }
    }

    /// Call a tool on this server
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        debug!(
            "Calling tool {} on server {} with args: {:?}",
            tool_name, self.name, arguments
        );

        let request = McpRequest::tool_call(tool_name.to_string(), arguments, 1);

        let response = self
            .client
            .post(&format!("{}/mcp", self.url))
            .json(&request)
            .send()
            .await?;

        let mcp_response: McpResponse = response.json().await?;

        if let Some(error) = mcp_response.error {
            anyhow::bail!("MCP error: {}", error.message);
        }

        Ok(mcp_response.result.unwrap_or(serde_json::json!({})))
    }
}
