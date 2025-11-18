use thiserror::Error;

#[derive(Error, Debug)]
pub enum MailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SMTP protocol error: {0}")]
    SmtpProtocol(String),

    #[error("IMAP protocol error: {0}")]
    ImapProtocol(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Invalid email address: {0}")]
    InvalidEmail(String),

    #[error("DNS lookup failed: {0}")]
    DnsLookup(String),

    #[error("TLS error: {0}")]
    Tls(String),
}

pub type Result<T> = std::result::Result<T, MailError>;
