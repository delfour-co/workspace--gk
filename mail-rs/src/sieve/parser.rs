//! Sieve script parser
//!
//! Implements a simplified Sieve parser for common filtering rules.

use anyhow::{anyhow, Result};
use regex::Regex;

use super::types::*;

/// Token types for lexer
#[derive(Debug, Clone, PartialEq)]
enum Token {
    // Keywords
    If,
    Elsif,
    Else,
    Require,
    // Actions
    Keep,
    Discard,
    FileInto,
    Redirect,
    Stop,
    SetFlag,
    AddFlag,
    // Tests
    Header,
    Address,
    Size,
    Exists,
    AllOf,
    AnyOf,
    Not,
    True,
    False,
    // Match types
    Is,
    Contains,
    Matches,
    // Address parts
    LocalPart,
    Domain,
    All,
    // Size comparators
    Over,
    Under,
    // Comparators
    Comparator,
    // Punctuation
    OpenBrace,
    CloseBrace,
    OpenBracket,
    CloseBracket,
    OpenParen,
    CloseParen,
    Semicolon,
    Comma,
    Colon,
    // Values
    String(String),
    Number(u64),
    // Identifiers (for extensions)
    Identifier(String),
}

/// Sieve script parser
pub struct SieveParser {
    tokens: Vec<Token>,
    pos: usize,
}

impl SieveParser {
    /// Create a new parser
    pub fn new(script: &str) -> Result<Self> {
        let tokens = Self::tokenize(script)?;
        Ok(Self { tokens, pos: 0 })
    }

    /// Tokenize the input script
    fn tokenize(script: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut chars: Vec<char> = script.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            // Skip whitespace
            if c.is_whitespace() {
                i += 1;
                continue;
            }

            // Skip comments (# to end of line or /* ... */)
            if c == '#' {
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                continue;
            }
            if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                i += 2;
                while i + 1 < chars.len() && !(chars[i] == '*' && chars[i + 1] == '/') {
                    i += 1;
                }
                i += 2;
                continue;
            }

            // Punctuation
            match c {
                '{' => {
                    tokens.push(Token::OpenBrace);
                    i += 1;
                    continue;
                }
                '}' => {
                    tokens.push(Token::CloseBrace);
                    i += 1;
                    continue;
                }
                '[' => {
                    tokens.push(Token::OpenBracket);
                    i += 1;
                    continue;
                }
                ']' => {
                    tokens.push(Token::CloseBracket);
                    i += 1;
                    continue;
                }
                '(' => {
                    tokens.push(Token::OpenParen);
                    i += 1;
                    continue;
                }
                ')' => {
                    tokens.push(Token::CloseParen);
                    i += 1;
                    continue;
                }
                ';' => {
                    tokens.push(Token::Semicolon);
                    i += 1;
                    continue;
                }
                ',' => {
                    tokens.push(Token::Comma);
                    i += 1;
                    continue;
                }
                _ => {}
            }

            // Quoted string
            if c == '"' {
                let mut s = String::new();
                i += 1;
                while i < chars.len() && chars[i] != '"' {
                    if chars[i] == '\\' && i + 1 < chars.len() {
                        i += 1;
                        s.push(chars[i]);
                    } else {
                        s.push(chars[i]);
                    }
                    i += 1;
                }
                i += 1; // Skip closing quote
                tokens.push(Token::String(s));
                continue;
            }

            // Multi-line string (text:)
            if c == 't' && script[i..].starts_with("text:") {
                i += 5;
                // Skip to end of line
                while i < chars.len() && chars[i] != '\n' {
                    i += 1;
                }
                i += 1;
                // Read until lone .
                let mut s = String::new();
                while i < chars.len() {
                    if chars[i] == '\n' {
                        // Check if next line starts with .
                        if i + 1 < chars.len() && chars[i + 1] == '.' {
                            if i + 2 >= chars.len() || chars[i + 2] == '\n' {
                                i += 2;
                                break;
                            }
                        }
                    }
                    s.push(chars[i]);
                    i += 1;
                }
                tokens.push(Token::String(s));
                continue;
            }

            // Number (with optional K/M/G suffix)
            if c.is_ascii_digit() {
                let mut num_str = String::new();
                while i < chars.len() && chars[i].is_ascii_digit() {
                    num_str.push(chars[i]);
                    i += 1;
                }
                let mut num: u64 = num_str.parse().unwrap_or(0);
                if i < chars.len() {
                    match chars[i].to_ascii_uppercase() {
                        'K' => {
                            num *= 1024;
                            i += 1;
                        }
                        'M' => {
                            num *= 1024 * 1024;
                            i += 1;
                        }
                        'G' => {
                            num *= 1024 * 1024 * 1024;
                            i += 1;
                        }
                        _ => {}
                    }
                }
                tokens.push(Token::Number(num));
                continue;
            }

