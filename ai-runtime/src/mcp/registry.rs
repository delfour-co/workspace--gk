//! MCP Server registry

use super::{McpServer, Tool};
use anyhow::Result;
use std::collections::HashMap;
use tracing::{debug, info};

/// Registry of MCP servers and their tools
pub struct McpRegistry {
    /// Registered servers
    servers: HashMap<String, McpServer>,
    /// All available tools (mapped by name)
    tools: HashMap<String, Tool>,
}

impl McpRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            servers: HashMap::new(),
            tools: HashMap::new(),
        }
    }

    /// Register a new MCP server
    pub async fn register_server(&mut self, server: McpServer) -> Result<()> {
        info!("Registering MCP server: {}", server.name);

        // Discover tools from the server
        let tools = server.discover_tools().await?;

        debug!(
            "Server {} provides {} tools",
            server.name,
            tools.len()
        );

        // Store tools
        for tool in tools {
            self.tools.insert(tool.name.clone(), tool);
        }

        // Store server
        self.servers.insert(server.name.clone(), server);

        Ok(())
    }

    /// Get all available tools
    pub fn get_tools(&self) -> Vec<&Tool> {
        self.tools.values().collect()
    }

    /// Get a specific tool by name
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.get(name)
    }

    /// Call a tool
    pub async fn call_tool(
        &self,
        tool_name: &str,
        arguments: HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value> {
        // Find the tool
        let tool = self
            .get_tool(tool_name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", tool_name))?;

        // Get the server
        let server = self
            .servers
            .get(&tool.server)
            .ok_or_else(|| anyhow::anyhow!("Server not found: {}", tool.server))?;

        // Call the tool
        server.call_tool(tool_name, arguments).await
    }

    /// Get tool schemas for LLM (function calling)
    pub fn get_tool_schemas(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|tool| tool.to_schema())
            .collect()
    }
}

impl Default for McpRegistry {
    fn default() -> Self {
        Self::new()
    }
}
