use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{sqlite::SqlitePool, Row};
use tracing::{debug, info};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSummary {
    pub id: i64,
    pub user_email: String,
    pub email_id: String,
    pub from_addr: String,
    pub subject: String,
    pub summary: String,
    pub timestamp: String,
    pub is_read: bool,
}

pub struct SummaryStore {
    pool: SqlitePool,
}

impl SummaryStore {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = SqlitePool::connect(database_url).await?;

        // Create table if not exists
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS email_summaries (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user_email TEXT NOT NULL,
                email_id TEXT NOT NULL,
                from_addr TEXT NOT NULL,
                subject TEXT NOT NULL,
                summary TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                is_read INTEGER NOT NULL DEFAULT 0,
                UNIQUE(user_email, email_id)
            )
            "#,
        )
        .execute(&pool)
        .await?;

        info!("📊 Summary store initialized");
        Ok(Self { pool })
    }

    /// Store a new email summary
    pub async fn store_summary(
        &self,
        user_email: &str,
        email_id: &str,
        from_addr: &str,
        subject: &str,
        summary: &str,
    ) -> Result<()> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO email_summaries
            (user_email, email_id, from_addr, subject, summary, timestamp, is_read)
            VALUES (?, ?, ?, ?, ?, ?, 0)
            "#,
        )
        .bind(user_email)
        .bind(email_id)
        .bind(from_addr)
        .bind(subject)
        .bind(summary)
        .bind(timestamp)
        .execute(&self.pool)
        .await?;

        info!("✅ Stored summary for {} (email_id: {})", user_email, email_id);
        Ok(())
    }

    /// Get all unread summaries for a user
    pub async fn get_unread_summaries(&self, user_email: &str) -> Result<Vec<EmailSummary>> {
        let rows = sqlx::query(
            r#"
            SELECT id, user_email, email_id, from_addr, subject, summary, timestamp, is_read
            FROM email_summaries
            WHERE user_email = ? AND is_read = 0
            ORDER BY timestamp DESC
            "#,
        )
        .bind(user_email)
        .fetch_all(&self.pool)
        .await?;

        let summaries: Vec<EmailSummary> = rows
            .into_iter()
            .map(|row| EmailSummary {
                id: row.get("id"),
                user_email: row.get("user_email"),
                email_id: row.get("email_id"),
                from_addr: row.get("from_addr"),
                subject: row.get("subject"),
                summary: row.get("summary"),
                timestamp: row.get("timestamp"),
                is_read: row.get::<i64, _>("is_read") != 0,
            })
            .collect();

        debug!("📬 Found {} unread summaries for {}", summaries.len(), user_email);
        Ok(summaries)
    }

    /// Mark a summary as read
    pub async fn mark_as_read(&self, user_email: &str, email_id: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE email_summaries
            SET is_read = 1
            WHERE user_email = ? AND email_id = ?
            "#,
        )
        .bind(user_email)
        .bind(email_id)
        .execute(&self.pool)
        .await?;

        debug!("✓ Marked email {} as read for {}", email_id, user_email);
        Ok(())
    }

    /// Mark all summaries as read for a user
    pub async fn mark_all_as_read(&self, user_email: &str) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE email_summaries
            SET is_read = 1
            WHERE user_email = ?
            "#,
        )
        .bind(user_email)
        .execute(&self.pool)
        .await?;

        info!("✓ Marked all summaries as read for {}", user_email);
        Ok(())
    }
}