            // Identifier or keyword
            if c.is_ascii_alphabetic() || c == '_' || c == ':' {
                let mut ident = String::new();
                while i < chars.len()
                    && (chars[i].is_ascii_alphanumeric() || chars[i] == '_' || chars[i] == ':')
                {
                    ident.push(chars[i]);
                    i += 1;
                }

                let token = match ident.to_lowercase().as_str() {
                    "if" => Token::If,
                    "elsif" => Token::Elsif,
                    "else" => Token::Else,
                    "require" => Token::Require,
                    "keep" => Token::Keep,
                    "discard" => Token::Discard,
                    "fileinto" => Token::FileInto,
                    "redirect" => Token::Redirect,
                    "stop" => Token::Stop,
                    "setflag" => Token::SetFlag,
                    "addflag" => Token::AddFlag,
                    "header" => Token::Header,
                    "address" => Token::Address,
                    "size" => Token::Size,
                    "exists" => Token::Exists,
                    "allof" => Token::AllOf,
                    "anyof" => Token::AnyOf,
                    "not" => Token::Not,
                    "true" => Token::True,
                    "false" => Token::False,
                    ":is" => Token::Is,
                    ":contains" => Token::Contains,
                    ":matches" => Token::Matches,
                    ":localpart" => Token::LocalPart,
                    ":domain" => Token::Domain,
                    ":all" => Token::All,
                    ":over" => Token::Over,
                    ":under" => Token::Under,
                    ":comparator" => Token::Comparator,
                    _ => Token::Identifier(ident),
                };
                tokens.push(token);
                continue;
            }

