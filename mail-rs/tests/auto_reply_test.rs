//! Integration tests for Auto-Reply system

use chrono::{Duration, Utc};
use mail_rs::auto_reply::{
    AutoReplyManager, CreateAutoReplyRequest, UpdateAutoReplyRequest,
};
use sqlx::SqlitePool;

async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
    let manager = AutoReplyManager::new(pool.clone());
    manager.init_db().await.unwrap();
    pool
}

#[tokio::test]
async fn test_create_auto_reply_config() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    let config = manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    assert_eq!(config.email, "test@example.com");
    assert!(config.is_active);
    assert_eq!(config.subject, "Out of Office");
    assert_eq!(config.reply_interval_hours, 24);
}

#[tokio::test]
async fn test_get_auto_reply_config() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // No config initially
    let config = manager.get_config("test@example.com").await.unwrap();
    assert!(config.is_none());

    // Create config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Get config
    let config = manager.get_config("test@example.com").await.unwrap();
    assert!(config.is_some());
    assert_eq!(config.unwrap().email, "test@example.com");
}

#[tokio::test]
async fn test_update_auto_reply_config() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create initial config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Update config
    let update = UpdateAutoReplyRequest {
        is_active: Some(false),
        start_date: None,
        end_date: None,
        subject: Some("New Subject".to_string()),
        body_html: None,
        body_text: None,
        reply_interval_hours: Some(48),
    };

    let updated = manager
        .update_config("test@example.com", update)
        .await
        .unwrap();

    assert!(!updated.is_active);
    assert_eq!(updated.subject, "New Subject");
    assert_eq!(updated.reply_interval_hours, 48);
}

#[tokio::test]
async fn test_delete_auto_reply_config() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Delete config
    manager.delete_config("test@example.com").await.unwrap();

    // Verify deleted
    let config = manager.get_config("test@example.com").await.unwrap();
    assert!(config.is_none());
}

#[tokio::test]
async fn test_should_send_auto_reply_first_time() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create active config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Should send (first time)
    let should_send = manager
        .should_send_auto_reply("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert!(should_send);
}

#[tokio::test]
async fn test_should_not_send_auto_reply_within_interval() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create active config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Record first auto-reply
    manager
        .record_auto_reply_sent("test@example.com", "sender@example.com")
        .await
        .unwrap();

    // Should NOT send immediately after
    let should_send = manager
        .should_send_auto_reply("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert!(!should_send);
}

#[tokio::test]
async fn test_should_not_send_when_inactive() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create inactive config
    let request = CreateAutoReplyRequest {
        is_active: false,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Should NOT send (inactive)
    let should_send = manager
        .should_send_auto_reply("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert!(!should_send);
}

#[tokio::test]
async fn test_should_not_send_before_start_date() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config with future start date
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: Some(Utc::now() + Duration::days(1)),
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Should NOT send (before start date)
    let should_send = manager
        .should_send_auto_reply("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert!(!should_send);
}

#[tokio::test]
async fn test_should_not_send_after_end_date() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config with past end date
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: Some(Utc::now() - Duration::days(10)),
        end_date: Some(Utc::now() - Duration::days(1)),
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Should NOT send (after end date)
    let should_send = manager
        .should_send_auto_reply("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert!(!should_send);
}

#[tokio::test]
async fn test_record_auto_reply_sent() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Record sent
    let record = manager
        .record_auto_reply_sent("test@example.com", "sender@example.com")
        .await
        .unwrap();

    assert_eq!(record.user_email, "test@example.com");
    assert_eq!(record.sent_to, "sender@example.com");
}

#[tokio::test]
async fn test_cleanup_old_records() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config
    let request = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "Out of Office".to_string(),
        body_html: "<p>I'm away</p>".to_string(),
        body_text: "I'm away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("test@example.com", request)
        .await
        .unwrap();

    // Record some sends
    for i in 0..5 {
        manager
            .record_auto_reply_sent("test@example.com", &format!("sender{}@example.com", i))
            .await
            .unwrap();
    }

    // Cleanup (shouldn't delete recent records)
    let deleted = manager.cleanup_old_records().await.unwrap();
    assert_eq!(deleted, 0);
}

#[tokio::test]
async fn test_multiple_users() {
    let pool = setup_test_db().await;
    let manager = AutoReplyManager::new(pool);

    // Create config for user A
    let request_a = CreateAutoReplyRequest {
        is_active: true,
        start_date: None,
        end_date: None,
        subject: "User A Out".to_string(),
        body_html: "<p>User A away</p>".to_string(),
        body_text: "User A away".to_string(),
        reply_interval_hours: Some(24),
    };

    manager
        .set_config("usera@example.com", request_a)
        .await
        .unwrap();

    // Create config for user B
    let request_b = CreateAutoReplyRequest {
        is_active: false,
        start_date: None,
        end_date: None,
        subject: "User B Out".to_string(),
        body_html: "<p>User B away</p>".to_string(),
        body_text: "User B away".to_string(),
        reply_interval_hours: Some(48),
    };

    manager
        .set_config("userb@example.com", request_b)
        .await
        .unwrap();

    // Get configs
    let config_a = manager.get_config("usera@example.com").await.unwrap().unwrap();
    let config_b = manager.get_config("userb@example.com").await.unwrap().unwrap();

    assert_eq!(config_a.subject, "User A Out");
    assert!(config_a.is_active);

    assert_eq!(config_b.subject, "User B Out");
    assert!(!config_b.is_active);
}
