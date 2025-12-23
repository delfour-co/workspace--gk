//! Auto-reply manager - handles auto-reply configuration and tracking

use crate::auto_reply::types::{
    AutoReplyConfig, AutoReplySent, CreateAutoReplyRequest, UpdateAutoReplyRequest,
};
use crate::error::MailError;
use chrono::{DateTime, Duration, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Manages auto-reply configurations and sent tracking
pub struct AutoReplyManager {
    db: SqlitePool,
}

impl AutoReplyManager {
    /// Create a new auto-reply manager
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Initialize database tables
    pub async fn init_db(&self) -> Result<(), MailError> {
        // Auto-reply configurations table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS auto_reply_configs (
                email TEXT PRIMARY KEY,
                is_active BOOLEAN NOT NULL DEFAULT 0,
                start_date TEXT,
                end_date TEXT,
                subject TEXT NOT NULL,
                body_html TEXT NOT NULL,
                body_text TEXT NOT NULL,
                reply_interval_hours INTEGER NOT NULL DEFAULT 24,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Tracking table for sent auto-replies
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS auto_reply_sent (
                id TEXT PRIMARY KEY,
                user_email TEXT NOT NULL,
                sent_to TEXT NOT NULL,
                sent_at TEXT NOT NULL,
                FOREIGN KEY (user_email) REFERENCES auto_reply_configs(email) ON DELETE CASCADE
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Index for efficient lookups
        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_auto_reply_sent_lookup
            ON auto_reply_sent(user_email, sent_to, sent_at)
            "#,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get auto-reply configuration for a user
    pub async fn get_config(&self, email: &str) -> Result<Option<AutoReplyConfig>, MailError> {
        let row = sqlx::query(
            r#"
            SELECT email, is_active, start_date, end_date, subject, body_html, body_text,
                   reply_interval_hours, created_at, updated_at
            FROM auto_reply_configs
            WHERE email = ?
            "#,
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_config(row)?))
        } else {
            Ok(None)
        }
    }

    /// Create or update auto-reply configuration
    pub async fn set_config(
        &self,
        email: &str,
        request: CreateAutoReplyRequest,
    ) -> Result<AutoReplyConfig, MailError> {
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO auto_reply_configs (
                email, is_active, start_date, end_date, subject, body_html, body_text,
                reply_interval_hours, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(email) DO UPDATE SET
                is_active = excluded.is_active,
                start_date = excluded.start_date,
                end_date = excluded.end_date,
                subject = excluded.subject,
                body_html = excluded.body_html,
                body_text = excluded.body_text,
                reply_interval_hours = excluded.reply_interval_hours,
                updated_at = excluded.updated_at
            "#,
        )
        .bind(email)
        .bind(request.is_active)
        .bind(request.start_date.map(|d| d.to_rfc3339()))
        .bind(request.end_date.map(|d| d.to_rfc3339()))
        .bind(&request.subject)
        .bind(&request.body_html)
        .bind(&request.body_text)
        .bind(request.reply_interval_hours.unwrap_or(24))
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        self.get_config(email)
            .await?
            .ok_or_else(|| MailError::NotFound("Failed to retrieve created config".to_string()))
    }

    /// Update auto-reply configuration
    pub async fn update_config(
        &self,
        email: &str,
        request: UpdateAutoReplyRequest,
    ) -> Result<AutoReplyConfig, MailError> {
        let existing = self
            .get_config(email)
            .await?
            .ok_or_else(|| MailError::NotFound("Auto-reply config not found".to_string()))?;

        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE auto_reply_configs
            SET is_active = ?, start_date = ?, end_date = ?, subject = ?,
                body_html = ?, body_text = ?, reply_interval_hours = ?, updated_at = ?
            WHERE email = ?
            "#,
        )
        .bind(request.is_active.unwrap_or(existing.is_active))
        .bind(
            request
                .start_date
                .or(existing.start_date)
                .map(|d| d.to_rfc3339()),
        )
        .bind(
            request
                .end_date
                .or(existing.end_date)
                .map(|d| d.to_rfc3339()),
        )
        .bind(request.subject.unwrap_or(existing.subject))
        .bind(request.body_html.unwrap_or(existing.body_html))
        .bind(request.body_text.unwrap_or(existing.body_text))
        .bind(
            request
                .reply_interval_hours
                .unwrap_or(existing.reply_interval_hours),
        )
        .bind(now.to_rfc3339())
        .bind(email)
        .execute(&self.db)
        .await?;

        self.get_config(email)
            .await?
            .ok_or_else(|| MailError::NotFound("Failed to retrieve updated config".to_string()))
    }

    /// Delete auto-reply configuration
    pub async fn delete_config(&self, email: &str) -> Result<(), MailError> {
        sqlx::query("DELETE FROM auto_reply_configs WHERE email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Check if auto-reply should be sent to a specific sender
    ///
    /// Returns true if:
    /// 1. Auto-reply is active and within date range
    /// 2. We haven't sent to this sender within the reply interval
    pub async fn should_send_auto_reply(
        &self,
        user_email: &str,
        sender_email: &str,
    ) -> Result<bool, MailError> {
        // Get config
        let config = match self.get_config(user_email).await? {
            Some(c) => c,
            None => return Ok(false),
        };

        // Check if active
        if !config.is_currently_active() {
            return Ok(false);
        }

        // Check if we've recently replied to this sender
        let cutoff = Utc::now() - Duration::hours(config.reply_interval_hours as i64);

        let recent_reply = sqlx::query(
            r#"
            SELECT id FROM auto_reply_sent
            WHERE user_email = ? AND sent_to = ? AND sent_at > ?
            LIMIT 1
            "#,
        )
        .bind(user_email)
        .bind(sender_email)
        .bind(cutoff.to_rfc3339())
        .fetch_optional(&self.db)
        .await?;

        Ok(recent_reply.is_none())
    }

    /// Record that an auto-reply was sent
    pub async fn record_auto_reply_sent(
        &self,
        user_email: &str,
        sent_to: &str,
    ) -> Result<AutoReplySent, MailError> {
        let id = Uuid::new_v4().to_string();
        let sent_at = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO auto_reply_sent (id, user_email, sent_to, sent_at)
            VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(user_email)
        .bind(sent_to)
        .bind(sent_at.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(AutoReplySent {
            id,
            user_email: user_email.to_string(),
            sent_to: sent_to.to_string(),
            sent_at,
        })
    }

    /// Clean up old auto-reply sent records (older than 30 days)
    pub async fn cleanup_old_records(&self) -> Result<u64, MailError> {
        let cutoff = Utc::now() - Duration::days(30);

        let result = sqlx::query("DELETE FROM auto_reply_sent WHERE sent_at < ?")
            .bind(cutoff.to_rfc3339())
            .execute(&self.db)
            .await?;

        Ok(result.rows_affected())
    }

    /// Helper: Convert database row to AutoReplyConfig
    fn row_to_config(&self, row: sqlx::sqlite::SqliteRow) -> Result<AutoReplyConfig, MailError> {
        use sqlx::Row;

        let start_date_str: Option<String> = row.try_get("start_date")?;
        let end_date_str: Option<String> = row.try_get("end_date")?;
        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        Ok(AutoReplyConfig {
            email: row.try_get("email")?,
            is_active: row.try_get("is_active")?,
            start_date: start_date_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            end_date: end_date_str
                .and_then(|s| DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
            subject: row.try_get("subject")?,
            body_html: row.try_get("body_html")?,
            body_text: row.try_get("body_text")?,
            reply_interval_hours: row.try_get("reply_interval_hours")?,
            created_at: DateTime::parse_from_rfc3339(&created_at_str)
                .map_err(|e| MailError::Parse(e.to_string()))?
                .with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&updated_at_str)
                .map_err(|e| MailError::Parse(e.to_string()))?
                .with_timezone(&Utc),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn setup_test_db() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let manager = AutoReplyManager::new(pool.clone());
        manager.init_db().await.unwrap();
        pool
    }

    #[tokio::test]
    async fn test_create_and_get_config() {
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

        let retrieved = manager.get_config("test@example.com").await.unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().email, "test@example.com");
    }

    #[tokio::test]
    async fn test_should_send_auto_reply() {
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

        // Should send (first time)
        let should_send = manager
            .should_send_auto_reply("test@example.com", "sender@example.com")
            .await
            .unwrap();
        assert!(should_send);

        // Record that we sent
        manager
            .record_auto_reply_sent("test@example.com", "sender@example.com")
            .await
            .unwrap();

        // Should NOT send again immediately
        let should_send = manager
            .should_send_auto_reply("test@example.com", "sender@example.com")
            .await
            .unwrap();
        assert!(!should_send);
    }

    #[tokio::test]
    async fn test_delete_config() {
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

        manager
            .set_config("test@example.com", request)
            .await
            .unwrap();

        manager.delete_config("test@example.com").await.unwrap();

        let config = manager.get_config("test@example.com").await.unwrap();
        assert!(config.is_none());
    }
}
