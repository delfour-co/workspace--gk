use crate::error::{MailError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum SmtpCommand {
    Helo(String),
    Ehlo(String),
    MailFrom(String),
    RcptTo(String),
    Data,
    Rset,
    Quit,
    Noop,
    Unknown(String),
}

impl SmtpCommand {
    pub fn parse(line: &str) -> Result<Self> {
        let line = line.trim();
        if line.is_empty() {
            return Err(MailError::SmtpProtocol("Empty command".to_string()));
        }

        let parts: Vec<&str> = line.splitn(2, ' ').collect();
        let command = parts[0].to_uppercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match command.as_str() {
            "HELO" => {
                if args.is_empty() {
                    return Err(MailError::SmtpProtocol("HELO requires domain".to_string()));
                }
                Ok(SmtpCommand::Helo(args.to_string()))
            }
            "EHLO" => {
                if args.is_empty() {
                    return Err(MailError::SmtpProtocol("EHLO requires domain".to_string()));
                }
                Ok(SmtpCommand::Ehlo(args.to_string()))
            }
            "MAIL" => {
                // Parse MAIL FROM:<address>
                let from = Self::parse_mail_from(args)?;
                Ok(SmtpCommand::MailFrom(from))
            }
            "RCPT" => {
                // Parse RCPT TO:<address>
                let to = Self::parse_rcpt_to(args)?;
                Ok(SmtpCommand::RcptTo(to))
            }
            "DATA" => Ok(SmtpCommand::Data),
            "RSET" => Ok(SmtpCommand::Rset),
            "QUIT" => Ok(SmtpCommand::Quit),
            "NOOP" => Ok(SmtpCommand::Noop),
            _ => Ok(SmtpCommand::Unknown(command)),
        }
    }

    fn parse_mail_from(args: &str) -> Result<String> {
        // Expected format: FROM:<email@domain.com>
        if !args.to_uppercase().starts_with("FROM:") {
            return Err(MailError::SmtpProtocol("Invalid MAIL FROM syntax".to_string()));
        }

        let email = args[5..].trim();
        let email = if email.starts_with('<') && email.ends_with('>') {
            &email[1..email.len() - 1]
        } else {
            email
        };

        Ok(email.to_string())
    }

    fn parse_rcpt_to(args: &str) -> Result<String> {
        // Expected format: TO:<email@domain.com>
        if !args.to_uppercase().starts_with("TO:") {
            return Err(MailError::SmtpProtocol("Invalid RCPT TO syntax".to_string()));
        }

        let email = args[3..].trim();
        let email = if email.starts_with('<') && email.ends_with('>') {
            &email[1..email.len() - 1]
        } else {
            email
        };

        Ok(email.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_helo() {
        let cmd = SmtpCommand::parse("HELO example.com").unwrap();
        assert_eq!(cmd, SmtpCommand::Helo("example.com".to_string()));
    }

    #[test]
    fn test_parse_ehlo() {
        let cmd = SmtpCommand::parse("EHLO example.com").unwrap();
        assert_eq!(cmd, SmtpCommand::Ehlo("example.com".to_string()));
    }

    #[test]
    fn test_parse_mail_from() {
        let cmd = SmtpCommand::parse("MAIL FROM:<sender@example.com>").unwrap();
        assert_eq!(cmd, SmtpCommand::MailFrom("sender@example.com".to_string()));
    }

    #[test]
    fn test_parse_rcpt_to() {
        let cmd = SmtpCommand::parse("RCPT TO:<recipient@example.com>").unwrap();
        assert_eq!(cmd, SmtpCommand::RcptTo("recipient@example.com".to_string()));
    }

    #[test]
    fn test_parse_data() {
        let cmd = SmtpCommand::parse("DATA").unwrap();
        assert_eq!(cmd, SmtpCommand::Data);
    }

    #[test]
    fn test_parse_quit() {
        let cmd = SmtpCommand::parse("QUIT").unwrap();
        assert_eq!(cmd, SmtpCommand::Quit);
    }
}
