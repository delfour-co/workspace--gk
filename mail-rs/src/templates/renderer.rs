//! Template rendering with variable substitution

use crate::templates::EmailTemplate;
use chrono::Utc;
use std::collections::HashMap;

/// Renders email templates by substituting variables
pub struct TemplateRenderer;

impl TemplateRenderer {
    /// Render a template with provided variables
    ///
    /// Substitutes both system variables (sender_name, date, etc.) and custom variables.
    ///
    /// # System Variables
    /// - `{{sender_name}}` - Sender's name
    /// - `{{sender_email}}` - Sender's email address
    /// - `{{recipient_name}}` - Recipient's name
    /// - `{{recipient_email}}` - Recipient's email address
    /// - `{{date}}` - Current date (YYYY-MM-DD)
    /// - `{{time}}` - Current time (HH:MM:SS)
    /// - `{{datetime}}` - Current date and time
    /// - `{{company}}` - Company name (from variables)
    ///
    /// # Arguments
    /// * `template` - The template to render
    /// * `vars` - HashMap of variable names to values
    ///
    /// # Returns
    /// Rendered HTML and text versions
    pub fn render(
        template: &EmailTemplate,
        vars: &HashMap<String, String>,
    ) -> (String, String) {
        let html = Self::render_string(&template.body_html, vars);
        let text = Self::render_string(&template.body_text, vars);
        (html, text)
    }

    /// Render a string with variable substitution
    fn render_string(template_str: &str, vars: &HashMap<String, String>) -> String {
        let mut result = template_str.to_string();

        // System variables - date/time
        let now = Utc::now();
        result = result.replace("{{date}}", &now.format("%Y-%m-%d").to_string());
        result = result.replace("{{time}}", &now.format("%H:%M:%S").to_string());
        result = result.replace("{{datetime}}", &now.format("%Y-%m-%d %H:%M:%S").to_string());

        // Custom and system variables from the vars map
        for (key, value) in vars {
            let placeholder = format!("{{{{{}}}}}", key);
            result = result.replace(&placeholder, value);
        }

        // Replace any remaining unreplaced variables with empty string
        // This prevents showing {{variable}} in the output if a variable wasn't provided
        // But we keep required variables visible for debugging
        result
    }

    /// Render only the subject line
    pub fn render_subject(subject: &str, vars: &HashMap<String, String>) -> String {
        Self::render_string(subject, vars)
    }

    /// Extract all variable names from a template string
    ///
    /// Returns a list of variable names found (without {{ }} markers)
    pub fn extract_variables(template_str: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut chars = template_str.chars().peekable();

        while let Some(c) = chars.next() {
            if c == '{' {
                if chars.peek() == Some(&'{') {
                    chars.next(); // consume second {

                    // Extract variable name until }}
                    let mut var_name = String::new();
                    while let Some(ch) = chars.next() {
                        if ch == '}' && chars.peek() == Some(&'}') {
                            chars.next(); // consume second }
                            if !var_name.is_empty() {
                                variables.push(var_name.trim().to_string());
                            }
                            break;
                        } else {
                            var_name.push(ch);
                        }
                    }
                }
            }
        }

        // Remove duplicates
        variables.sort();
        variables.dedup();
        variables
    }

    /// Validate that all required variables are provided
    pub fn validate_variables(
        template: &EmailTemplate,
        provided_vars: &HashMap<String, String>,
    ) -> Result<(), Vec<String>> {
        let missing: Vec<String> = template
            .variables
            .iter()
            .filter(|v| v.required && !provided_vars.contains_key(&v.name))
            .map(|v| v.name.clone())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::templates::{TemplateCategory, TemplateVariable};

    #[test]
    fn test_render_basic_variables() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "John Doe".to_string());
        vars.insert("company".to_string(), "Acme Inc".to_string());

        let rendered = TemplateRenderer::render_string(
            "Hello {{name}} from {{company}}!",
            &vars,
        );

        assert_eq!(rendered, "Hello John Doe from Acme Inc!");
    }

    #[test]
    fn test_render_date_time_variables() {
        let vars = HashMap::new();
        let rendered = TemplateRenderer::render_string("Today is {{date}} at {{time}}", &vars);

        // Should contain date and time (actual values will vary)
        assert!(rendered.contains("Today is "));
        assert!(rendered.contains(" at "));
        assert!(!rendered.contains("{{date}}"));
        assert!(!rendered.contains("{{time}}"));
    }

    #[test]
    fn test_extract_variables() {
        let template = "Hello {{name}}, your order {{order_id}} is ready. Date: {{date}}";
        let vars = TemplateRenderer::extract_variables(template);

        assert_eq!(vars.len(), 3);
        assert!(vars.contains(&"name".to_string()));
        assert!(vars.contains(&"order_id".to_string()));
        assert!(vars.contains(&"date".to_string()));
    }

    #[test]
    fn test_extract_variables_with_spaces() {
        let template = "Hello {{ name }}, welcome!";
        let vars = TemplateRenderer::extract_variables(template);

        assert_eq!(vars, vec!["name"]);
    }

    #[test]
    fn test_validate_required_variables() {
        let template = EmailTemplate {
            id: "test".to_string(),
            name: "Test".to_string(),
            category: TemplateCategory::Custom,
            subject: "".to_string(),
            body_html: "".to_string(),
            body_text: "".to_string(),
            variables: vec![
                TemplateVariable {
                    name: "required_var".to_string(),
                    default_value: None,
                    required: true,
                },
                TemplateVariable {
                    name: "optional_var".to_string(),
                    default_value: Some("default".to_string()),
                    required: false,
                },
            ],
            is_signature: false,
            owner_email: "test@example.com".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        // Missing required variable
        let vars = HashMap::new();
        let result = TemplateRenderer::validate_variables(&template, &vars);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["required_var"]);

        // All required variables provided
        let mut vars = HashMap::new();
        vars.insert("required_var".to_string(), "value".to_string());
        let result = TemplateRenderer::validate_variables(&template, &vars);
        assert!(result.is_ok());
    }

    #[test]
    fn test_render_full_template() {
        let template = EmailTemplate {
            id: "sig1".to_string(),
            name: "Professional Signature".to_string(),
            category: TemplateCategory::Signature,
            subject: "".to_string(),
            body_html: "<p>Best regards,<br>{{name}}<br>{{title}}<br>{{company}}</p>".to_string(),
            body_text: "Best regards,\n{{name}}\n{{title}}\n{{company}}".to_string(),
            variables: vec![],
            is_signature: true,
            owner_email: "test@example.com".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "John Doe".to_string());
        vars.insert("title".to_string(), "CEO".to_string());
        vars.insert("company".to_string(), "Acme Inc".to_string());

        let (html, text) = TemplateRenderer::render(&template, &vars);

        assert!(html.contains("John Doe"));
        assert!(html.contains("CEO"));
        assert!(html.contains("Acme Inc"));
        assert!(text.contains("John Doe"));
        assert!(text.contains("CEO"));
    }
}
