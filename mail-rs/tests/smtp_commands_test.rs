use mail_rs::smtp::SmtpCommand;

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
fn test_parse_helo_case_insensitive() {
    let cmd = SmtpCommand::parse("helo example.com").unwrap();
    assert_eq!(cmd, SmtpCommand::Helo("example.com".to_string()));
}

#[test]
fn test_parse_mail_from() {
    let cmd = SmtpCommand::parse("MAIL FROM:<sender@example.com>").unwrap();
    assert_eq!(cmd, SmtpCommand::MailFrom("sender@example.com".to_string()));
}

#[test]
fn test_parse_mail_from_no_brackets() {
    let cmd = SmtpCommand::parse("MAIL FROM:sender@example.com").unwrap();
    assert_eq!(cmd, SmtpCommand::MailFrom("sender@example.com".to_string()));
}

#[test]
fn test_parse_rcpt_to() {
    let cmd = SmtpCommand::parse("RCPT TO:<recipient@example.com>").unwrap();
    assert_eq!(
        cmd,
        SmtpCommand::RcptTo("recipient@example.com".to_string())
    );
}

#[test]
fn test_parse_rcpt_to_no_brackets() {
    let cmd = SmtpCommand::parse("RCPT TO:recipient@example.com").unwrap();
    assert_eq!(
        cmd,
        SmtpCommand::RcptTo("recipient@example.com".to_string())
    );
}

#[test]
fn test_parse_data() {
    let cmd = SmtpCommand::parse("DATA").unwrap();
    assert_eq!(cmd, SmtpCommand::Data);
}

#[test]
fn test_parse_rset() {
    let cmd = SmtpCommand::parse("RSET").unwrap();
    assert_eq!(cmd, SmtpCommand::Rset);
}

#[test]
fn test_parse_quit() {
    let cmd = SmtpCommand::parse("QUIT").unwrap();
    assert_eq!(cmd, SmtpCommand::Quit);
}

#[test]
fn test_parse_noop() {
    let cmd = SmtpCommand::parse("NOOP").unwrap();
    assert_eq!(cmd, SmtpCommand::Noop);
}

#[test]
fn test_parse_unknown_command() {
    let cmd = SmtpCommand::parse("INVALID").unwrap();
    assert_eq!(cmd, SmtpCommand::Unknown("INVALID".to_string()));
}

#[test]
fn test_parse_empty_command() {
    assert!(SmtpCommand::parse("").is_err());
}

#[test]
fn test_parse_helo_missing_domain() {
    assert!(SmtpCommand::parse("HELO").is_err());
}

#[test]
fn test_parse_ehlo_missing_domain() {
    assert!(SmtpCommand::parse("EHLO").is_err());
}

#[test]
fn test_parse_mail_from_invalid() {
    assert!(SmtpCommand::parse("MAIL invalid").is_err());
}

#[test]
fn test_parse_rcpt_to_invalid() {
    assert!(SmtpCommand::parse("RCPT invalid").is_err());
}