            // Unknown character
            i += 1;
        }

        Ok(tokens)
    }

    /// Parse the script
    pub fn parse(&mut self) -> Result<Vec<SieveRule>> {
        let mut rules = Vec::new();

        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos] {
                Token::Require => {
                    self.parse_require()?;
                }
                Token::If => {
                    rules.push(self.parse_if_statement()?);
                }
                // Top-level actions
                Token::Keep
                | Token::Discard
                | Token::FileInto
                | Token::Redirect
                | Token::Stop => {
                    let action = self.parse_action()?;
                    rules.push(SieveRule {
                        condition: SieveCondition::True,
                        actions: vec![action],
                        elsif_branches: vec![],
                        else_actions: None,
                    });
                }
                _ => {
                    self.pos += 1;
                }
            }
        }

        Ok(rules)
    }

    /// Parse require statement
    fn parse_require(&mut self) -> Result<()> {
        self.expect(Token::Require)?;
        // Skip the required capabilities
        self.parse_string_list()?;
        self.expect(Token::Semicolon)?;
        Ok(())
    }

    /// Parse if statement
    fn parse_if_statement(&mut self) -> Result<SieveRule> {
        self.expect(Token::If)?;
        let condition = self.parse_condition()?;
        let actions = self.parse_block()?;

        let mut elsif_branches = vec![];
        let mut else_actions = None;

        // Parse elsif branches
        while self.pos < self.tokens.len() && self.tokens[self.pos] == Token::Elsif {
            self.pos += 1;
            let elsif_cond = self.parse_condition()?;
            let elsif_actions = self.parse_block()?;
            elsif_branches.push((elsif_cond, elsif_actions));
        }

        // Parse else
        if self.pos < self.tokens.len() && self.tokens[self.pos] == Token::Else {
            self.pos += 1;
            else_actions = Some(self.parse_block()?);
        }

        Ok(SieveRule {
            condition,
            actions,
            elsif_branches,
            else_actions,
        })
    }

    /// Parse a condition
    fn parse_condition(&mut self) -> Result<SieveCondition> {
        if self.pos >= self.tokens.len() {
            return Err(anyhow!("Unexpected end of script"));
        }

        match &self.tokens[self.pos] {
            Token::True => {
                self.pos += 1;
                Ok(SieveCondition::True)
            }
            Token::False => {
                self.pos += 1;
                Ok(SieveCondition::False)
            }
            Token::Not => {
                self.pos += 1;
                let inner = self.parse_condition()?;
                Ok(SieveCondition::Not(Box::new(inner)))
            }
            Token::AllOf => {
                self.pos += 1;
                self.expect(Token::OpenParen)?;
                let conditions = self.parse_condition_list()?;
                self.expect(Token::CloseParen)?;
                Ok(SieveCondition::AllOf(conditions))
            }
            Token::AnyOf => {
                self.pos += 1;
                self.expect(Token::OpenParen)?;
                let conditions = self.parse_condition_list()?;
                self.expect(Token::CloseParen)?;
                Ok(SieveCondition::AnyOf(conditions))
            }
            Token::Header => {
                self.parse_header_test()
            }
            Token::Address => {
                self.parse_address_test()
            }
            Token::Size => {
                self.parse_size_test()
            }
            Token::Exists => {
                self.pos += 1;
                let headers = self.parse_string_list()?;
                Ok(SieveCondition::Exists(headers))
            }
            _ => Err(anyhow!("Expected condition")),
        }
    }

    /// Parse header test
    fn parse_header_test(&mut self) -> Result<SieveCondition> {
        self.expect(Token::Header)?;

        let mut match_type = MatchType::Is;
        let mut comparator = Comparator::AsciiCasemap;

        // Parse optional modifiers
        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos] {
                Token::Is => {
                    match_type = MatchType::Is;
                    self.pos += 1;
                }
                Token::Contains => {
                    match_type = MatchType::Contains;
                    self.pos += 1;
                }
                Token::Matches => {
                    match_type = MatchType::Matches;
                    self.pos += 1;
                }
                Token::Comparator => {
                    self.pos += 1;
                    if let Token::String(s) = &self.tokens[self.pos] {
                        comparator = match s.as_str() {
                            "i;octet" => Comparator::Octet,
                            _ => Comparator::AsciiCasemap,
                        };
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }

        let headers = self.parse_string_list()?;
        let values = self.parse_string_list()?;

        Ok(SieveCondition::Header(HeaderTest {
            headers,
            values,
            match_type,
            comparator,
        }))
    }

    /// Parse address test
    fn parse_address_test(&mut self) -> Result<SieveCondition> {
        self.expect(Token::Address)?;

        let mut match_type = MatchType::Is;
        let mut comparator = Comparator::AsciiCasemap;
        let mut address_part = AddressPart::All;

        // Parse optional modifiers
        while self.pos < self.tokens.len() {
            match &self.tokens[self.pos] {
                Token::Is => {
                    match_type = MatchType::Is;
                    self.pos += 1;
                }
                Token::Contains => {
                    match_type = MatchType::Contains;
                    self.pos += 1;
                }
                Token::Matches => {
                    match_type = MatchType::Matches;
                    self.pos += 1;
                }
                Token::LocalPart => {
                    address_part = AddressPart::LocalPart;
                    self.pos += 1;
                }
                Token::Domain => {
                    address_part = AddressPart::Domain;
                    self.pos += 1;
                }
                Token::All => {
                    address_part = AddressPart::All;
                    self.pos += 1;
                }
                Token::Comparator => {
                    self.pos += 1;
                    if let Token::String(s) = &self.tokens[self.pos] {
                        comparator = match s.as_str() {
                            "i;octet" => Comparator::Octet,
                            _ => Comparator::AsciiCasemap,
                        };
                        self.pos += 1;
                    }
                }
                _ => break,
            }
        }

        let headers = self.parse_string_list()?;
        let values = self.parse_string_list()?;

        Ok(SieveCondition::Address(AddressTest {
            headers,
            values,
            match_type,
            address_part,
            comparator,
        }))
    }

    /// Parse size test
    fn parse_size_test(&mut self) -> Result<SieveCondition> {
        self.expect(Token::Size)?;

        let comparison = match &self.tokens[self.pos] {
            Token::Over => {
                self.pos += 1;
                SizeComparison::Over
            }
            Token::Under => {
                self.pos += 1;
                SizeComparison::Under
            }
            _ => return Err(anyhow!("Expected :over or :under")),
        };

        let size = match &self.tokens[self.pos] {
            Token::Number(n) => {
                let n = *n;
                self.pos += 1;
                n
            }
            _ => return Err(anyhow!("Expected size")),
        };

        Ok(SieveCondition::Size(SizeTest { comparison, size }))
    }

    /// Parse condition list
    fn parse_condition_list(&mut self) -> Result<Vec<SieveCondition>> {
        let mut conditions = vec![self.parse_condition()?];

        while self.pos < self.tokens.len() && self.tokens[self.pos] == Token::Comma {
            self.pos += 1;
            conditions.push(self.parse_condition()?);
        }

        Ok(conditions)
    }

    /// Parse action block
    fn parse_block(&mut self) -> Result<Vec<SieveAction>> {
        self.expect(Token::OpenBrace)?;

        let mut actions = vec![];
        while self.pos < self.tokens.len() && self.tokens[self.pos] != Token::CloseBrace {
            actions.push(self.parse_action()?);
        }

        self.expect(Token::CloseBrace)?;
        Ok(actions)
    }

    /// Parse an action
    fn parse_action(&mut self) -> Result<SieveAction> {
        if self.pos >= self.tokens.len() {
            return Err(anyhow!("Unexpected end of script"));
        }

        let action = match &self.tokens[self.pos] {
            Token::Keep => {
                self.pos += 1;
                self.expect(Token::Semicolon)?;
                SieveAction::Keep
            }
            Token::Discard => {
                self.pos += 1;
                self.expect(Token::Semicolon)?;
                SieveAction::Discard
            }
            Token::Stop => {
                self.pos += 1;
                self.expect(Token::Semicolon)?;
                SieveAction::Stop
            }
            Token::FileInto => {
                self.pos += 1;
                let folder = self.parse_string()?;
                self.expect(Token::Semicolon)?;
                SieveAction::FileInto(folder)
            }
            Token::Redirect => {
                self.pos += 1;
                let address = self.parse_string()?;
                self.expect(Token::Semicolon)?;
                SieveAction::Redirect(address)
            }
            Token::SetFlag | Token::AddFlag => {
                self.pos += 1;
                let flags = self.parse_string_list()?;
                self.expect(Token::Semicolon)?;
                SieveAction::Flag(flags)
            }
            _ => return Err(anyhow!("Expected action")),
        };

        Ok(action)
    }

    /// Parse string list or single string
    fn parse_string_list(&mut self) -> Result<Vec<String>> {
        if self.pos >= self.tokens.len() {
            return Err(anyhow!("Unexpected end of script"));
        }

        if self.tokens[self.pos] == Token::OpenBracket {
            self.pos += 1;
            let mut strings = vec![];

            while self.pos < self.tokens.len() && self.tokens[self.pos] != Token::CloseBracket {
                if let Token::String(s) = &self.tokens[self.pos] {
                    strings.push(s.clone());
                    self.pos += 1;
                } else {
                    return Err(anyhow!("Expected string"));
                }

                if self.pos < self.tokens.len() && self.tokens[self.pos] == Token::Comma {
                    self.pos += 1;
                }
            }

            self.expect(Token::CloseBracket)?;
            Ok(strings)
        } else if let Token::String(s) = &self.tokens[self.pos] {
            let s = s.clone();
            self.pos += 1;
            Ok(vec![s])
        } else {
            Err(anyhow!("Expected string or string list"))
        }
    }

    /// Parse single string
    fn parse_string(&mut self) -> Result<String> {
        if let Token::String(s) = &self.tokens[self.pos] {
            let s = s.clone();
            self.pos += 1;
            Ok(s)
        } else {
            Err(anyhow!("Expected string"))
        }
    }

    /// Expect a specific token
    fn expect(&mut self, expected: Token) -> Result<()> {
        if self.pos >= self.tokens.len() {
            return Err(anyhow!("Unexpected end of script, expected {:?}", expected));
        }
        if std::mem::discriminant(&self.tokens[self.pos]) != std::mem::discriminant(&expected) {
            return Err(anyhow!(
                "Expected {:?}, got {:?}",
                expected,
                self.tokens[self.pos]
            ));
        }
        self.pos += 1;
        Ok(())
    }
}

