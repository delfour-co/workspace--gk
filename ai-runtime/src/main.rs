//! ai-runtime - AI Orchestration Engine
//!
//! Core component that orchestrates LLM + MCP servers

mod llm;
mod mcp;

use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use llm::{LlmEngine, Message, MessageRole};
use llm::mock::MockLlm;
use llm::ollama::OllamaLlm;
use mcp::{McpRegistry, McpServer};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn, Level};
use tracing_subscriber::FmtSubscriber;

/// Application state
struct AppState {
    llm: Arc<dyn LlmEngine>,
    mcp_registry: Arc<Mutex<McpRegistry>>,
}

/// Chat request
#[derive(Debug, Deserialize)]
struct ChatRequest {
    message: String,
}

/// Chat response
#[derive(Debug, Serialize)]
struct ChatResponse {
    response: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_result: Option<serde_json::Value>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::DEBUG)
        .pretty()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("🚀 Starting ai-runtime...");

    // Initialize LLM (Ollama or Mock based on environment)
    let use_ollama = std::env::var("USE_OLLAMA")
        .unwrap_or_else(|_| "true".to_string())
        .parse::<bool>()
        .unwrap_or(true);

    let llm: Arc<dyn LlmEngine> = if use_ollama {
        let model = std::env::var("OLLAMA_MODEL").unwrap_or_else(|_| "mistral:latest".to_string());
        info!("🤖 Using Ollama with model: {}", model);
        Arc::new(OllamaLlm::new(model))
    } else {
        info!("🤖 Using Mock LLM");
        Arc::new(MockLlm::new())
    };

    info!("✅ LLM initialized: {}", llm.model_name());

    // Initialize MCP registry
    let mut mcp_registry = McpRegistry::new();

    // Register MCP servers
    info!("📡 Registering MCP servers...");

    // Register mail server
    let mail_server = McpServer::new(
        "mail".to_string(),
        "http://localhost:8090".to_string(),
    );

    match mcp_registry.register_server(mail_server).await {
        Ok(_) => info!("✅ Mail server registered"),
        Err(e) => warn!("⚠️  Could not register mail server: {}", e),
    }

    info!("✅ MCP registry initialized with {} tools", mcp_registry.get_tools().len());

    // Create app state
    let state = Arc::new(AppState {
        llm,
        mcp_registry: Arc::new(Mutex::new(mcp_registry)),
    });

    // Build router
    let app = Router::new()
        .route("/", get(health_check))
        .route("/health", get(health_check))
        .route("/chat", post(chat_handler))
        .with_state(state);

    // Start server
    let addr = "0.0.0.0:8888";
    info!("🌐 Server listening on http://{}", addr);
    info!("💬 Try: curl -X POST http://localhost:8888/chat -H 'Content-Type: application/json' -d '{{\"message\": \"Envoie un email à john@example.com pour dire bonjour\"}}'");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Health check endpoint
async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "ai-runtime",
        "version": "0.1.0"
    }))
}

/// Chat endpoint
async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, (StatusCode, String)> {
    info!("📥 Chat request: {}", payload.message);

    // Create message
    let messages = vec![Message {
        role: MessageRole::User,
        content: payload.message,
    }];

    // Get tools from MCP registry
    let registry = state.mcp_registry.lock().await;
    let tool_schemas = registry.get_tool_schemas();
    drop(registry); // Release lock

    // Generate LLM response
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
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    // If LLM requested tool calls, execute them
    let mut tool_result = None;
    if !llm_response.tool_calls.is_empty() {
        info!("🔧 Executing tool calls: {:?}", llm_response.tool_calls);

        let registry = state.mcp_registry.lock().await;

        for tool_call in &llm_response.tool_calls {
            let result = registry
                .call_tool(&tool_call.name, tool_call.arguments.clone())
                .await
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            tool_result = Some(result);
            info!("✅ Tool executed: {}", tool_call.name);
        }
    }

    // Prepare response
    let response_text = if llm_response.text.is_empty() {
        if let Some(ref result) = tool_result {
            format!("✅ Action completed: {:?}", result)
        } else {
            "Action completed.".to_string()
        }
    } else {
        llm_response.text
    };

    info!("📤 Chat response: {}", response_text);

    Ok(Json(ChatResponse {
        response: response_text,
        tool_result,
    }))
}
