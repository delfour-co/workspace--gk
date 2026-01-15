//! Spam scoring engine
//!
//! Provides multiple spam detection methods including rule-based scoring
//! and Bayesian classification.

use regex::Regex;
use rust_stemmers::{Algorithm, Stemmer};
use std::collections::HashMap;

use super::types::*;

/// Spam scorer engine
pub struct SpamScorer {
    config: SpamConfig,
    rules: Vec<SpamRule>,
    bayesian: BayesianClassifier,
}

impl SpamScorer {
    /// Create a new spam scorer
    pub fn new(config: SpamConfig) -> Self {
        Self {
            config,
            rules: Self::default_rules(),
            bayesian: BayesianClassifier::new(),
        }
    }

    /// Set custom rules
    pub fn set_rules(&mut self, rules: Vec<SpamRule>) {
        self.rules = rules;
    }

    /// Get current config
    pub fn config(&self) -> &SpamConfig {
        &self.config
    }

    /// Get bayesian classifier (immutable)
    pub fn bayesian(&self) -> &BayesianClassifier {
        &self.bayesian
    }

    /// Get mutable bayesian classifier
    pub fn bayesian_mut(&mut self) -> &mut BayesianClassifier {
        &mut self.bayesian
    }

    /// Score a message using all available methods
    pub fn score(
        &self,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        headers: &[(String, String)],
    ) -> SpamResult {
        let mut total_score = 0.0;
        let mut rules_matched = Vec::new();

        // Run header-based rules
        for rule in self.rules.iter().filter(|r| r.is_enabled) {
            if let Some(score) = self.check_rule(rule, from, to, subject, body, headers) {
                total_score += score;
                rules_matched.push(SpamRuleMatch {
                    rule_name: rule.name.clone(),
                    score,
                    description: rule.description.clone(),
                });
            }
        }

        // Run Bayesian classification if enabled
        if self.config.learning_enabled {
            let bayesian_score = self.bayesian.classify(body);
            if bayesian_score.abs() > 0.1 {
                total_score += bayesian_score * 3.0; // Weight Bayesian score
                rules_matched.push(SpamRuleMatch {
                    rule_name: "BAYESIAN_SCORE".to_string(),
                    score: bayesian_score * 3.0,
                    description: format!("Bayesian spam probability: {:.2}", (bayesian_score + 1.0) / 2.0),
                });
            }
        }

        // Determine action based on score
        let is_spam = total_score >= self.config.spam_threshold;
        let action = if total_score >= self.config.spam_threshold {
            if self.config.quarantine_enabled {
                SpamAction::Quarantine
            } else {
                SpamAction::AddHeaders
            }
        } else if total_score > 0.0 {
            SpamAction::AddHeaders
        } else {
            SpamAction::Deliver
        };

        SpamResult {
            score: total_score,
            is_spam,
            rules_matched,
            action,
        }
    }

    /// Check a single rule
    fn check_rule(
        &self,
        rule: &SpamRule,
        from: &str,
        to: &str,
        subject: &str,
        body: &str,
        headers: &[(String, String)],
    ) -> Option<f64> {
        match rule.rule_type {
            SpamRuleType::Header => self.check_header_rule(rule, from, to, subject, headers),
            SpamRuleType::Body => self.check_body_rule(rule, body),
            SpamRuleType::Regex => self.check_regex_rule(rule, from, subject, body),
            _ => None,
        }
    }

    /// Check header-based rule
    fn check_header_rule(
        &self,
        rule: &SpamRule,
        from: &str,
        to: &str,
        subject: &str,
        headers: &[(String, String)],
    ) -> Option<f64> {
        let pattern = &rule.pattern.to_lowercase();

        // Check specific headers
        if pattern.starts_with("from:") {
            let check = pattern.strip_prefix("from:").unwrap();
            if from.to_lowercase().contains(check) {
                return Some(rule.score);
            }
        } else if pattern.starts_with("to:") {
            let check = pattern.strip_prefix("to:").unwrap();
            if to.to_lowercase().contains(check) {
                return Some(rule.score);
            }
        } else if pattern.starts_with("subject:") {
            let check = pattern.strip_prefix("subject:").unwrap();
            if subject.to_lowercase().contains(check) {
                return Some(rule.score);
            }
        } else {
            // Generic header check
            for (name, value) in headers {
                if name.to_lowercase().contains(pattern) || value.to_lowercase().contains(pattern) {
                    return Some(rule.score);
                }
            }
        }

        None
    }

    /// Check body content rule
    fn check_body_rule(&self, rule: &SpamRule, body: &str) -> Option<f64> {
        let pattern = &rule.pattern.to_lowercase();
        let body_lower = body.to_lowercase();

        // Count occurrences
        let count = body_lower.matches(pattern).count();
        if count > 0 {
            // More occurrences = higher score
            return Some(rule.score * (1.0 + (count as f64 - 1.0) * 0.5).min(3.0));
        }

        None
    }

