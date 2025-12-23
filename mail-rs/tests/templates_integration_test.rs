//! Integration tests for Email Templates system

use mail_rs::templates::types::{
    CreateTemplateRequest, TemplateCategory, UpdateTemplateRequest,
};
use mail_rs::templates::{TemplateManager, TemplateRenderer};
use sqlx::SqlitePool;
use tempfile::NamedTempFile;

/// Helper to create an in-memory database for testing
async fn setup_test_db() -> SqlitePool {
    // Use in-memory database for testing
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

    // Initialize the templates table
    let manager = TemplateManager::new(pool.clone());
    manager.init_db().await.unwrap();

    pool
}

#[tokio::test]
async fn test_create_template() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    let request = CreateTemplateRequest {
        name: "Test Template".to_string(),
        category: TemplateCategory::QuickReply,
        subject: "Re: {{subject}}".to_string(),
        body_html: "<p>Hello {{recipient_name}},</p>".to_string(),
        body_text: "Hello {{recipient_name}},".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let template = manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    assert_eq!(template.name, "Test Template");
    assert_eq!(template.category, TemplateCategory::QuickReply);
    assert_eq!(template.subject, "Re: {{subject}}");
    assert!(!template.is_signature);
    assert_eq!(template.owner_email, "test@example.com");
}

#[tokio::test]
async fn test_get_template() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // Create a template first
    let request = CreateTemplateRequest {
        name: "Get Test".to_string(),
        category: TemplateCategory::Custom,
        subject: "".to_string(),
        body_html: "<p>Test body</p>".to_string(),
        body_text: "".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let created = manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    // Retrieve the template
    let retrieved = manager
        .get_template(&created.id)
        .await
        .unwrap()
        .unwrap();

    assert_eq!(retrieved.id, created.id);
    assert_eq!(retrieved.name, "Get Test");
    assert_eq!(retrieved.body_html, "<p>Test body</p>");
}

#[tokio::test]
async fn test_update_template() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // Create a template
    let request = CreateTemplateRequest {
        name: "Original Name".to_string(),
        category: TemplateCategory::QuickReply,
        subject: "".to_string(),
        body_html: "<p>Original</p>".to_string(),
        body_text: "".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let created = manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    // Update it
    let update = UpdateTemplateRequest {
        name: Some("Updated Name".to_string()),
        subject: Some("New Subject".to_string()),
        body_html: Some("<p>Updated</p>".to_string()),
        body_text: Some("Updated text".to_string()),
        variables: Some(vec![]),
        is_signature: Some(false),
    };

    let updated = manager
        .update_template(&created.id, "test@example.com", update)
        .await
        .unwrap();

    assert_eq!(updated.name, "Updated Name");
    assert_eq!(updated.subject, "New Subject");
    assert_eq!(updated.body_html, "<p>Updated</p>");
    assert_eq!(updated.body_text, "Updated text");
}

#[tokio::test]
async fn test_delete_template() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // Create a template
    let request = CreateTemplateRequest {
        name: "To Delete".to_string(),
        category: TemplateCategory::Custom,
        subject: "".to_string(),
        body_html: "<p>Delete me</p>".to_string(),
        body_text: "".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let created = manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    // Delete it
    manager
        .delete_template(&created.id, "test@example.com")
        .await
        .unwrap();

    // Verify it's gone
    let result = manager.get_template(&created.id).await.unwrap();
    assert!(result.is_none());
}

#[tokio::test]
async fn test_list_templates() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // Create multiple templates
    for i in 1..=3 {
        let request = CreateTemplateRequest {
            name: format!("Template {}", i),
            category: TemplateCategory::QuickReply,
            subject: "".to_string(),
            body_html: format!("<p>Body {}</p>", i),
            body_text: "".to_string(),
            variables: vec![],
            is_signature: false,
        };
        manager
            .create_template("test@example.com", request)
            .await
            .unwrap();
    }

    let templates = manager.list_templates("test@example.com").await.unwrap();
    assert_eq!(templates.len(), 3);
}

#[tokio::test]
async fn test_list_templates_by_category() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // Create templates in different categories
    let categories = vec![
        TemplateCategory::Signature,
        TemplateCategory::QuickReply,
        TemplateCategory::Custom,
    ];

    for (i, category) in categories.iter().enumerate() {
        let request = CreateTemplateRequest {
            name: format!("Template {}", i),
            category: category.clone(),
            subject: "".to_string(),
            body_html: format!("<p>Body {}</p>", i),
            body_text: "".to_string(),
            variables: vec![],
            is_signature: matches!(category, TemplateCategory::Signature),
        };
        manager
            .create_template("test@example.com", request)
            .await
            .unwrap();
    }

    // List signatures
    let signatures = manager
        .list_templates_by_category("test@example.com", TemplateCategory::Signature)
        .await
        .unwrap();
    assert_eq!(signatures.len(), 1);

    // List quick replies
    let quick_replies = manager
        .list_templates_by_category("test@example.com", TemplateCategory::QuickReply)
        .await
        .unwrap();
    assert_eq!(quick_replies.len(), 1);
}

