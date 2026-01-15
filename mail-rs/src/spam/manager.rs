//! Spam manager for database persistence and API
//!
//! Provides management of spam rules, configuration, and logging.

use anyhow::Result;
use chrono::Utc;
use sqlx::SqlitePool;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::scorer::SpamScorer;
use super::types::*;

/// Spam management statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct SpamStats {
    /// Total messages scanned
    pub messages_scanned: u64,
    /// Messages classified as spam
    pub spam_detected: u64,
    /// Messages classified as ham
    pub ham_detected: u64,
    /// Messages quarantined
    pub quarantined: u64,
    /// Bayesian training: spam messages learned
    pub spam_learned: u32,
    /// Bayesian training: ham messages learned
    pub ham_learned: u32,
    /// Number of active rules
    pub active_rules: usize,
    /// Total unique tokens in Bayesian database
    pub bayesian_tokens: usize,
}

/// Spam manager
pub struct SpamManager {
    db: SqlitePool,
    scorer: Arc<RwLock<SpamScorer>>,
}

impl SpamManager {
    /// Create a new spam manager
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            scorer: Arc::new(RwLock::new(SpamScorer::default())),
        }
    }

    /// Initialize database tables
    pub async fn init_db(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS spam_config (
                id TEXT PRIMARY KEY,
                owner_email TEXT,
                spam_threshold REAL DEFAULT 5.0,
                ham_threshold REAL DEFAULT -0.5,
                quarantine_enabled INTEGER DEFAULT 1,
                learning_enabled INTEGER DEFAULT 1,
                quarantine_folder TEXT DEFAULT 'Spam'
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS spam_rules (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                rule_type TEXT NOT NULL,
                pattern TEXT,
                score REAL NOT NULL,
                is_enabled INTEGER DEFAULT 1
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS spam_tokens (
                id TEXT PRIMARY KEY,
                token TEXT NOT NULL UNIQUE,
                spam_count INTEGER DEFAULT 0,
                ham_count INTEGER DEFAULT 0,
                updated_at TEXT
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS spam_log (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL,
                recipient_email TEXT NOT NULL,
                from_address TEXT,
                subject TEXT,
                total_score REAL NOT NULL,
                rules_matched TEXT,
                action_taken TEXT NOT NULL,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Load Bayesian tokens
        self.load_tokens().await?;

        // Load custom rules
        self.load_rules().await?;

        Ok(())
    }

    /// Load Bayesian tokens from database
    async fn load_tokens(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, (String, i64, i64)>(
            "SELECT token, spam_count, ham_count FROM spam_tokens"
        )
        .fetch_all(&self.db)
        .await?;

        let tokens: Vec<(String, u32, u32)> = rows
            .into_iter()
            .map(|(t, s, h)| (t, s as u32, h as u32))
            .collect();

        let mut scorer = self.scorer.write().await;
        scorer.bayesian_mut().load_tokens(tokens);

        Ok(())
    }

    /// Load custom rules from database
    async fn load_rules(&self) -> Result<()> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, f64, i64)>(
            "SELECT id, name, description, rule_type, pattern, score, is_enabled FROM spam_rules"
        )
        .fetch_all(&self.db)
        .await?;

        if !rows.is_empty() {
            let rules: Vec<SpamRule> = rows
                .into_iter()
                .map(|(id, name, description, rule_type, pattern, score, is_enabled)| SpamRule {
                    id,
                    name,
                    description,
                    rule_type: match rule_type.as_str() {
                        "Header" => SpamRuleType::Header,
                        "Body" => SpamRuleType::Body,
                        "Dns" => SpamRuleType::Dns,
                        "Bayesian" => SpamRuleType::Bayesian,
                        _ => SpamRuleType::Regex,
                    },
                    pattern,
                    score,
                    is_enabled: is_enabled != 0,
                })
                .collect();

            let mut scorer = self.scorer.write().await;
            scorer.set_rules(rules);
        }

        Ok(())
    }

    /// Get spam config for a user
    pub async fn get_config(&self, email: Option<&str>) -> Result<SpamConfig> {
        let row = if let Some(email) = email {
            sqlx::query_as::<_, (f64, f64, i64, i64, String)>(
                "SELECT spam_threshold, ham_threshold, quarantine_enabled, learning_enabled, quarantine_folder FROM spam_config WHERE owner_email = ?"
            )
            .bind(email)
            .fetch_optional(&self.db)
            .await?
        } else {
            sqlx::query_as::<_, (f64, f64, i64, i64, String)>(
                "SELECT spam_threshold, ham_threshold, quarantine_enabled, learning_enabled, quarantine_folder FROM spam_config WHERE owner_email IS NULL LIMIT 1"
            )
            .fetch_optional(&self.db)
            .await?
        };

        if let Some((spam_threshold, ham_threshold, quarantine_enabled, learning_enabled, quarantine_folder)) = row {
            Ok(SpamConfig {
                spam_threshold,
                ham_threshold,
                quarantine_enabled: quarantine_enabled != 0,
                learning_enabled: learning_enabled != 0,
                quarantine_folder,
            })
        } else {
            Ok(SpamConfig::default())
        }
    }

    /// Update spam config
    pub async fn update_config(&self, email: Option<&str>, config: &SpamConfig) -> Result<()> {
        let id = Uuid::new_v4().to_string();

        sqlx::query(
            r#"
            INSERT OR REPLACE INTO spam_config (id, owner_email, spam_threshold, ham_threshold, quarantine_enabled, learning_enabled, quarantine_folder)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#
        )
        .bind(&id)
        .bind(email)
        .bind(config.spam_threshold)
        .bind(config.ham_threshold)
        .bind(config.quarantine_enabled as i64)
        .bind(config.learning_enabled as i64)
        .bind(&config.quarantine_folder)
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// List all spam rules
    pub async fn list_rules(&self) -> Result<Vec<SpamRule>> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String, f64, i64)>(
            "SELECT id, name, description, rule_type, pattern, score, is_enabled FROM spam_rules ORDER BY name"
        )
        .fetch_all(&self.db)
        .await?;

        let rules: Vec<SpamRule> = rows
            .into_iter()
            .map(|(id, name, description, rule_type, pattern, score, is_enabled)| SpamRule {
                id,
                name,
                description,
                rule_type: match rule_type.as_str() {
                    "Header" => SpamRuleType::Header,
                    "Body" => SpamRuleType::Body,
                    "Dns" => SpamRuleType::Dns,
                    "Bayesian" => SpamRuleType::Bayesian,
                    _ => SpamRuleType::Regex,
                },
                pattern,
                score,
                is_enabled: is_enabled != 0,
            })
            .collect();

        Ok(rules)
    }

    /// Create a spam rule
    pub async fn create_rule(&self, rule: &SpamRule) -> Result<SpamRule> {
        let id = Uuid::new_v4().to_string();
        let rule_type = match rule.rule_type {
            SpamRuleType::Header => "Header",
            SpamRuleType::Body => "Body",
            SpamRuleType::Dns => "Dns",
            SpamRuleType::Bayesian => "Bayesian",
            SpamRuleType::Regex => "Regex",
        };

        sqlx::query(
            "INSERT INTO spam_rules (id, name, description, rule_type, pattern, score, is_enabled) VALUES (?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(rule_type)
        .bind(&rule.pattern)
        .bind(rule.score)
        .bind(rule.is_enabled as i64)
        .execute(&self.db)
        .await?;

        // Reload rules
        self.load_rules().await?;

        Ok(SpamRule {
            id,
            ..rule.clone()
        })
    }

    /// Update a spam rule
    pub async fn update_rule(&self, id: &str, rule: &SpamRule) -> Result<SpamRule> {
        let rule_type = match rule.rule_type {
            SpamRuleType::Header => "Header",
            SpamRuleType::Body => "Body",
            SpamRuleType::Dns => "Dns",
            SpamRuleType::Bayesian => "Bayesian",
            SpamRuleType::Regex => "Regex",
        };

        sqlx::query(
            "UPDATE spam_rules SET name = ?, description = ?, rule_type = ?, pattern = ?, score = ?, is_enabled = ? WHERE id = ?"
        )
        .bind(&rule.name)
        .bind(&rule.description)
        .bind(rule_type)
        .bind(&rule.pattern)
        .bind(rule.score)
        .bind(rule.is_enabled as i64)
        .bind(id)
        .execute(&self.db)
        .await?;

        // Reload rules
        self.load_rules().await?;

        Ok(SpamRule {
            id: id.to_string(),
            ..rule.clone()
        })
    }

    /// Delete a spam rule
    pub async fn delete_rule(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM spam_rules WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        // Reload rules
        self.load_rules().await?;

        Ok(())
    }

    /// Score a message
    pub async fn score_message(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        headers: &[(String, String)],
    ) -> SpamResult {
        let scorer = self.scorer.read().await;
        scorer.score(from, to, subject, body, headers)
    }

    /// Log a spam check result
    pub async fn log_result(
        &self,
        message_id: &str,
        recipient_email: &str,
        from_address: &str,
        subject: &str,
        result: &SpamResult,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let action = match result.action {
            SpamAction::Deliver => "Deliver",
            SpamAction::AddHeaders => "AddHeaders",
            SpamAction::Quarantine => "Quarantine",
            SpamAction::Reject => "Reject",
        };
        let rules_json = serde_json::to_string(&result.rules_matched)?;

        sqlx::query(
            "INSERT INTO spam_log (id, message_id, recipient_email, from_address, subject, total_score, rules_matched, action_taken, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(&id)
        .bind(message_id)
        .bind(recipient_email)
        .bind(from_address)
        .bind(subject)
        .bind(result.score)
        .bind(&rules_json)
        .bind(action)
        .bind(Utc::now().to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(())
    }

    /// Get spam logs
    pub async fn get_logs(&self, limit: i64) -> Result<Vec<SpamLog>> {
        let rows = sqlx::query_as::<_, (String, String, String, f64, String, String, String)>(
            "SELECT id, message_id, recipient_email, total_score, rules_matched, action_taken, created_at FROM spam_log ORDER BY created_at DESC LIMIT ?"
        )
        .bind(limit)
        .fetch_all(&self.db)
        .await?;

        let logs: Vec<SpamLog> = rows
            .into_iter()
            .map(|(id, message_id, recipient_email, total_score, rules_matched, action_taken, created_at)| SpamLog {
                id,
                message_id,
                recipient_email,
                total_score,
                rules_matched,
                action_taken: match action_taken.as_str() {
                    "AddHeaders" => SpamAction::AddHeaders,
                    "Quarantine" => SpamAction::Quarantine,
                    "Reject" => SpamAction::Reject,
                    _ => SpamAction::Deliver,
                },
                created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
                    .map(|d| d.with_timezone(&Utc))
                    .unwrap_or_else(|_| Utc::now()),
            })
            .collect();

        Ok(logs)
    }

    /// Clear spam logs
    pub async fn clear_logs(&self) -> Result<()> {
        sqlx::query("DELETE FROM spam_log")
            .execute(&self.db)
            .await?;
        Ok(())
    }

    /// Learn from spam message
    pub async fn learn_spam(&self, body: &str) -> Result<()> {
        {
            let mut scorer = self.scorer.write().await;
            scorer.learn_spam(body);
        }

        // Save tokens
        self.save_tokens().await?;

        Ok(())
    }

    /// Learn from ham message
    pub async fn learn_ham(&self, body: &str) -> Result<()> {
        {
            let mut scorer = self.scorer.write().await;
            scorer.learn_ham(body);
        }

        // Save tokens
        self.save_tokens().await?;

        Ok(())
    }

    /// Save Bayesian tokens to database
    async fn save_tokens(&self) -> Result<()> {
        let tokens = {
            let scorer = self.scorer.read().await;
            scorer.bayesian().get_tokens()
        };

        // Batch update tokens
        for (token, spam_count, ham_count) in tokens {
            let id = Uuid::new_v4().to_string();
            sqlx::query(
                r#"
                INSERT OR REPLACE INTO spam_tokens (id, token, spam_count, ham_count, updated_at)
                VALUES (?, ?, ?, ?, ?)
                "#
            )
            .bind(&id)
            .bind(&token)
            .bind(spam_count as i64)
            .bind(ham_count as i64)
            .bind(Utc::now().to_rfc3339())
            .execute(&self.db)
            .await?;
        }

        Ok(())
    }

    /// Get spam statistics
    pub async fn get_stats(&self) -> Result<SpamStats> {
        // Count messages
        let (messages_scanned,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM spam_log"
        )
        .fetch_one(&self.db)
        .await?;

        let (spam_detected,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM spam_log WHERE total_score >= 5.0"
        )
        .fetch_one(&self.db)
        .await?;

        let (quarantined,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM spam_log WHERE action_taken = 'Quarantine'"
        )
        .fetch_one(&self.db)
        .await?;

        let (active_rules,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM spam_rules WHERE is_enabled = 1"
        )
        .fetch_one(&self.db)
        .await?;

        let (bayesian_tokens,): (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM spam_tokens"
        )
        .fetch_one(&self.db)
        .await?;

        let scorer = self.scorer.read().await;
        let (spam_learned, ham_learned) = scorer.bayesian().training_counts();

        Ok(SpamStats {
            messages_scanned: messages_scanned as u64,
            spam_detected: spam_detected as u64,
            ham_detected: (messages_scanned - spam_detected) as u64,
            quarantined: quarantined as u64,
            spam_learned,
            ham_learned,
            active_rules: active_rules as usize,
            bayesian_tokens: bayesian_tokens as usize,
        })
    }

    /// Test a message against spam rules without logging
    pub async fn test_message(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
    ) -> SpamResult {
        self.score_message(from, to, subject, body, &[]).await
    }
}