    /// Check regex rule
    fn check_regex_rule(&self, rule: &SpamRule, from: &str, subject: &str, body: &str) -> Option<f64> {
        if let Ok(re) = Regex::new(&rule.pattern) {
            let combined = format!("{}\n{}\n{}", from, subject, body);
            if re.is_match(&combined) {
                return Some(rule.score);
            }
        }
        None
    }

    /// Learn from a spam message
    pub fn learn_spam(&mut self, body: &str) {
        self.bayesian.learn(body, true);
    }

    /// Learn from a ham (non-spam) message
    pub fn learn_ham(&mut self, body: &str) {
        self.bayesian.learn(body, false);
    }

    /// Default spam detection rules
    fn default_rules() -> Vec<SpamRule> {
        vec![
            // Subject-based rules
            SpamRule {
                id: "SUBJECT_FREE".to_string(),
                name: "SUBJECT_FREE".to_string(),
                description: "Subject contains 'FREE' in caps".to_string(),
                rule_type: SpamRuleType::Header,
                pattern: "subject:free".to_string(),
                score: 1.5,
                is_enabled: true,
            },
            SpamRule {
                id: "SUBJECT_URGENT".to_string(),
                name: "SUBJECT_URGENT".to_string(),
                description: "Subject contains urgent language".to_string(),
                rule_type: SpamRuleType::Header,
                pattern: "subject:urgent".to_string(),
                score: 1.0,
                is_enabled: true,
            },
            SpamRule {
                id: "SUBJECT_WINNER".to_string(),
                name: "SUBJECT_WINNER".to_string(),
                description: "Subject mentions winning".to_string(),
                rule_type: SpamRuleType::Header,
                pattern: "subject:winner".to_string(),
                score: 2.0,
                is_enabled: true,
            },
            SpamRule {
                id: "SUBJECT_LOTTERY".to_string(),
                name: "SUBJECT_LOTTERY".to_string(),
                description: "Subject mentions lottery".to_string(),
                rule_type: SpamRuleType::Header,
                pattern: "subject:lottery".to_string(),
                score: 3.0,
                is_enabled: true,
            },

            // Body content rules
            SpamRule {
                id: "BODY_UNSUBSCRIBE".to_string(),
                name: "BODY_UNSUBSCRIBE".to_string(),
                description: "Body contains unsubscribe link".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "unsubscribe".to_string(),
                score: 0.5,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_VIAGRA".to_string(),
                name: "BODY_VIAGRA".to_string(),
                description: "Body mentions pharmaceutical spam".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "viagra".to_string(),
                score: 5.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_CIALIS".to_string(),
                name: "BODY_CIALIS".to_string(),
                description: "Body mentions pharmaceutical spam".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "cialis".to_string(),
                score: 5.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_CLICK_HERE".to_string(),
                name: "BODY_CLICK_HERE".to_string(),
                description: "Body contains 'click here' spam pattern".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "click here".to_string(),
                score: 1.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_MILLION_DOLLARS".to_string(),
                name: "BODY_MILLION_DOLLARS".to_string(),
                description: "Body mentions large money amounts".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "million dollar".to_string(),
                score: 3.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_BANK_TRANSFER".to_string(),
                name: "BODY_BANK_TRANSFER".to_string(),
                description: "Body mentions bank transfers".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "bank transfer".to_string(),
                score: 2.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_NIGERIAN".to_string(),
                name: "BODY_NIGERIAN".to_string(),
                description: "Nigerian prince scam pattern".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "nigerian prince".to_string(),
                score: 5.0,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_ACT_NOW".to_string(),
                name: "BODY_ACT_NOW".to_string(),
                description: "Urgency pressure pattern".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "act now".to_string(),
                score: 1.5,
                is_enabled: true,
            },
            SpamRule {
                id: "BODY_LIMITED_TIME".to_string(),
                name: "BODY_LIMITED_TIME".to_string(),
                description: "Limited time offer pattern".to_string(),
                rule_type: SpamRuleType::Body,
                pattern: "limited time".to_string(),
                score: 1.0,
                is_enabled: true,
            },

            // Regex rules for more complex patterns
            SpamRule {
                id: "REGEX_ALL_CAPS_SUBJECT".to_string(),
                name: "REGEX_ALL_CAPS_SUBJECT".to_string(),
                description: "Subject line mostly uppercase".to_string(),
                rule_type: SpamRuleType::Regex,
                pattern: r"^[A-Z\s!?.,]{20,}$".to_string(),
                score: 2.0,
                is_enabled: true,
            },
            SpamRule {
                id: "REGEX_MONEY_AMOUNT".to_string(),
                name: "REGEX_MONEY_AMOUNT".to_string(),
                description: "Large money amount mentioned".to_string(),
                rule_type: SpamRuleType::Regex,
                pattern: r"\$\d{1,3}(,\d{3}){2,}".to_string(),
                score: 2.5,
                is_enabled: true,
            },
            SpamRule {
                id: "REGEX_EXCESSIVE_EXCLAMATION".to_string(),
                name: "REGEX_EXCESSIVE_EXCLAMATION".to_string(),
                description: "Excessive exclamation marks".to_string(),
                rule_type: SpamRuleType::Regex,
                pattern: r"!{3,}".to_string(),
                score: 1.5,
                is_enabled: true,
            },
        ]
    }
}