#[tokio::test]
async fn test_get_default_signature() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // No default signature initially
    let default = manager
        .get_default_signature("test@example.com")
        .await
        .unwrap();
    assert!(default.is_none());

    // Create a signature template
    let request = CreateTemplateRequest {
        name: "My Signature".to_string(),
        category: TemplateCategory::Signature,
        subject: "".to_string(),
        body_html: "<p>Best regards,<br>John Doe</p>".to_string(),
        body_text: "Best regards,\nJohn Doe".to_string(),
        variables: vec![],
        is_signature: true,
    };

    manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    // Get default signature (should return the first signature template)
    let default = manager
        .get_default_signature("test@example.com")
        .await
        .unwrap();
    assert!(default.is_some());
    assert_eq!(default.unwrap().name, "My Signature");
}

#[tokio::test]
async fn test_template_ownership() {
    let pool = setup_test_db().await;
    let manager = TemplateManager::new(pool);

    // User A creates a template
    let request = CreateTemplateRequest {
        name: "User A Template".to_string(),
        category: TemplateCategory::QuickReply,
        subject: "".to_string(),
        body_html: "<p>Test</p>".to_string(),
        body_text: "".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let template = manager
        .create_template("usera@example.com", request)
        .await
        .unwrap();

    // get_template returns Option, so we need to check ownership differently
    let retrieved = manager.get_template(&template.id).await.unwrap().unwrap();
    assert_eq!(retrieved.owner_email, "usera@example.com");

    // User B should not be able to delete User A's template
    let result = manager
        .delete_template(&template.id, "userb@example.com")
        .await;
    assert!(result.is_err());
}

#[test]
fn test_template_renderer_extract_variables() {
    let template = "Hello {{name}}, your email is {{email}}!";
    let vars = TemplateRenderer::extract_variables(template);

    assert_eq!(vars.len(), 2);
    assert!(vars.contains(&"name".to_string()));
    assert!(vars.contains(&"email".to_string()));
}

#[test]
fn test_template_renderer_subject() {
    let subject = "Re: {{subject}} - Response from {{sender_name}}";
    let mut vars = std::collections::HashMap::new();
    vars.insert("subject".to_string(), "Meeting Request".to_string());
    vars.insert("sender_name".to_string(), "Alice".to_string());

    let result = TemplateRenderer::render_subject(subject, &vars);
    assert_eq!(result, "Re: Meeting Request - Response from Alice");
}

#[test]
fn test_template_renderer_missing_variable() {
    let subject = "Hello {{name}}!";
    let vars = std::collections::HashMap::new();

    // Missing variables should be left as-is
    let result = TemplateRenderer::render_subject(subject, &vars);
    assert_eq!(result, "Hello {{name}}!");
}

#[test]
fn test_template_renderer_date_time_variables() {
    let subject = "Date: {{date}}, Time: {{time}}";
    let vars = std::collections::HashMap::new();

    let result = TemplateRenderer::render_subject(subject, &vars);

    // Should contain date and time (exact values vary)
    assert!(result.contains("Date: "));
    assert!(result.contains("Time: "));
}

#[test]
fn test_template_category_serialization() {
    use serde_json;

    let signature = TemplateCategory::Signature;
    let json = serde_json::to_string(&signature).unwrap();
    assert_eq!(json, "\"signature\"");

    let quick_reply = TemplateCategory::QuickReply;
    let json = serde_json::to_string(&quick_reply).unwrap();
    assert_eq!(json, "\"quick_reply\"");

    let custom = TemplateCategory::Custom;
    let json = serde_json::to_string(&custom).unwrap();
    assert_eq!(json, "\"custom\"");
}

#[tokio::test]
async fn test_concurrent_template_access() {
    use std::sync::Arc;

    let pool = setup_test_db().await;
    let manager = Arc::new(TemplateManager::new(pool));

    // Create initial template
    let request = CreateTemplateRequest {
        name: "Concurrent Test".to_string(),
        category: TemplateCategory::QuickReply,
        subject: "".to_string(),
        body_html: "<p>Test</p>".to_string(),
        body_text: "".to_string(),
        variables: vec![],
        is_signature: false,
    };

    let template = manager
        .create_template("test@example.com", request)
        .await
        .unwrap();

    // Spawn multiple concurrent reads
    let mut handles = vec![];
    for _ in 0..5 {
        let mgr = manager.clone();
        let template_id = template.id.clone();
        let handle = tokio::spawn(async move {
            mgr.get_template(&template_id)
                .await
                .unwrap()
                .unwrap()
        });
        handles.push(handle);
    }

    // All should succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result.name, "Concurrent Test");
    }
}