/// Parse a Sieve script
pub fn parse_script(script: &str) -> Result<Vec<SieveRule>> {
    let mut parser = SieveParser::new(script)?;
    parser.parse()
}

/// Validate a Sieve script
pub fn validate_script(script: &str) -> Result<ValidationResult> {
    match parse_script(script) {
        Ok(_) => Ok(ValidationResult {
            valid: true,
            error: None,
            warnings: vec![],
        }),
        Err(e) => Ok(ValidationResult {
            valid: false,
            error: Some(e.to_string()),
            warnings: vec![],
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_rule() {
        let script = r#"
            require "fileinto";
            if header :contains "Subject" "newsletter" {
                fileinto "Newsletters";
            }
        "#;

        let rules = parse_script(script).unwrap();
        assert_eq!(rules.len(), 1);
    }

    #[test]
    fn test_parse_multiple_rules() {
        let script = r#"
            if header :is "From" "spam@example.com" {
                discard;
            }
            if size :over 10M {
                fileinto "Large";
            }
        "#;

        let rules = parse_script(script).unwrap();
        assert_eq!(rules.len(), 2);
    }

    #[test]
    fn test_parse_allof() {
        let script = r#"
            if allof (header :contains "From" "boss", header :contains "Subject" "urgent") {
                keep;
            }
        "#;

        let rules = parse_script(script).unwrap();
        assert_eq!(rules.len(), 1);
        if let SieveCondition::AllOf(conditions) = &rules[0].condition {
            assert_eq!(conditions.len(), 2);
        } else {
            panic!("Expected AllOf condition");
        }
    }
}