impl Default for SpamScorer {
    fn default() -> Self {
        Self::new(SpamConfig::default())
    }
}

/// Bayesian spam classifier
pub struct BayesianClassifier {
    spam_tokens: HashMap<String, u32>,
    ham_tokens: HashMap<String, u32>,
    spam_count: u32,
    ham_count: u32,
    stemmer: Stemmer,
}

impl BayesianClassifier {
    /// Create a new classifier
    pub fn new() -> Self {
        Self {
            spam_tokens: HashMap::new(),
            ham_tokens: HashMap::new(),
            spam_count: 0,
            ham_count: 0,
            stemmer: Stemmer::create(Algorithm::English),
        }
    }

    /// Learn from a message
    pub fn learn(&mut self, text: &str, is_spam: bool) {
        let tokens = self.tokenize(text);

        if is_spam {
            self.spam_count += 1;
            for token in tokens {
                *self.spam_tokens.entry(token).or_insert(0) += 1;
            }
        } else {
            self.ham_count += 1;
            for token in tokens {
                *self.ham_tokens.entry(token).or_insert(0) += 1;
            }
        }
    }

    /// Classify a message, returns score between -1 (ham) and +1 (spam)
    pub fn classify(&self, text: &str) -> f64 {
        if self.spam_count == 0 || self.ham_count == 0 {
            return 0.0; // Not enough training data
        }

        let tokens = self.tokenize(text);
        let mut spam_prob_sum = 0.0f64;
        let mut ham_prob_sum = 0.0f64;
        let mut count = 0;

        for token in &tokens {
            let spam_token_count = self.spam_tokens.get(token).copied().unwrap_or(0) as f64;
            let ham_token_count = self.ham_tokens.get(token).copied().unwrap_or(0) as f64;

            // Apply Laplace smoothing
            let p_spam = (spam_token_count + 1.0) / (self.spam_count as f64 + 2.0);
            let p_ham = (ham_token_count + 1.0) / (self.ham_count as f64 + 2.0);

            // Avoid log of zero
            if p_spam > 0.0 && p_ham > 0.0 {
                spam_prob_sum += p_spam.ln();
                ham_prob_sum += p_ham.ln();
                count += 1;
            }
        }

        if count == 0 {
            return 0.0;
        }

        // Normalize and return difference
        let avg_spam = spam_prob_sum / count as f64;
        let avg_ham = ham_prob_sum / count as f64;

        // Sigmoid-like function to normalize between -1 and 1
        let diff = avg_spam - avg_ham;
        (2.0 / (1.0 + (-diff).exp())) - 1.0
    }

    /// Tokenize text into stemmed words
    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 3 && s.len() <= 25)
            .map(|s| self.stemmer.stem(s).to_string())
            .collect()
    }

    /// Get spam token count
    pub fn spam_token_count(&self) -> usize {
        self.spam_tokens.len()
    }

    /// Get ham token count
    pub fn ham_token_count(&self) -> usize {
        self.ham_tokens.len()
    }

    /// Get training counts
    pub fn training_counts(&self) -> (u32, u32) {
        (self.spam_count, self.ham_count)
    }

    /// Load token data from database
    pub fn load_tokens(&mut self, tokens: Vec<(String, u32, u32)>) {
        for (token, spam_count, ham_count) in tokens {
            if spam_count > 0 {
                self.spam_tokens.insert(token.clone(), spam_count);
            }
            if ham_count > 0 {
                self.ham_tokens.insert(token, ham_count);
            }
        }
    }

    /// Get all tokens for persistence
    pub fn get_tokens(&self) -> Vec<(String, u32, u32)> {
        let mut result = Vec::new();

        for (token, &spam_count) in &self.spam_tokens {
            let ham_count = self.ham_tokens.get(token).copied().unwrap_or(0);
            result.push((token.clone(), spam_count, ham_count));
        }

        for (token, &ham_count) in &self.ham_tokens {
            if !self.spam_tokens.contains_key(token) {
                result.push((token.clone(), 0, ham_count));
            }
        }

        result
    }
}

impl Default for BayesianClassifier {
    fn default() -> Self {
        Self::new()
    }
}
