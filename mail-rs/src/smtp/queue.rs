//! SMTP queue system with retry logic
//!
//! This module manages outgoing email queue with automatic retry on failures.
//!
//! # Features
//! - Persistent queue (SQLite)
//! - Retry with exponential backoff
//! - Maximum retry attempts
//! - Bounce handling
//!
//! # Architecture
//! ```text
//! ┌─────────┐
//! │ Enqueue │ → [Queue] → [Retry Worker] → [SMTP Client] → ✓ Sent
//! └─────────┘      ↑           ↓                              ↓
//!                  └──── Failed ←─────────────────────── X Failed
//! ```

use crate::error::{MailError, Result};
use crate::smtp::SmtpClient;
use crate::utils::dns::lookup_mx;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

/// Maximum number of retry attempts before giving up
const MAX_RETRY_ATTEMPTS: i32 = 5;

/// Base delay for retry (2 minutes)
const RETRY_BASE_DELAY_SECS: i64 = 120;

/// Queue entry status
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum QueueStatus {
    Pending,
    Sending,
    Sent,
    Failed,
    Bounced,
}

/// A queued email
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedEmail {
    pub id: String,
    pub from_addr: String,
    pub to_addr: String,
    pub data: Vec<u8>,
    pub status: QueueStatus,
    pub retry_count: i32,
    pub last_error: Option<String>,
    pub created_at: DateTime<Utc>,
    pub next_retry_at: Option<DateTime<Utc>>,
}

/// SMTP queue manager
pub struct SmtpQueue {
    db: Arc<SqlitePool>,
}

