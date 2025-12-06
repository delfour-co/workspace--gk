use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

/// DNS record types for email server
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DnsRecordType {
    /// A record (IPv4 address)
    A,
    /// AAAA record (IPv6 address)
    AAAA,
    /// MX record (mail exchange)
    MX,
    /// TXT record (SPF, DKIM, DMARC)
    TXT,
    /// CNAME record
    CNAME,
    /// PTR record (reverse DNS)
    PTR,
}

impl std::fmt::Display for DnsRecordType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DnsRecordType::A => write!(f, "A"),
            DnsRecordType::AAAA => write!(f, "AAAA"),
            DnsRecordType::MX => write!(f, "MX"),
            DnsRecordType::TXT => write!(f, "TXT"),
            DnsRecordType::CNAME => write!(f, "CNAME"),
            DnsRecordType::PTR => write!(f, "PTR"),
        }
    }
}

/// DNS record representation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    /// Record type
    pub record_type: DnsRecordType,
    /// Hostname/domain
    pub name: String,
    /// Record value
    pub value: String,
    /// TTL in seconds
    pub ttl: u32,
    /// Priority (for MX records)
    pub priority: Option<u16>,
    /// Description/purpose
    pub description: String,
}

impl DnsRecord {
    /// Create new DNS record
    pub fn new(
        record_type: DnsRecordType,
        name: String,
        value: String,
        ttl: u32,
        description: String,
    ) -> Self {
        DnsRecord {
            record_type,
            name,
            value,
            ttl,
            priority: None,
            description,
        }
    }

    /// Create MX record with priority
    pub fn mx(name: String, value: String, priority: u16, ttl: u32) -> Self {
        DnsRecord {
            record_type: DnsRecordType::MX,
            name,
            value,
            ttl,
            priority: Some(priority),
            description: "Mail server MX record".to_string(),
        }
    }

    /// Format as zone file line
    pub fn to_zone_line(&self) -> String {
        match self.record_type {
            DnsRecordType::MX => {
                format!(
                    "{}\t{}\tIN\t{}\t{} {}",
                    self.name,
                    self.ttl,
                    self.record_type,
                    self.priority.unwrap_or(10),
                    self.value
                )
            }
            _ => {
                format!(
                    "{}\t{}\tIN\t{}\t{}",
                    self.name, self.ttl, self.record_type, self.value
                )
            }
        }
    }
}

/// DNS configuration generator
pub struct DnsConfigGenerator {
    domain: String,
    mail_server_hostname: String,
    server_ip: IpAddr,
    dkim_selector: String,
    dkim_public_key: Option<String>,
}

impl DnsConfigGenerator {
    /// Create new DNS config generator
    pub fn new(
        domain: String,
        mail_server_hostname: String,
        server_ip: IpAddr,
        dkim_selector: String,
    ) -> Self {
        DnsConfigGenerator {
            domain,
            mail_server_hostname,
            server_ip,
            dkim_selector,
            dkim_public_key: None,
        }
    }

    /// Set DKIM public key
    pub fn with_dkim_public_key(mut self, public_key: String) -> Self {
        self.dkim_public_key = Some(public_key);
        self
    }

