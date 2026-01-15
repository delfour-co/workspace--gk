//! Sieve script executor
//!
//! Evaluates conditions and executes actions on messages.

use anyhow::Result;
use regex::Regex;

use super::types::*;

/// Sieve rule executor
pub struct SieveExecutor;

impl SieveExecutor {
    /// Execute a list of rules on a message
    pub fn execute(rules: &[SieveRule], message: &MessageContext) -> Result<SieveResult> {
        let mut result = SieveResult::default();

        for rule in rules {
            let matched = Self::evaluate_condition(&rule.condition, message)?;

            if matched {
                // Execute main actions
                for action in &rule.actions {
                    result.actions.push(action.clone());
                    if *action == SieveAction::Stop {
                        result.implicit_keep = false;
                        return Ok(result);
                    }
                    if *action == SieveAction::Discard {
                        result.implicit_keep = false;
                    }
                    if *action == SieveAction::Keep {
                        result.implicit_keep = false;
                    }
                }
            } else {
                // Check elsif branches
                let mut elsif_matched = false;
                for (elsif_cond, elsif_actions) in &rule.elsif_branches {
                    if Self::evaluate_condition(elsif_cond, message)? {
                        elsif_matched = true;
                        for action in elsif_actions {
                            result.actions.push(action.clone());
                            if *action == SieveAction::Stop {
                                result.implicit_keep = false;
                                return Ok(result);
                            }
                            if *action == SieveAction::Discard {
                                result.implicit_keep = false;
                            }
                            if *action == SieveAction::Keep {
                                result.implicit_keep = false;
                            }
                        }
                        break;
                    }
                }

                // Check else
                if !elsif_matched {
                    if let Some(else_actions) = &rule.else_actions {
                        for action in else_actions {
                            result.actions.push(action.clone());
                            if *action == SieveAction::Stop {
                                result.implicit_keep = false;
                                return Ok(result);
                            }
                            if *action == SieveAction::Discard {
                                result.implicit_keep = false;
                            }
                            if *action == SieveAction::Keep {
                                result.implicit_keep = false;
                            }
                        }
                    }
                }
            }
        }

        Ok(result)
    }

