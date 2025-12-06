/// Admin module for Mail-in-a-Box equivalent functionality
///
/// Provides:
/// - DNS auto-configuration
/// - System diagnostics and monitoring
/// - Backup management
/// - SSL certificate automation (Let's Encrypt)

pub mod backup;
pub mod diagnostics;
pub mod dns;
pub mod ssl;

pub use backup::{BackupManager, BackupConfig, BackupStatus};
pub use diagnostics::{SystemDiagnostics, DiagnosticResult, HealthStatus};
pub use dns::{DnsConfigGenerator, DnsRecord, DnsRecordType};
pub use ssl::{SslManager, SslConfig, CertificateStatus};
