//! Sieve manager for database persistence

use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::executor::SieveExecutor;
use super::parser::{parse_script, validate_script};
use super::types::*;

/// Sieve manager for script storage and execution
pub struct SieveManager {
    db: SqlitePool,
}

impl SieveManager {
    /// Create a new Sieve manager
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Initialize database tables
    pub async fn init_db(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sieve_scripts (
                id TEXT PRIMARY KEY,
                owner_email TEXT NOT NULL,
                name TEXT NOT NULL,
                script_content TEXT NOT NULL,
                is_active INTEGER DEFAULT 0,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sieve_scripts_owner ON sieve_scripts(owner_email)
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS sieve_logs (
                id TEXT PRIMARY KEY,
                owner_email TEXT NOT NULL,
                script_id TEXT,
                message_id TEXT,
                action_taken TEXT NOT NULL,
                executed_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_sieve_logs_owner ON sieve_logs(owner_email)
            "#,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Create a new script
    pub async fn create_script(
        &self,
        email: &str,
        request: &CreateSieveScriptRequest,
    ) -> Result<SieveScript> {
        // Validate the script first
        let validation = validate_script(&request.script_content)?;
        if !validation.valid {
            return Err(anyhow!(
                "Invalid script: {}",
                validation.error.unwrap_or_default()
            ));
        }

        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        // If activating this script, deactivate others
        if request.activate {
            sqlx::query("UPDATE sieve_scripts SET is_active = 0 WHERE owner_email = ?")
                .bind(email)
                .execute(&self.db)
                .await?;
        }

        sqlx::query(
            r#"
            INSERT INTO sieve_scripts (id, owner_email, name, script_content, is_active, created_at, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(email)
        .bind(&request.name)
        .bind(&request.script_content)
        .bind(if request.activate { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(SieveScript {
            id,
            owner_email: email.to_string(),
            name: request.name.clone(),
            script_content: request.script_content.clone(),
            rules: None,
            is_active: request.activate,
            created_at: now,
            updated_at: now,
        })
    }

    /// List scripts for a user
    pub async fn list_scripts(&self, email: &str) -> Result<Vec<SieveScript>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, i32, String, String)>(
            r#"
            SELECT id, owner_email, name, script_content, is_active, created_at, updated_at
            FROM sieve_scripts
            WHERE owner_email = ?
            ORDER BY created_at DESC
            "#,
        )
        .bind(email)
        .fetch_all(&self.db)
        .await?;

        let scripts = rows
            .into_iter()
            .map(
                |(id, owner_email, name, script_content, is_active, created_at, updated_at)| {
                    SieveScript {
                        id,
                        owner_email,
                        name,
                        script_content,
                        rules: None,
                        is_active: is_active == 1,
                        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                            .map(|d| d.with_timezone(&Utc))
                            .unwrap_or_else(|_| Utc::now()),
                    }
                },
            )
            .collect();

        Ok(scripts)
    }

    /// Get a specific script
    pub async fn get_script(&self, email: &str, script_id: &str) -> Result<Option<SieveScript>> {
        let row = sqlx::query_as::<_, (String, String, String, String, i32, String, String)>(
            r#"
            SELECT id, owner_email, name, script_content, is_active, created_at, updated_at
            FROM sieve_scripts
            WHERE id = ? AND owner_email = ?
            "#,
        )
        .bind(script_id)
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(
            |(id, owner_email, name, script_content, is_active, created_at, updated_at)| {
                SieveScript {
                    id,
                    owner_email,
                    name,
                    script_content,
                    rules: None,
                    is_active: is_active == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }
            },
        ))
    }

    /// Get active script for a user
    pub async fn get_active_script(&self, email: &str) -> Result<Option<SieveScript>> {
        let row = sqlx::query_as::<_, (String, String, String, String, i32, String, String)>(
            r#"
            SELECT id, owner_email, name, script_content, is_active, created_at, updated_at
            FROM sieve_scripts
            WHERE owner_email = ? AND is_active = 1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(
            |(id, owner_email, name, script_content, is_active, created_at, updated_at)| {
                SieveScript {
                    id,
                    owner_email,
                    name,
                    script_content,
                    rules: None,
                    is_active: is_active == 1,
                    created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                    updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                }
            },
        ))
    }

    /// Update a script
    pub async fn update_script(
        &self,
        email: &str,
        script_id: &str,
        request: &CreateSieveScriptRequest,
    ) -> Result<SieveScript> {
        // Validate the script first
        let validation = validate_script(&request.script_content)?;
        if !validation.valid {
            return Err(anyhow!(
                "Invalid script: {}",
                validation.error.unwrap_or_default()
            ));
        }

        // Check if script exists
        let existing = self.get_script(email, script_id).await?;
        if existing.is_none() {
            return Err(anyhow!("Script not found"));
        }

        let now = Utc::now();

        // If activating this script, deactivate others
        if request.activate {
            sqlx::query("UPDATE sieve_scripts SET is_active = 0 WHERE owner_email = ?")
                .bind(email)
                .execute(&self.db)
                .await?;
        }

        sqlx::query(
            r#"
            UPDATE sieve_scripts
            SET name = ?, script_content = ?, is_active = ?, updated_at = ?
            WHERE id = ? AND owner_email = ?
            "#,
        )
        .bind(&request.name)
        .bind(&request.script_content)
        .bind(if request.activate { 1 } else { 0 })
        .bind(now.to_rfc3339())
        .bind(script_id)
        .bind(email)
        .execute(&self.db)
        .await?;

        self.get_script(email, script_id)
            .await?
            .ok_or_else(|| anyhow!("Script not found after update"))
    }

    /// Delete a script
    pub async fn delete_script(&self, email: &str, script_id: &str) -> Result<()> {
        let result = sqlx::query("DELETE FROM sieve_scripts WHERE id = ? AND owner_email = ?")
            .bind(script_id)
            .bind(email)
            .execute(&self.db)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Script not found"));
        }

        Ok(())
    }

    /// Activate a script
    pub async fn activate_script(&self, email: &str, script_id: &str) -> Result<()> {
        // Check if script exists
        let existing = self.get_script(email, script_id).await?;
        if existing.is_none() {
            return Err(anyhow!("Script not found"));
        }

        // Deactivate all scripts for this user
        sqlx::query("UPDATE sieve_scripts SET is_active = 0 WHERE owner_email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        // Activate the specified script
        sqlx::query("UPDATE sieve_scripts SET is_active = 1 WHERE id = ? AND owner_email = ?")
            .bind(script_id)
            .bind(email)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Deactivate a script
    pub async fn deactivate_script(&self, email: &str, script_id: &str) -> Result<()> {
        let result = sqlx::query(
            "UPDATE sieve_scripts SET is_active = 0 WHERE id = ? AND owner_email = ?",
        )
        .bind(script_id)
        .bind(email)
        .execute(&self.db)
        .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Script not found"));
        }

        Ok(())
    }

    /// Validate a script without saving
    pub fn validate_script(&self, script: &str) -> Result<ValidationResult> {
        validate_script(script)
    }

    /// Execute the active script on a message
    pub async fn execute_for_message(
        &self,
        email: &str,
        message: &MessageContext,
        message_id: &str,
    ) -> Result<SieveResult> {
        let script = self.get_active_script(email).await?;

        let result = match script {
            Some(script) => {
                let rules = parse_script(&script.script_content)?;
                let result = SieveExecutor::execute(&rules, message)?;

                // Log the execution
                self.log_execution(
                    email,
                    &script.id,
                    message_id,
                    &format!("{:?}", result.actions),
                )
                .await?;

                result
            }
            None => {
                // No active script, use implicit keep
                SieveResult::default()
            }
        };

        Ok(result)
    }

    /// Log script execution
    async fn log_execution(
        &self,
        email: &str,
        script_id: &str,
        message_id: &str,
        action_taken: &str,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();

        sqlx::query(
            r#"
            INSERT INTO sieve_logs (id, owner_email, script_id, message_id, action_taken, executed_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(email)
        .bind(script_id)
        .bind(message_id)
        .bind(action_taken)
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get execution logs for a user
    pub async fn get_logs(&self, email: &str, limit: u32) -> Result<Vec<SieveLog>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, String)>(
            r#"
            SELECT id, owner_email, script_id, message_id, action_taken, executed_at
            FROM sieve_logs
            WHERE owner_email = ?
            ORDER BY executed_at DESC
            LIMIT ?
            "#,
        )
        .bind(email)
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        let logs = rows
            .into_iter()
            .map(
                |(id, owner_email, script_id, message_id, action_taken, executed_at)| SieveLog {
                    id,
                    owner_email,
                    script_id,
                    message_id,
                    action_taken,
                    executed_at: chrono::DateTime::parse_from_rfc3339(&executed_at)
                        .map(|d| d.with_timezone(&Utc))
                        .unwrap_or_else(|_| Utc::now()),
                },
            )
            .collect();

        Ok(logs)
    }

    /// Clear logs for a user
    pub async fn clear_logs(&self, email: &str) -> Result<()> {
        sqlx::query("DELETE FROM sieve_logs WHERE owner_email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        Ok(())
    }
}