    /// Generate all required DNS records
    pub fn generate_records(&self) -> Result<Vec<DnsRecord>> {
        let mut records = Vec::new();

        // A record for mail server
        records.push(DnsRecord::new(
            DnsRecordType::A,
            self.mail_server_hostname.clone(),
            self.server_ip.to_string(),
            3600,
            "Mail server IP address".to_string(),
        ));

        // MX record
        records.push(DnsRecord::mx(
            self.domain.clone(),
            format!("{}.", self.mail_server_hostname),
            10,
            3600,
        ));

        // SPF record
        let spf_record = format!("v=spf1 mx a:{} -all", self.mail_server_hostname);
        records.push(DnsRecord::new(
            DnsRecordType::TXT,
            self.domain.clone(),
            format!("\"{}\"", spf_record),
            3600,
            "SPF policy - Only this server can send email".to_string(),
        ));

        // DKIM record
        if let Some(ref pubkey) = self.dkim_public_key {
            let dkim_name = format!("{}._domainkey.{}", self.dkim_selector, self.domain);
            let dkim_value = format!("v=DKIM1; k=rsa; p={}", pubkey.replace('\n', ""));
            records.push(DnsRecord::new(
                DnsRecordType::TXT,
                dkim_name,
                format!("\"{}\"", dkim_value),
                3600,
                "DKIM public key for email signing".to_string(),
            ));
        }

        // DMARC record
        let dmarc_name = format!("_dmarc.{}", self.domain);
        let dmarc_value = "v=DMARC1; p=quarantine; rua=mailto:postmaster@".to_string()
            + &self.domain
            + "; pct=100; adkim=s; aspf=s";
        records.push(DnsRecord::new(
            DnsRecordType::TXT,
            dmarc_name,
            format!("\"{}\"", dmarc_value),
            3600,
            "DMARC policy - Quarantine unauthenticated emails".to_string(),
        ));

        // Autodiscover for mail clients (optional)
        records.push(DnsRecord::new(
            DnsRecordType::CNAME,
            format!("autoconfig.{}", self.domain),
            format!("{}.", self.mail_server_hostname),
            3600,
            "Autoconfiguration for mail clients".to_string(),
        ));

        records.push(DnsRecord::new(
            DnsRecordType::CNAME,
            format!("autodiscover.{}", self.domain),
            format!("{}.", self.mail_server_hostname),
            3600,
            "Autodiscovery for mail clients".to_string(),
        ));

        Ok(records)
    }

    /// Generate human-readable DNS setup instructions
    pub fn generate_instructions(&self) -> Result<String> {
        let records = self.generate_records()?;

        let mut instructions = String::new();
        instructions.push_str(&format!(
            "DNS Configuration for {}\n",
            self.domain
        ));
        instructions.push_str("=".repeat(60).as_str());
        instructions.push_str("\n\n");

        instructions.push_str("Add the following DNS records to your domain:\n\n");

        for record in &records {
            instructions.push_str(&format!("Record Type: {}\n", record.record_type));
            instructions.push_str(&format!("Name: {}\n", record.name));
            instructions.push_str(&format!("Value: {}\n", record.value));
            if let Some(priority) = record.priority {
                instructions.push_str(&format!("Priority: {}\n", priority));
            }
            instructions.push_str(&format!("TTL: {} seconds\n", record.ttl));
            instructions.push_str(&format!("Purpose: {}\n", record.description));
            instructions.push_str("\n");
        }

        instructions.push_str("IMPORTANT NOTES:\n");
        instructions.push_str("- DNS changes may take up to 48 hours to propagate\n");
        instructions.push_str("- Verify your SPF record with: dig TXT yourdomain.com\n");
        instructions.push_str("- Verify your DKIM record with: dig TXT selector._domainkey.yourdomain.com\n");
        instructions.push_str("- Verify your DMARC record with: dig TXT _dmarc.yourdomain.com\n");
        instructions.push_str("- Test your configuration at: https://mxtoolbox.com/\n");

        Ok(instructions)
    }

    /// Generate zone file format
    pub fn generate_zone_file(&self) -> Result<String> {
        let records = self.generate_records()?;

        let mut zone = String::new();
        zone.push_str(&format!("; DNS Zone file for {}\n", self.domain));
        zone.push_str(&format!("; Generated on {}\n\n", chrono::Utc::now()));

        for record in &records {
            zone.push_str(&record.to_zone_line());
            zone.push_str(&format!(" ; {}\n", record.description));
        }

        Ok(zone)
    }

