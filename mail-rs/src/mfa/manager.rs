//! MFA Manager - Database persistence and business logic

use anyhow::Result;
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::Utc;
use sqlx::SqlitePool;
use uuid::Uuid;

use super::totp::TotpService;
use super::types::*;

/// Number of backup codes to generate
const BACKUP_CODE_COUNT: usize = 10;

/// MFA Manager for handling multi-factor authentication
pub struct MfaManager {
    db: SqlitePool,
    totp_service: TotpService,
}

impl MfaManager {
    /// Create a new MFA manager
    pub fn new(db: SqlitePool) -> Self {
        Self {
            db,
            totp_service: TotpService::new(),
        }
    }

    /// Initialize database tables
    pub async fn init_db(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mfa_config (
                email TEXT PRIMARY KEY,
                secret_encrypted TEXT NOT NULL,
                is_enabled INTEGER DEFAULT 0,
                last_used_at TEXT,
                enabled_at TEXT
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mfa_backup_codes (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                code_hash TEXT NOT NULL,
                is_used INTEGER DEFAULT 0,
                used_at TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS mfa_audit_log (
                id TEXT PRIMARY KEY,
                email TEXT NOT NULL,
                event_type TEXT NOT NULL,
                ip_address TEXT,
                user_agent TEXT,
                created_at TEXT NOT NULL
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        // Create indexes
        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mfa_backup_email ON mfa_backup_codes(email)")
            .execute(&self.db)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_mfa_audit_email ON mfa_audit_log(email)")
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Start MFA setup for a user
    pub async fn start_setup(&self, email: &str) -> Result<MfaSetupResponse> {
        let setup = self.totp_service.setup(email)?;

        // Store the secret (not yet enabled)
        sqlx::query(
            r#"
            INSERT INTO mfa_config (email, secret_encrypted, is_enabled)
            VALUES (?, ?, 0)
            ON CONFLICT(email) DO UPDATE SET
                secret_encrypted = excluded.secret_encrypted,
                is_enabled = 0
            "#,
        )
        .bind(email)
        .bind(&setup.secret)
        .execute(&self.db)
        .await?;

        // Log the event
        self.log_event(email, MfaEventType::SetupStarted, None, None)
            .await?;

        Ok(setup)
    }

    /// Complete MFA setup by verifying a code
    pub async fn complete_setup(&self, email: &str, code: &str) -> Result<Vec<String>> {
        // Get the stored secret
        let config = self.get_config(email).await?;

        if config.is_none() {
            return Err(anyhow::anyhow!("MFA setup not started"));
        }

        let config = config.unwrap();

        // Validate the code
        if !self.totp_service.validate(&config.secret_encrypted, code)? {
            self.log_event(email, MfaEventType::VerifyFailed, None, None)
                .await?;
            return Err(anyhow::anyhow!("Invalid TOTP code"));
        }

        // Enable MFA
        let now = Utc::now();
        sqlx::query(
            r#"
            UPDATE mfa_config
            SET is_enabled = 1, enabled_at = ?, last_used_at = ?
            WHERE email = ?
            "#,
        )
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .bind(email)
        .execute(&self.db)
        .await?;

        // Generate backup codes
        let backup_codes = self.generate_backup_codes(email).await?;

        // Log the event
        self.log_event(email, MfaEventType::SetupCompleted, None, None)
            .await?;

        Ok(backup_codes)
    }

    /// Verify a TOTP code
    pub async fn verify(&self, email: &str, code: &str) -> Result<MfaVerifyResult> {
        let config = self.get_config(email).await?;

        if config.is_none() {
            return Ok(MfaVerifyResult::NotEnabled);
        }

        let config = config.unwrap();

        if !config.is_enabled {
            return Ok(MfaVerifyResult::NotEnabled);
        }

        // Validate the code
        if self.totp_service.validate(&config.secret_encrypted, code)? {
            // Update last used timestamp
            sqlx::query("UPDATE mfa_config SET last_used_at = ? WHERE email = ?")
                .bind(Utc::now().to_rfc3339())
                .bind(email)
                .execute(&self.db)
                .await?;

            self.log_event(email, MfaEventType::VerifySuccess, None, None)
                .await?;

            return Ok(MfaVerifyResult::Valid);
        }

        // Check if it's a backup code
        if self.verify_backup_code(email, code).await? {
            self.log_event(email, MfaEventType::BackupCodeUsed, None, None)
                .await?;
            return Ok(MfaVerifyResult::Valid);
        }

        self.log_event(email, MfaEventType::VerifyFailed, None, None)
            .await?;

        Ok(MfaVerifyResult::Invalid)
    }

    /// Disable MFA for a user
    pub async fn disable(&self, email: &str, code: &str) -> Result<()> {
        // First verify the current code
        let result = self.verify(email, code).await?;

        if result != MfaVerifyResult::Valid {
            return Err(anyhow::anyhow!("Invalid TOTP code"));
        }

        // Delete MFA config
        sqlx::query("DELETE FROM mfa_config WHERE email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        // Delete backup codes
        sqlx::query("DELETE FROM mfa_backup_codes WHERE email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        // Log the event
        self.log_event(email, MfaEventType::Disabled, None, None)
            .await?;

        Ok(())
    }

    /// Get MFA status for a user
    pub async fn get_status(&self, email: &str) -> Result<MfaStatusResponse> {
        let config = self.get_config(email).await?;

        let (is_enabled, enabled_at, last_used_at) = match config {
            Some(c) => (c.is_enabled, c.enabled_at, c.last_used_at),
            None => (false, None, None),
        };

        let backup_codes_remaining = self.count_remaining_backup_codes(email).await?;

        Ok(MfaStatusResponse {
            is_enabled,
            backup_codes_remaining,
            enabled_at,
            last_used_at,
        })
    }

    /// Check if MFA is enabled for a user
    pub async fn is_enabled(&self, email: &str) -> Result<bool> {
        let config = self.get_config(email).await?;
        Ok(config.map(|c| c.is_enabled).unwrap_or(false))
    }

    /// Generate new backup codes (replaces existing ones)
    pub async fn generate_backup_codes(&self, email: &str) -> Result<Vec<String>> {
        // Delete existing backup codes
        sqlx::query("DELETE FROM mfa_backup_codes WHERE email = ?")
            .bind(email)
            .execute(&self.db)
            .await?;

        let mut codes = Vec::with_capacity(BACKUP_CODE_COUNT);
        let argon2 = Argon2::default();

        for _ in 0..BACKUP_CODE_COUNT {
            // Generate a random 8-character code
            let code = generate_backup_code();
            let salt = SaltString::generate(&mut OsRng);
            let hash = argon2
                .hash_password(code.as_bytes(), &salt)
                .map_err(|e| anyhow::anyhow!("Failed to hash backup code: {}", e))?
                .to_string();

            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();

            sqlx::query(
                "INSERT INTO mfa_backup_codes (id, email, code_hash, is_used, created_at) VALUES (?, ?, ?, 0, ?)",
            )
            .bind(&id)
            .bind(email)
            .bind(&hash)
            .bind(&now)
            .execute(&self.db)
            .await?;

            codes.push(code);
        }

        self.log_event(email, MfaEventType::BackupCodesRegenerated, None, None)
            .await?;

        Ok(codes)
    }

    /// Get MFA config for a user
    async fn get_config(&self, email: &str) -> Result<Option<MfaConfig>> {
        let row = sqlx::query_as::<_, (String, String, i32, Option<String>, Option<String>)>(
            "SELECT email, secret_encrypted, is_enabled, last_used_at, enabled_at FROM mfa_config WHERE email = ?",
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await?;

        Ok(row.map(|(email, secret, enabled, last_used, enabled_at)| MfaConfig {
            email,
            secret_encrypted: secret,
            is_enabled: enabled != 0,
            last_used_at: last_used.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
            enabled_at: enabled_at.and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc))),
        }))
    }

    /// Verify a backup code
    async fn verify_backup_code(&self, email: &str, code: &str) -> Result<bool> {
        let codes = sqlx::query_as::<_, (String, String)>(
            "SELECT id, code_hash FROM mfa_backup_codes WHERE email = ? AND is_used = 0",
        )
        .bind(email)
        .fetch_all(&self.db)
        .await?;

        let argon2 = Argon2::default();

        for (id, hash) in codes {
            let parsed_hash = match PasswordHash::new(&hash) {
                Ok(h) => h,
                Err(_) => continue,
            };

            if argon2.verify_password(code.as_bytes(), &parsed_hash).is_ok() {
                // Mark code as used
                sqlx::query("UPDATE mfa_backup_codes SET is_used = 1, used_at = ? WHERE id = ?")
                    .bind(Utc::now().to_rfc3339())
                    .bind(&id)
                    .execute(&self.db)
                    .await?;

                return Ok(true);
            }
        }

        Ok(false)
    }

    /// Count remaining backup codes
    async fn count_remaining_backup_codes(&self, email: &str) -> Result<usize> {
        let count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM mfa_backup_codes WHERE email = ? AND is_used = 0")
                .bind(email)
                .fetch_one(&self.db)
                .await?;

        Ok(count.0 as usize)
    }

    /// Log an MFA event
    async fn log_event(
        &self,
        email: &str,
        event_type: MfaEventType,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<()> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now().to_rfc3339();

        sqlx::query(
            "INSERT INTO mfa_audit_log (id, email, event_type, ip_address, user_agent, created_at) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(email)
        .bind(event_type.to_string())
        .bind(ip_address)
        .bind(user_agent)
        .bind(&now)
        .execute(&self.db)
        .await?;

        Ok(())
    }
}

/// Generate a random backup code (8 alphanumeric characters)
fn generate_backup_code() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();

    (0..8)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_backup_code() {
        let code = generate_backup_code();
        assert_eq!(code.len(), 8);
        assert!(code.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn test_backup_codes_unique() {
        let codes: Vec<String> = (0..10).map(|_| generate_backup_code()).collect();
        let unique_codes: std::collections::HashSet<_> = codes.iter().collect();
        // All codes should be unique (very high probability)
        assert_eq!(codes.len(), unique_codes.len());
    }
}
