//! Template manager for CRUD operations

use crate::error::MailError;
use crate::templates::types::{
    CreateTemplateRequest, EmailTemplate, TemplateCategory, TemplateVariable, UpdateTemplateRequest,
};
use chrono::{DateTime, Utc};
use sqlx::SqlitePool;
use uuid::Uuid;

/// Manages email templates with database persistence
pub struct TemplateManager {
    db: SqlitePool,
}

impl TemplateManager {
    /// Create a new template manager
    pub fn new(db: SqlitePool) -> Self {
        Self { db }
    }

    /// Initialize the templates table
    pub async fn init_db(&self) -> Result<(), MailError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS email_templates (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                category TEXT NOT NULL,
                subject TEXT,
                body_html TEXT NOT NULL,
                body_text TEXT,
                variables TEXT,
                is_signature BOOLEAN DEFAULT 0,
                owner_email TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            )
            "#,
        )
        .execute(&self.db)
        .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_owner ON email_templates(owner_email)")
            .execute(&self.db)
            .await?;

        sqlx::query("CREATE INDEX IF NOT EXISTS idx_category ON email_templates(category)")
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Create a new template
    pub async fn create_template(
        &self,
        owner_email: &str,
        request: CreateTemplateRequest,
    ) -> Result<EmailTemplate, MailError> {
        let id = Uuid::new_v4().to_string();
        let now = Utc::now();
        let variables_json = serde_json::to_string(&request.variables)?;

        sqlx::query(
            r#"
            INSERT INTO email_templates (
                id, name, category, subject, body_html, body_text,
                variables, is_signature, owner_email, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&id)
        .bind(&request.name)
        .bind(request.category.to_db_string())
        .bind(&request.subject)
        .bind(&request.body_html)
        .bind(&request.body_text)
        .bind(&variables_json)
        .bind(request.is_signature)
        .bind(owner_email)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&self.db)
        .await?;

        Ok(EmailTemplate {
            id,
            name: request.name,
            category: request.category,
            subject: request.subject,
            body_html: request.body_html,
            body_text: request.body_text,
            variables: request.variables,
            is_signature: request.is_signature,
            owner_email: owner_email.to_string(),
            created_at: now,
            updated_at: now,
        })
    }

    /// Get a template by ID
    pub async fn get_template(&self, id: &str) -> Result<Option<EmailTemplate>, MailError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, category, subject, body_html, body_text,
                   variables, is_signature, owner_email, created_at, updated_at
            FROM email_templates
            WHERE id = ?
            "#,
        )
        .bind(id)
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_template(row)?))
        } else {
            Ok(None)
        }
    }

    /// List all templates for a user
    pub async fn list_templates(&self, owner_email: &str) -> Result<Vec<EmailTemplate>, MailError> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, category, subject, body_html, body_text,
                   variables, is_signature, owner_email, created_at, updated_at
            FROM email_templates
            WHERE owner_email = ?
            ORDER BY category, name
            "#,
        )
        .bind(owner_email)
        .fetch_all(&self.db)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_template(row))
            .collect()
    }

    /// List templates by category
    pub async fn list_templates_by_category(
        &self,
        owner_email: &str,
        category: TemplateCategory,
    ) -> Result<Vec<EmailTemplate>, MailError> {
        let rows = sqlx::query(
            r#"
            SELECT id, name, category, subject, body_html, body_text,
                   variables, is_signature, owner_email, created_at, updated_at
            FROM email_templates
            WHERE owner_email = ? AND category = ?
            ORDER BY name
            "#,
        )
        .bind(owner_email)
        .bind(category.to_db_string())
        .fetch_all(&self.db)
        .await?;

        rows.into_iter()
            .map(|row| self.row_to_template(row))
            .collect()
    }

    /// Update a template
    pub async fn update_template(
        &self,
        id: &str,
        owner_email: &str,
        request: UpdateTemplateRequest,
    ) -> Result<EmailTemplate, MailError> {
        // Verify ownership
        let existing = self.get_template(id).await?;
        let existing = existing.ok_or_else(|| {
            MailError::NotFound(format!("Template not found: {}", id))
        })?;

        if existing.owner_email != owner_email {
            return Err(MailError::Unauthorized("Not owner of this template".to_string()));
        }

        // Build update query dynamically
        let mut updates = Vec::new();
        let mut params: Vec<String> = Vec::new();

        if let Some(name) = request.name {
            updates.push("name = ?");
            params.push(name);
        }
        if let Some(subject) = request.subject {
            updates.push("subject = ?");
            params.push(subject);
        }
        if let Some(body_html) = request.body_html {
            updates.push("body_html = ?");
            params.push(body_html);
        }
        if let Some(body_text) = request.body_text {
            updates.push("body_text = ?");
            params.push(body_text);
        }
        if let Some(variables) = request.variables {
            updates.push("variables = ?");
            params.push(serde_json::to_string(&variables)?);
        }
        if let Some(is_signature) = request.is_signature {
            updates.push("is_signature = ?");
            params.push(is_signature.to_string());
        }

        updates.push("updated_at = ?");
        params.push(Utc::now().to_rfc3339());

        if updates.is_empty() {
            return Ok(existing);
        }

        let query_str = format!(
            "UPDATE email_templates SET {} WHERE id = ?",
            updates.join(", ")
        );

        let mut query = sqlx::query(&query_str);
        for param in params {
            query = query.bind(param);
        }
        query = query.bind(id);

        query.execute(&self.db).await?;

        // Fetch updated template
        self.get_template(id)
            .await?
            .ok_or_else(|| MailError::NotFound("Template disappeared after update".to_string()))
    }

    /// Delete a template
    pub async fn delete_template(&self, id: &str, owner_email: &str) -> Result<(), MailError> {
        // Verify ownership
        let existing = self.get_template(id).await?;
        if let Some(template) = existing {
            if template.owner_email != owner_email {
                return Err(MailError::Unauthorized("Not owner of this template".to_string()));
            }
        } else {
            return Err(MailError::NotFound(format!("Template not found: {}", id)));
        }

        sqlx::query("DELETE FROM email_templates WHERE id = ?")
            .bind(id)
            .execute(&self.db)
            .await?;

        Ok(())
    }

    /// Get the default signature for a user
    pub async fn get_default_signature(
        &self,
        owner_email: &str,
    ) -> Result<Option<EmailTemplate>, MailError> {
        let row = sqlx::query(
            r#"
            SELECT id, name, category, subject, body_html, body_text,
                   variables, is_signature, owner_email, created_at, updated_at
            FROM email_templates
            WHERE owner_email = ? AND is_signature = 1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(owner_email)
        .fetch_optional(&self.db)
        .await?;

        if let Some(row) = row {
            Ok(Some(self.row_to_template(row)?))
        } else {
            Ok(None)
        }
    }

    /// Create default templates for a new user
    pub async fn create_default_templates(&self, owner_email: &str) -> Result<(), MailError> {
        let templates = vec![
            CreateTemplateRequest {
                name: "Professional Signature".to_string(),
                category: TemplateCategory::Signature,
                subject: "".to_string(),
                body_html: r#"<div style="font-family: Arial, sans-serif; color: #333;">
    <p>Best regards,</p>
    <p><strong>{{sender_name}}</strong><br>
    {{job_title}}<br>
    {{company}}<br>
    {{phone}}<br>
    <a href="mailto:{{sender_email}}">{{sender_email}}</a></p>
</div>"#.to_string(),
                body_text: "Best regards,\n{{sender_name}}\n{{job_title}}\n{{company}}\n{{phone}}\n{{sender_email}}".to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "job_title".to_string(),
                        default_value: None,
                        required: true,
                    },
                    TemplateVariable {
                        name: "company".to_string(),
                        default_value: None,
                        required: true,
                    },
                    TemplateVariable {
                        name: "phone".to_string(),
                        default_value: None,
                        required: false,
                    },
                ],
                is_signature: true,
            },
            CreateTemplateRequest {
                name: "Thank You Reply".to_string(),
                category: TemplateCategory::QuickReply,
                subject: "Re: {{subject}}".to_string(),
                body_html: "<p>Hi {{recipient_name}},</p><p>Thank you for your email. I'll get back to you shortly.</p><p>Best regards</p>".to_string(),
                body_text: "Hi {{recipient_name}},\n\nThank you for your email. I'll get back to you shortly.\n\nBest regards".to_string(),
                variables: vec![],
                is_signature: false,
            },
            CreateTemplateRequest {
                name: "Meeting Request".to_string(),
                category: TemplateCategory::QuickReply,
                subject: "Meeting Request: {{meeting_topic}}".to_string(),
                body_html: "<p>Hi {{recipient_name}},</p><p>I'd like to schedule a meeting to discuss {{meeting_topic}}.</p><p>Are you available on {{proposed_date}} at {{proposed_time}}?</p><p>Best regards</p>".to_string(),
                body_text: "Hi {{recipient_name}},\n\nI'd like to schedule a meeting to discuss {{meeting_topic}}.\n\nAre you available on {{proposed_date}} at {{proposed_time}}?\n\nBest regards".to_string(),
                variables: vec![
                    TemplateVariable {
                        name: "meeting_topic".to_string(),
                        default_value: None,
                        required: true,
                    },
                    TemplateVariable {
                        name: "proposed_date".to_string(),
                        default_value: None,
                        required: true,
                    },
                    TemplateVariable {
                        name: "proposed_time".to_string(),
                        default_value: None,
                        required: true,
                    },
                ],
                is_signature: false,
            },
        ];

        for template in templates {
            self.create_template(owner_email, template).await?;
        }

        Ok(())
    }

    /// Convert database row to EmailTemplate
    fn row_to_template(&self, row: sqlx::sqlite::SqliteRow) -> Result<EmailTemplate, MailError> {
        use sqlx::Row;

        let variables_json: String = row.try_get("variables").unwrap_or_default();
        let variables: Vec<TemplateVariable> = serde_json::from_str(&variables_json)
            .unwrap_or_default();

        let category_str: String = row.try_get("category")?;
        let category = TemplateCategory::from_db_string(&category_str)
            .ok_or_else(|| MailError::Parse(format!("Invalid category: {}", category_str)))?;

        let created_at_str: String = row.try_get("created_at")?;
        let updated_at_str: String = row.try_get("updated_at")?;

        let created_at = DateTime::parse_from_rfc3339(&created_at_str)
            .map_err(|e| MailError::Parse(format!("Invalid created_at date: {}", e)))?
            .with_timezone(&Utc);

        let updated_at = DateTime::parse_from_rfc3339(&updated_at_str)
            .map_err(|e| MailError::Parse(format!("Invalid updated_at date: {}", e)))?
            .with_timezone(&Utc);

        Ok(EmailTemplate {
            id: row.try_get("id")?,
            name: row.try_get("name")?,
            category,
            subject: row.try_get("subject").unwrap_or_default(),
            body_html: row.try_get("body_html")?,
            body_text: row.try_get("body_text").unwrap_or_default(),
            variables,
            is_signature: row.try_get("is_signature")?,
            owner_email: row.try_get("owner_email")?,
            created_at,
            updated_at,
        })
    }
}