    /// Verify DNS records are correctly configured
    pub async fn verify_records(&self) -> Result<Vec<(DnsRecord, bool, String)>> {
        let records = self.generate_records()?;
        let mut results = Vec::new();

        // For now, return placeholder results
        // In production, this would use DNS queries to verify
        for record in records {
            results.push((
                record.clone(),
                false,
                "Verification not yet implemented".to_string(),
            ));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_dns_record_new() {
        let record = DnsRecord::new(
            DnsRecordType::A,
            "mail.example.com".to_string(),
            "192.0.2.1".to_string(),
            3600,
            "Test record".to_string(),
        );

        assert_eq!(record.record_type, DnsRecordType::A);
        assert_eq!(record.name, "mail.example.com");
        assert_eq!(record.value, "192.0.2.1");
        assert_eq!(record.ttl, 3600);
    }

    #[test]
    fn test_dns_record_mx() {
        let record = DnsRecord::mx(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            10,
            3600,
        );

        assert_eq!(record.record_type, DnsRecordType::MX);
        assert_eq!(record.priority, Some(10));
    }

    #[test]
    fn test_dns_record_to_zone_line() {
        let record = DnsRecord::new(
            DnsRecordType::A,
            "mail.example.com".to_string(),
            "192.0.2.1".to_string(),
            3600,
            "Test".to_string(),
        );

        let zone_line = record.to_zone_line();
        assert!(zone_line.contains("mail.example.com"));
        assert!(zone_line.contains("A"));
        assert!(zone_line.contains("192.0.2.1"));
    }

    #[test]
    fn test_dns_record_mx_zone_line() {
        let record = DnsRecord::mx(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            10,
            3600,
        );

        let zone_line = record.to_zone_line();
        assert!(zone_line.contains("MX"));
        assert!(zone_line.contains("10"));
        assert!(zone_line.contains("mail.example.com"));
    }

    #[test]
    fn test_dns_config_generator_new() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        );

        assert_eq!(generator.domain, "example.com");
        assert_eq!(generator.mail_server_hostname, "mail.example.com");
    }

    #[test]
    fn test_generate_records_without_dkim() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        );

        let records = generator.generate_records().unwrap();

        // Should have: A, MX, SPF, DMARC, 2x CNAME (no DKIM without key)
        assert_eq!(records.len(), 6);

        // Check A record
        assert!(records.iter().any(|r| r.record_type == DnsRecordType::A));

        // Check MX record
        let mx_record = records
            .iter()
            .find(|r| r.record_type == DnsRecordType::MX)
            .unwrap();
        assert_eq!(mx_record.priority, Some(10));

        // Check SPF record
        let spf_record = records
            .iter()
            .find(|r| r.value.contains("v=spf1"))
            .unwrap();
        assert!(spf_record.value.contains("mail.example.com"));
    }

    #[test]
    fn test_generate_records_with_dkim() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        )
        .with_dkim_public_key("MIGfMA0GCS...".to_string());

        let records = generator.generate_records().unwrap();

        // Should have: A, MX, SPF, DKIM, DMARC, 2x CNAME
        assert_eq!(records.len(), 7);

        // Check DKIM record
        let dkim_record = records
            .iter()
            .find(|r| r.name.contains("_domainkey"))
            .unwrap();
        assert!(dkim_record.value.contains("v=DKIM1"));
        assert!(dkim_record.value.contains("k=rsa"));
    }

    #[test]
    fn test_generate_instructions() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        );

        let instructions = generator.generate_instructions().unwrap();

        assert!(instructions.contains("DNS Configuration for example.com"));
        assert!(instructions.contains("Record Type:"));
        assert!(instructions.contains("IMPORTANT NOTES"));
        assert!(instructions.contains("mxtoolbox.com"));
    }

    #[test]
    fn test_generate_zone_file() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        );

        let zone = generator.generate_zone_file().unwrap();

        assert!(zone.contains("; DNS Zone file for example.com"));
        assert!(zone.contains("IN"));
        assert!(zone.contains("A"));
        assert!(zone.contains("MX"));
    }

    #[test]
    fn test_dns_record_type_display() {
        assert_eq!(DnsRecordType::A.to_string(), "A");
        assert_eq!(DnsRecordType::AAAA.to_string(), "AAAA");
        assert_eq!(DnsRecordType::MX.to_string(), "MX");
        assert_eq!(DnsRecordType::TXT.to_string(), "TXT");
        assert_eq!(DnsRecordType::CNAME.to_string(), "CNAME");
        assert_eq!(DnsRecordType::PTR.to_string(), "PTR");
    }

    #[tokio::test]
    async fn test_verify_records() {
        let ip = IpAddr::V4(Ipv4Addr::new(192, 0, 2, 1));
        let generator = DnsConfigGenerator::new(
            "example.com".to_string(),
            "mail.example.com".to_string(),
            ip,
            "default".to_string(),
        );

        let results = generator.verify_records().await.unwrap();
        assert!(!results.is_empty());
    }
}