    /// Evaluate a condition against a message
    fn evaluate_condition(condition: &SieveCondition, message: &MessageContext) -> Result<bool> {
        match condition {
            SieveCondition::True => Ok(true),
            SieveCondition::False => Ok(false),
            SieveCondition::Not(inner) => Ok(!Self::evaluate_condition(inner, message)?),
            SieveCondition::AllOf(conditions) => {
                for cond in conditions {
                    if !Self::evaluate_condition(cond, message)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            SieveCondition::AnyOf(conditions) => {
                for cond in conditions {
                    if Self::evaluate_condition(cond, message)? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            SieveCondition::Header(test) => Self::evaluate_header_test(test, message),
            SieveCondition::Address(test) => Self::evaluate_address_test(test, message),
            SieveCondition::Size(test) => Self::evaluate_size_test(test, message),
            SieveCondition::Exists(headers) => Self::evaluate_exists_test(headers, message),
        }
    }

    /// Evaluate a header test
    fn evaluate_header_test(test: &HeaderTest, message: &MessageContext) -> Result<bool> {
        for header_name in &test.headers {
            let header_name_lower = header_name.to_lowercase();

            // Find matching headers
            for (name, value) in &message.headers {
                if name.to_lowercase() != header_name_lower {
                    continue;
                }

                // Test against each value
                for test_value in &test.values {
                    if Self::string_match(value, test_value, &test.match_type, &test.comparator) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Evaluate an address test
    fn evaluate_address_test(test: &AddressTest, message: &MessageContext) -> Result<bool> {
        for header_name in &test.headers {
            let header_name_lower = header_name.to_lowercase();

            // Get addresses from appropriate header
            let addresses: Vec<&str> = match header_name_lower.as_str() {
                "from" => vec![message.from.as_str()],
                "to" => message.to.iter().map(|s| s.as_str()).collect(),
                "cc" => message.cc.iter().map(|s| s.as_str()).collect(),
                _ => {
                    // Look up in headers
                    message
                        .headers
                        .iter()
                        .filter(|(n, _)| n.to_lowercase() == header_name_lower)
                        .map(|(_, v)| v.as_str())
                        .collect()
                }
            };

            for address in addresses {
                let test_part = Self::extract_address_part(address, &test.address_part);

                for test_value in &test.values {
                    if Self::string_match(&test_part, test_value, &test.match_type, &test.comparator)
                    {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Extract part of an email address
    fn extract_address_part(address: &str, part: &AddressPart) -> String {
        // Remove any display name (e.g., "John Doe <john@example.com>" -> "john@example.com")
        let email = if let Some(start) = address.find('<') {
            if let Some(end) = address.find('>') {
                &address[start + 1..end]
            } else {
                address
            }
        } else {
            address
        }
        .trim();

        match part {
            AddressPart::All => email.to_string(),
            AddressPart::LocalPart => {
                if let Some(at) = email.find('@') {
                    email[..at].to_string()
                } else {
                    email.to_string()
                }
            }
            AddressPart::Domain => {
                if let Some(at) = email.find('@') {
                    email[at + 1..].to_string()
                } else {
                    String::new()
                }
            }
        }
    }

    /// Evaluate a size test
    fn evaluate_size_test(test: &SizeTest, message: &MessageContext) -> Result<bool> {
        match test.comparison {
            SizeComparison::Over => Ok(message.size > test.size),
            SizeComparison::Under => Ok(message.size < test.size),
        }
    }

    /// Evaluate an exists test
    fn evaluate_exists_test(headers: &[String], message: &MessageContext) -> Result<bool> {
        for header_name in headers {
            let header_name_lower = header_name.to_lowercase();
            let exists = message
                .headers
                .iter()
                .any(|(name, _)| name.to_lowercase() == header_name_lower);

            if !exists {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Match strings according to match type
    fn string_match(
        value: &str,
        pattern: &str,
        match_type: &MatchType,
        comparator: &Comparator,
    ) -> bool {
        let (value, pattern) = match comparator {
            Comparator::AsciiCasemap => (value.to_lowercase(), pattern.to_lowercase()),
            Comparator::Octet => (value.to_string(), pattern.to_string()),
        };

        match match_type {
            MatchType::Is => value == pattern,
            MatchType::Contains => value.contains(&pattern),
            MatchType::Matches => Self::wildcard_match(&value, &pattern),
            MatchType::Regex => {
                if let Ok(re) = Regex::new(&pattern) {
                    re.is_match(&value)
                } else {
                    false
                }
            }
        }
    }

    /// Wildcard match (* and ?)
    fn wildcard_match(value: &str, pattern: &str) -> bool {
        // Convert Sieve wildcards to regex
        let mut regex_pattern = String::from("^");
        for c in pattern.chars() {
            match c {
                '*' => regex_pattern.push_str(".*"),
                '?' => regex_pattern.push('.'),
                '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                    regex_pattern.push('\\');
                    regex_pattern.push(c);
                }
                _ => regex_pattern.push(c),
            }
        }
        regex_pattern.push('$');

        if let Ok(re) = Regex::new(&regex_pattern) {
            re.is_match(value)
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_message() -> MessageContext {
        MessageContext {
            from: "sender@example.com".to_string(),
            to: vec!["recipient@example.com".to_string()],
            cc: vec![],
            subject: "Test Newsletter".to_string(),
            headers: vec![
                ("From".to_string(), "sender@example.com".to_string()),
                ("To".to_string(), "recipient@example.com".to_string()),
                ("Subject".to_string(), "Test Newsletter".to_string()),
                (
                    "X-Custom".to_string(),
                    "custom value".to_string(),
                ),
            ],
            body: "This is a test message.".to_string(),
            size: 1024,
        }
    }

    #[test]
    fn test_header_contains() {
        let message = create_test_message();
        let condition = SieveCondition::Header(HeaderTest {
            headers: vec!["Subject".to_string()],
            values: vec!["Newsletter".to_string()],
            match_type: MatchType::Contains,
            comparator: Comparator::AsciiCasemap,
        });

        assert!(SieveExecutor::evaluate_condition(&condition, &message).unwrap());
    }

    #[test]
    fn test_header_is() {
        let message = create_test_message();
        let condition = SieveCondition::Header(HeaderTest {
            headers: vec!["From".to_string()],
            values: vec!["sender@example.com".to_string()],
            match_type: MatchType::Is,
            comparator: Comparator::AsciiCasemap,
        });

        assert!(SieveExecutor::evaluate_condition(&condition, &message).unwrap());
    }

    #[test]
    fn test_address_domain() {
        let message = create_test_message();
        let condition = SieveCondition::Address(AddressTest {
            headers: vec!["From".to_string()],
            values: vec!["example.com".to_string()],
            match_type: MatchType::Is,
            address_part: AddressPart::Domain,
            comparator: Comparator::AsciiCasemap,
        });

        assert!(SieveExecutor::evaluate_condition(&condition, &message).unwrap());
    }

    #[test]
    fn test_size_over() {
        let message = create_test_message();
        let condition = SieveCondition::Size(SizeTest {
            comparison: SizeComparison::Over,
            size: 500,
        });

        assert!(SieveExecutor::evaluate_condition(&condition, &message).unwrap());
    }

    #[test]
    fn test_size_under() {
        let message = create_test_message();
        let condition = SieveCondition::Size(SizeTest {
            comparison: SizeComparison::Under,
            size: 500,
        });

        assert!(!SieveExecutor::evaluate_condition(&condition, &message).unwrap());
    }

    #[test]
    fn test_wildcard_match() {
        assert!(SieveExecutor::wildcard_match("hello world", "*world"));
        assert!(SieveExecutor::wildcard_match("hello world", "hello*"));
        assert!(SieveExecutor::wildcard_match("hello world", "*lo wo*"));
        assert!(SieveExecutor::wildcard_match("test", "t?st"));
        assert!(!SieveExecutor::wildcard_match("test", "t?t"));
    }
}
