//! Spam management API endpoints
//!
//! REST API for spam configuration, rules, and Bayesian learning.

use axum::{
    extract::State,
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::spam::{SpamAction, SpamConfig, SpamManager, SpamResult, SpamRule, SpamRuleType, SpamStats};

/// Spam API state
pub struct SpamState {
    pub spam_manager: Arc<SpamManager>,
}

/// API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
        }
    }

    pub fn error(msg: &str) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(msg.to_string()),
        }
    }
}

/// Spam config request
#[derive(Debug, Deserialize)]
pub struct SpamConfigRequest {
    pub spam_threshold: f64,
    pub ham_threshold: f64,
    pub quarantine_enabled: bool,
    pub learning_enabled: bool,
    pub quarantine_folder: String,
}

/// Spam rule request
#[derive(Debug, Deserialize)]
pub struct SpamRuleRequest {
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub pattern: String,
    pub score: f64,
    pub is_enabled: bool,
}

/// Test message request
#[derive(Debug, Deserialize)]
pub struct TestMessageRequest {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub body: String,
}

/// Learn request
#[derive(Debug, Deserialize)]
pub struct LearnRequest {
    pub body: String,
}

/// Spam result response
#[derive(Debug, Serialize)]
pub struct SpamResultResponse {
    pub score: f64,
    pub is_spam: bool,
    pub action: String,
    pub rules_matched: Vec<RuleMatchResponse>,
}

#[derive(Debug, Serialize)]
pub struct RuleMatchResponse {
    pub rule_name: String,
    pub score: f64,
    pub description: String,
}

impl From<SpamResult> for SpamResultResponse {
    fn from(result: SpamResult) -> Self {
        Self {
            score: result.score,
            is_spam: result.is_spam,
            action: match result.action {
                SpamAction::Deliver => "Deliver".to_string(),
                SpamAction::AddHeaders => "AddHeaders".to_string(),
                SpamAction::Quarantine => "Quarantine".to_string(),
                SpamAction::Reject => "Reject".to_string(),
            },
            rules_matched: result.rules_matched.into_iter().map(|m| RuleMatchResponse {
                rule_name: m.rule_name,
                score: m.score,
                description: m.description,
            }).collect(),
        }
    }
}

/// Spam rule response
#[derive(Debug, Serialize)]
pub struct SpamRuleResponse {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule_type: String,
    pub pattern: String,
    pub score: f64,
    pub is_enabled: bool,
}

impl From<SpamRule> for SpamRuleResponse {
    fn from(rule: SpamRule) -> Self {
        Self {
            id: rule.id,
            name: rule.name,
            description: rule.description,
            rule_type: match rule.rule_type {
                SpamRuleType::Header => "Header".to_string(),
                SpamRuleType::Body => "Body".to_string(),
                SpamRuleType::Dns => "Dns".to_string(),
                SpamRuleType::Bayesian => "Bayesian".to_string(),
                SpamRuleType::Regex => "Regex".to_string(),
            },
            pattern: rule.pattern,
            score: rule.score,
            is_enabled: rule.is_enabled,
        }
    }
}

/// Spam config response
#[derive(Debug, Serialize)]
pub struct SpamConfigResponse {
    pub spam_threshold: f64,
    pub ham_threshold: f64,
    pub quarantine_enabled: bool,
    pub learning_enabled: bool,
    pub quarantine_folder: String,
}

impl From<SpamConfig> for SpamConfigResponse {
    fn from(config: SpamConfig) -> Self {
        Self {
            spam_threshold: config.spam_threshold,
            ham_threshold: config.ham_threshold,
            quarantine_enabled: config.quarantine_enabled,
            learning_enabled: config.learning_enabled,
            quarantine_folder: config.quarantine_folder,
        }
    }
}

/// Spam log response
#[derive(Debug, Serialize)]
pub struct SpamLogResponse {
    pub id: String,
    pub message_id: String,
    pub recipient_email: String,
    pub total_score: f64,
    pub rules_matched: String,
    pub action_taken: String,
    pub created_at: String,
}

// === API Handlers ===