impl SmtpQueue {
    /// Create a new SMTP queue
    pub async fn new(database_url: &str) -> Result<Self> {
        let db = SqlitePool::connect(database_url).await?;

        // Create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS smtp_queue (
                id TEXT PRIMARY KEY,
                from_addr TEXT NOT NULL,
                to_addr TEXT NOT NULL,
                data BLOB NOT NULL,
                status TEXT NOT NULL,
                retry_count INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                created_at TEXT NOT NULL,
                next_retry_at TEXT
            )
            "#,
        )
        .execute(&db)
        .await?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Enqueue an email for sending
    ///
    /// # Arguments
    /// * `from` - Sender email address
    /// * `to` - Recipient email address
    /// * `data` - Email content (headers + body)
    ///
    /// # Returns
    /// ID of the queued email
    pub async fn enqueue(&self, from: &str, to: &str, data: &[u8]) -> Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        info!("Enqueuing email from {} to {}: {}", from, to, id);

        sqlx::query(
            r#"
            INSERT INTO smtp_queue (
                id, from_addr, to_addr, data, status,
                retry_count, created_at, next_retry_at
            ) VALUES (?, ?, ?, ?, 'pending', 0, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(from)
        .bind(to)
        .bind(data)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&*self.db)
        .await?;

        Ok(id)
    }

    /// Get pending emails ready for sending
    pub async fn get_pending(&self, limit: i64) -> Result<Vec<QueuedEmail>> {
        let now = Utc::now();

        let rows = sqlx::query_as::<_, (String, String, String, Vec<u8>, String, i32, Option<String>, String, Option<String>)>(
            r#"
            SELECT id, from_addr, to_addr, data, status, retry_count, last_error, created_at, next_retry_at
            FROM smtp_queue
            WHERE status = 'pending'
              AND (next_retry_at IS NULL OR next_retry_at <= ?)
            ORDER BY created_at ASC
            LIMIT ?
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(limit)
        .fetch_all(&*self.db)
        .await?;

        let emails: Result<Vec<QueuedEmail>> = rows
            .into_iter()
            .map(|(id, from, to, data, status, retry, error, created, next_retry)| {
                Ok(QueuedEmail {
                    id,
                    from_addr: from,
                    to_addr: to,
                    data,
                    status: match status.as_str() {
                        "pending" => QueueStatus::Pending,
                        "sending" => QueueStatus::Sending,
                        "sent" => QueueStatus::Sent,
                        "failed" => QueueStatus::Failed,
                        "bounced" => QueueStatus::Bounced,
                        _ => QueueStatus::Pending,
                    },
                    retry_count: retry,
                    last_error: error,
                    created_at: DateTime::parse_from_rfc3339(&created)
                        .map_err(|e| MailError::Storage(e.to_string()))?
                        .with_timezone(&Utc),
                    next_retry_at: next_retry
                        .map(|s| DateTime::parse_from_rfc3339(&s)
                            .map(|dt| dt.with_timezone(&Utc)))
                        .transpose()
                        .map_err(|e| MailError::Storage(e.to_string()))?,
                })
            })
            .collect();

        emails
    }

    /// Mark email as sent
    pub async fn mark_sent(&self, id: &str) -> Result<()> {
        info!("Marking email {} as sent", id);

        sqlx::query(
            r#"
            UPDATE smtp_queue
            SET status = 'sent'
            WHERE id = ?
            "#,
        )
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// Mark email as failed and schedule retry
    pub async fn mark_failed(&self, id: &str, error_msg: &str, retry_count: i32) -> Result<()> {
        if retry_count >= MAX_RETRY_ATTEMPTS {
            warn!("Email {} exceeded max retries, marking as bounced", id);
            return self.mark_bounced(id, error_msg).await;
        }

        // Calculate next retry time with exponential backoff
        let delay_secs = RETRY_BASE_DELAY_SECS * 2_i64.pow(retry_count as u32);
        let next_retry = Utc::now() + Duration::seconds(delay_secs);

        info!(
            "Marking email {} as failed (attempt {}), next retry at {}",
            id, retry_count + 1, next_retry
        );

        sqlx::query(
            r#"
            UPDATE smtp_queue
            SET status = 'pending',
                retry_count = ?,
                last_error = ?,
                next_retry_at = ?
            WHERE id = ?
            "#,
        )
        .bind(retry_count + 1)
        .bind(error_msg)
        .bind(next_retry.to_rfc3339())
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// Mark email as permanently bounced
    pub async fn mark_bounced(&self, id: &str, error_msg: &str) -> Result<()> {
        error!("Email {} bounced: {}", id, error_msg);

        sqlx::query(
            r#"
            UPDATE smtp_queue
            SET status = 'bounced',
                last_error = ?
            WHERE id = ?
            "#,
        )
        .bind(error_msg)
        .bind(id)
        .execute(&*self.db)
        .await?;

        Ok(())
    }

    /// Process queue - send pending emails
    pub async fn process_queue(&self) -> Result<usize> {
        debug!("Processing queue");

        let pending = self.get_pending(10).await?;
        let count = pending.len();

        for email in pending {
            if let Err(e) = self.process_email(&email).await {
                error!("Failed to process email {}: {}", email.id, e);
                self.mark_failed(&email.id, &e.to_string(), email.retry_count).await?;
            } else {
                self.mark_sent(&email.id).await?;
            }
        }

        if count > 0 {
            info!("Processed {} emails from queue", count);
        }

        Ok(count)
    }

    /// Process a single email
    async fn process_email(&self, email: &QueuedEmail) -> Result<()> {
        info!("Processing email {}: {} -> {}", email.id, email.from_addr, email.to_addr);

        // Extract domain from recipient
        let domain = email
            .to_addr
            .split('@')
            .nth(1)
            .ok_or_else(|| MailError::InvalidEmail("Invalid recipient address".to_string()))?;

        // Lookup MX records
        let mx_servers = lookup_mx(domain).await?;

        if mx_servers.is_empty() {
            return Err(MailError::DnsLookup(format!("No MX records for {}", domain)));
        }

        // Try each MX server in order
        let mut last_error = None;
        for server in &mx_servers {
            info!("Trying to send via {}", server);

            let client = SmtpClient::new(server.clone());
            match client.send_mail(&email.from_addr, &email.to_addr, &email.data).await {
                Ok(_) => {
                    info!("Email {} sent successfully via {}", email.id, server);
                    return Ok(());
                }
                Err(e) => {
                    warn!("Failed to send via {}: {}", server, e);
                    last_error = Some(e);
                }
            }
        }

        // All servers failed
        Err(last_error.unwrap_or_else(|| {
            MailError::SmtpProtocol("All MX servers failed".to_string())
        }))
    }

    /// Start queue worker loop
    pub async fn start_worker(self: Arc<Self>) {
        info!("Starting queue worker");

        loop {
            match self.process_queue().await {
                Ok(count) => {
                    if count == 0 {
                        // No emails processed, sleep longer
                        sleep(std::time::Duration::from_secs(30)).await;
                    } else {
                        // More emails might be pending, check again soon
                        sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
                Err(e) => {
                    error!("Queue processing error: {}", e);
                    sleep(std::time::Duration::from_secs(60)).await;
                }
            }
        }
    }
}