/// Get spam configuration
pub async fn get_config(
    State(state): State<Arc<SpamState>>,
) -> Result<Json<ApiResponse<SpamConfigResponse>>, StatusCode> {
    match state.spam_manager.get_config(None).await {
        Ok(config) => Ok(Json(ApiResponse::success(config.into()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get config: {}", e)))),
    }
}

/// Update spam configuration
pub async fn update_config(
    State(state): State<Arc<SpamState>>,
    Json(req): Json<SpamConfigRequest>,
) -> Result<Json<ApiResponse<SpamConfigResponse>>, StatusCode> {
    let config = SpamConfig {
        spam_threshold: req.spam_threshold,
        ham_threshold: req.ham_threshold,
        quarantine_enabled: req.quarantine_enabled,
        learning_enabled: req.learning_enabled,
        quarantine_folder: req.quarantine_folder,
    };

    match state.spam_manager.update_config(None, &config).await {
        Ok(()) => Ok(Json(ApiResponse::success(config.into()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to update config: {}", e)))),
    }
}

/// Get spam statistics
pub async fn get_stats(
    State(state): State<Arc<SpamState>>,
) -> Result<Json<ApiResponse<SpamStats>>, StatusCode> {
    match state.spam_manager.get_stats().await {
        Ok(stats) => Ok(Json(ApiResponse::success(stats))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get stats: {}", e)))),
    }
}

/// List all spam rules
pub async fn list_rules(
    State(state): State<Arc<SpamState>>,
) -> Result<Json<ApiResponse<Vec<SpamRuleResponse>>>, StatusCode> {
    match state.spam_manager.list_rules().await {
        Ok(rules) => {
            let response: Vec<SpamRuleResponse> = rules.into_iter().map(|r| r.into()).collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to list rules: {}", e)))),
    }
}

/// Create a spam rule
pub async fn create_rule(
    State(state): State<Arc<SpamState>>,
    Json(req): Json<SpamRuleRequest>,
) -> Result<Json<ApiResponse<SpamRuleResponse>>, StatusCode> {
    let rule = SpamRule {
        id: String::new(),
        name: req.name,
        description: req.description,
        rule_type: match req.rule_type.as_str() {
            "Header" => SpamRuleType::Header,
            "Body" => SpamRuleType::Body,
            "Dns" => SpamRuleType::Dns,
            "Bayesian" => SpamRuleType::Bayesian,
            _ => SpamRuleType::Regex,
        },
        pattern: req.pattern,
        score: req.score,
        is_enabled: req.is_enabled,
    };

    match state.spam_manager.create_rule(&rule).await {
        Ok(created) => Ok(Json(ApiResponse::success(created.into()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to create rule: {}", e)))),
    }
}

/// Update a spam rule
pub async fn update_rule(
    State(state): State<Arc<SpamState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(req): Json<SpamRuleRequest>,
) -> Result<Json<ApiResponse<SpamRuleResponse>>, StatusCode> {
    let rule = SpamRule {
        id: id.clone(),
        name: req.name,
        description: req.description,
        rule_type: match req.rule_type.as_str() {
            "Header" => SpamRuleType::Header,
            "Body" => SpamRuleType::Body,
            "Dns" => SpamRuleType::Dns,
            "Bayesian" => SpamRuleType::Bayesian,
            _ => SpamRuleType::Regex,
        },
        pattern: req.pattern,
        score: req.score,
        is_enabled: req.is_enabled,
    };

    match state.spam_manager.update_rule(&id, &rule).await {
        Ok(updated) => Ok(Json(ApiResponse::success(updated.into()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to update rule: {}", e)))),
    }
}

/// Delete a spam rule
pub async fn delete_rule(
    State(state): State<Arc<SpamState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Result<Json<ApiResponse<()>>, StatusCode> {
    match state.spam_manager.delete_rule(&id).await {
        Ok(()) => Ok(Json(ApiResponse::success(()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to delete rule: {}", e)))),
    }
}

/// Test a message against spam rules
pub async fn test_message(
    State(state): State<Arc<SpamState>>,
    Json(req): Json<TestMessageRequest>,
) -> Result<Json<ApiResponse<SpamResultResponse>>, StatusCode> {
    let result = state.spam_manager.test_message(&req.from, &req.to, &req.subject, &req.body).await;
    Ok(Json(ApiResponse::success(result.into())))
}

/// Learn from spam message
pub async fn learn_spam(
    State(state): State<Arc<SpamState>>,
    Json(req): Json<LearnRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.spam_manager.learn_spam(&req.body).await {
        Ok(()) => Ok(Json(ApiResponse::success("Learned as spam".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to learn spam: {}", e)))),
    }
}

/// Learn from ham (non-spam) message
pub async fn learn_ham(
    State(state): State<Arc<SpamState>>,
    Json(req): Json<LearnRequest>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.spam_manager.learn_ham(&req.body).await {
        Ok(()) => Ok(Json(ApiResponse::success("Learned as ham".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to learn ham: {}", e)))),
    }
}

/// Get spam logs
pub async fn get_logs(
    State(state): State<Arc<SpamState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<ApiResponse<Vec<SpamLogResponse>>>, StatusCode> {
    let limit = params.get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);

    match state.spam_manager.get_logs(limit).await {
        Ok(logs) => {
            let response: Vec<SpamLogResponse> = logs.into_iter().map(|log| SpamLogResponse {
                id: log.id,
                message_id: log.message_id,
                recipient_email: log.recipient_email,
                total_score: log.total_score,
                rules_matched: log.rules_matched,
                action_taken: match log.action_taken {
                    SpamAction::Deliver => "Deliver".to_string(),
                    SpamAction::AddHeaders => "AddHeaders".to_string(),
                    SpamAction::Quarantine => "Quarantine".to_string(),
                    SpamAction::Reject => "Reject".to_string(),
                },
                created_at: log.created_at.to_rfc3339(),
            }).collect();
            Ok(Json(ApiResponse::success(response)))
        }
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to get logs: {}", e)))),
    }
}

/// Clear spam logs
pub async fn clear_logs(
    State(state): State<Arc<SpamState>>,
) -> Result<Json<ApiResponse<String>>, StatusCode> {
    match state.spam_manager.clear_logs().await {
        Ok(()) => Ok(Json(ApiResponse::success("Logs cleared".to_string()))),
        Err(e) => Ok(Json(ApiResponse::error(&format!("Failed to clear logs: {}", e)))),
    }
}
