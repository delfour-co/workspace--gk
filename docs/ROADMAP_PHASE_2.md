# Roadmap Phase 2 : Production Mail Server

**Date de création**: 2025-12-06
**Status**: En cours - Sprint 11 complété

---

## 📋 Vue d'ensemble

Phase 2 vise à transformer le serveur mail de base en un système production-ready avec authentification email, gestion avancée, et intégration complète type Mail-in-a-Box.

### Progression Globale

- ✅ **Sprint 11: SPF + DKIM** - Complété (46/46 tests)
- 🔄 **Sprint 12: DMARC** - À démarrer
- ⏳ **Sprint 13: Pièces Jointes** - Planifié
- ⏳ **Sprint 14: Quotas** - Planifié
- ⏳ **Sprint 15: Greylisting** - Planifié
- ⏳ **Sprint 16: Mail-in-a-Box Integration** - Nouveau

---

## ✅ Sprint 11: SPF + DKIM (COMPLÉTÉ)

**Dates**: 2025-12-03 → 2025-12-06
**Status**: ✅ Implementation Complete

### Réalisations

- ✅ Modules SPF/DKIM complets (1400+ lignes)
- ✅ 46 tests unitaires (100% pass rate)
- ✅ Intégration SMTP pour validation entrante
- ✅ Configuration complète (config.toml + AuthenticationConfig)
- ✅ Test E2E réussi (Authentication-Results header ajouté)
- ✅ Documentation complète

### Résultats E2E

```
📧 Email sent successfully!
✅ Authentication-Results header found!
   Authentication-Results: mail.delfour.co; spf=fail smtp.mailfrom=test@example.com
📊 Validation Results:
   SPF validated: ✅
   DKIM validated: ❌ (pas de signature dans le message test)
```

### À finaliser (optionnel)

- [ ] DKIM signing pour emails sortants (relay/forward)
- [ ] Configuration DNS (SPF + DKIM records) - Par l'utilisateur
- [ ] Tests production Gmail/Outlook

---

## 🔄 Sprint 12: DMARC

**Durée estimée**: 2-3 jours
**Dépendances**: Sprint 11 (SPF + DKIM)

### Objectifs

Implémenter DMARC (Domain-based Message Authentication, Reporting & Conformance) pour :
1. Définir politique d'authentification basée sur SPF/DKIM
2. Alignement domaine (From vs envelope-from)
3. Reporting des échecs d'authentification
4. Protection contre phishing/spoofing

### Fonctionnalités

#### 1. DMARC Validation (Entrant)

**Fichier**: `mail-rs/src/authentication/dmarc.rs`

```rust
pub struct DmarcValidator {
    resolver: Arc<Resolver>,
}

pub struct DmarcResult {
    pub policy: DmarcPolicy,  // none, quarantine, reject
    pub spf_aligned: bool,
    pub dkim_aligned: bool,
    pub pass: bool,
    pub reason: Option<String>,
}

pub enum DmarcPolicy {
    None,       // p=none : monitoring only
    Quarantine, // p=quarantine : mark as spam
    Reject,     // p=reject : block email
}

impl DmarcValidator {
    // Récupérer policy DMARC du domaine
    pub async fn get_policy(&self, domain: &str) -> Result<DmarcPolicy>;

    // Valider l'email contre DMARC
    pub async fn validate(
        &self,
        from_domain: &str,
        spf_result: &SpfAuthResult,
        dkim_result: &DkimAuthResult
    ) -> DmarcResult;

    // Vérifier alignement SPF
    fn check_spf_alignment(&self, from: &str, envelope_from: &str) -> bool;

    // Vérifier alignement DKIM
    fn check_dkim_alignment(&self, from: &str, dkim_domain: &str) -> bool;
}
```

#### 2. DMARC Reporting

**Fichier**: `mail-rs/src/authentication/dmarc_reporter.rs`

```rust
pub struct DmarcReporter {
    storage: Arc<dyn Storage>,
    config: DmarcReportConfig,
}

pub struct DmarcReport {
    pub org_name: String,
    pub email: String,
    pub report_id: String,
    pub date_range: (DateTime<Utc>, DateTime<Utc>),
    pub records: Vec<DmarcRecord>,
}

impl DmarcReporter {
    // Enregistrer échec d'authentification
    pub async fn record_failure(&self, failure: DmarcFailure);

    // Générer rapport quotidien
    pub async fn generate_daily_report(&self) -> DmarcReport;

    // Envoyer rapport au domaine source
    pub async fn send_report(&self, report: DmarcReport) -> Result<()>;
}
```

#### 3. Intégration dans SmtpSession

```rust
// Dans receive_data()
let dmarc_result = if let Some(ref validator) = self.dmarc_validator {
    validator.validate(
        &from_domain,
        &auth_result.spf,
        &auth_result.dkim
    ).await?
} else {
    DmarcResult::default()
};

// Appliquer politique DMARC
match dmarc_result.policy {
    DmarcPolicy::Reject if !dmarc_result.pass => {
        return Err("550 DMARC validation failed");
    },
    DmarcPolicy::Quarantine if !dmarc_result.pass => {
        // Marquer comme spam
        self.add_header("X-Spam-Flag", "YES");
    },
    _ => {}
}
```

### Configuration

**config.toml**:
```toml
[authentication.dmarc]
enabled = true
check_policy = true
send_reports = false  # Désactivé par défaut
report_email = "dmarc-reports@delfour.co"
report_organization = "delfour.co"
```

### Tests

- [ ] Test DMARC DNS lookup (p=none, p=quarantine, p=reject)
- [ ] Test alignement SPF (strict vs relaxed)
- [ ] Test alignement DKIM (strict vs relaxed)
- [ ] Test avec domaines réels (gmail.com, yahoo.com)
- [ ] Test génération rapports
- [ ] 20+ tests unitaires minimum

### Livrables

1. Module `dmarc.rs` (validation)
2. Module `dmarc_reporter.rs` (reporting)
3. Intégration SmtpSession
4. Configuration DMARC
5. Tests unitaires (20+)
6. Documentation DMARC
7. Guide configuration DNS DMARC

---

## 📎 Sprint 13: Gestion des Pièces Jointes

**Durée estimée**: 3-4 jours
**Dépendances**: Aucune

### Objectifs

Support complet des pièces jointes (MIME multipart) :
1. Parser emails multipart/mixed
2. Extraire et stocker pièces jointes
3. Validation (taille, types, virus)
4. API pour récupérer attachments
5. Interface web pour télécharger

### Fonctionnalités

#### 1. MIME Parser

**Fichier**: `mail-rs/src/mime/parser.rs`

```rust
pub struct MimeParser;

pub struct MimePart {
    pub content_type: String,
    pub content_disposition: Option<String>,
    pub filename: Option<String>,
    pub body: Vec<u8>,
    pub is_attachment: bool,
}

pub struct ParsedEmail {
    pub headers: HashMap<String, String>,
    pub text_body: Option<String>,
    pub html_body: Option<String>,
    pub attachments: Vec<MimePart>,
}

impl MimeParser {
    pub fn parse(message: &[u8]) -> Result<ParsedEmail>;

    fn parse_multipart(boundary: &str, body: &[u8]) -> Vec<MimePart>;

    fn decode_base64(content: &str) -> Vec<u8>;

    fn decode_quoted_printable(content: &str) -> String;
}
```

#### 2. Attachment Storage

**Fichier**: `mail-rs/src/storage/attachments.rs`

```rust
pub struct AttachmentStorage {
    base_path: PathBuf,
}

pub struct Attachment {
    pub id: String,
    pub filename: String,
    pub content_type: String,
    pub size: u64,
    pub path: PathBuf,
}

impl AttachmentStorage {
    // Sauvegarder pièce jointe
    pub async fn store(&self, email_id: &str, part: &MimePart) -> Result<Attachment>;

    // Récupérer pièce jointe
    pub async fn get(&self, attachment_id: &str) -> Result<Vec<u8>>;

    // Lister pièces jointes d'un email
    pub async fn list_for_email(&self, email_id: &str) -> Vec<Attachment>;

    // Supprimer pièce jointe
    pub async fn delete(&self, attachment_id: &str) -> Result<()>;
}
```

#### 3. Validation des Pièces Jointes

```rust
pub struct AttachmentValidator {
    max_size: u64,
    allowed_types: Vec<String>,
    virus_scanner: Option<Arc<VirusScanner>>,
}

impl AttachmentValidator {
    // Valider taille
    pub fn check_size(&self, size: u64) -> Result<()>;

    // Valider type MIME
    pub fn check_type(&self, content_type: &str) -> Result<()>;

    // Scanner virus (avec ClamAV)
    pub async fn scan_virus(&self, data: &[u8]) -> Result<()>;
}
```

#### 4. API Endpoints

**Fichier**: `mail-rs/src/api/attachments.rs`

```rust
// GET /api/emails/{id}/attachments
pub async fn list_attachments(
    Path(email_id): Path<String>
) -> Result<Json<Vec<Attachment>>>;

// GET /api/attachments/{id}
pub async fn download_attachment(
    Path(attachment_id): Path<String>
) -> Result<Response>;

// DELETE /api/attachments/{id}
pub async fn delete_attachment(
    Path(attachment_id): Path<String>
) -> Result<StatusCode>;
```

### Configuration

**config.toml**:
```toml
[attachments]
enabled = true
max_size = 26214400  # 25MB
storage_path = "data/attachments"
allowed_types = ["*"]  # All types by default
scan_virus = false  # Requires ClamAV
```

### Tests

- [ ] Parser multipart/mixed
- [ ] Parser multipart/alternative
- [ ] Base64 decoding
- [ ] Quoted-printable decoding
- [ ] Extraction nom fichier
- [ ] Storage pièces jointes
- [ ] Validation taille
- [ ] Validation type MIME
- [ ] API endpoints
- [ ] 30+ tests unitaires

### Livrables

1. Module `mime/parser.rs`
2. Module `storage/attachments.rs`
3. Module `validation/attachments.rs`
4. API endpoints attachments
5. Tests unitaires (30+)
6. Documentation MIME/attachments

---

## 📊 Sprint 14: Quotas et Limites

**Durée estimée**: 2-3 jours
**Dépendances**: Aucune

### Objectifs

Implémenter quotas par utilisateur/domaine :
1. Limite stockage mailbox
2. Limite taille messages
3. Limite nombre messages/jour
4. Rate limiting SMTP
5. Reporting usage

### Fonctionnalités

#### 1. Quota Manager

**Fichier**: `mail-rs/src/quota/manager.rs`

```rust
pub struct QuotaManager {
    db: Arc<Database>,
}

pub struct UserQuota {
    pub email: String,
    pub storage_limit: u64,      // Bytes
    pub storage_used: u64,
    pub message_limit_daily: u32,
    pub message_count_today: u32,
    pub max_message_size: u64,
}

impl QuotaManager {
    // Vérifier quota storage
    pub async fn check_storage(&self, email: &str) -> Result<bool>;

    // Vérifier quota messages
    pub async fn check_message_limit(&self, email: &str) -> Result<bool>;

    // Mettre à jour usage
    pub async fn update_usage(&self, email: &str, message_size: u64) -> Result<()>;

    // Récupérer quota utilisateur
    pub async fn get_quota(&self, email: &str) -> Result<UserQuota>;

    // Définir quota utilisateur
    pub async fn set_quota(&self, email: &str, quota: UserQuota) -> Result<()>;
}
```

#### 2. Rate Limiter

**Fichier**: `mail-rs/src/quota/rate_limiter.rs`

```rust
pub struct RateLimiter {
    redis: Option<Arc<RedisClient>>,
    limits: HashMap<String, RateLimit>,
}

pub struct RateLimit {
    pub max_requests: u32,
    pub window_secs: u64,
}

impl RateLimiter {
    // Vérifier limite
    pub async fn check(&self, key: &str, limit: &RateLimit) -> Result<bool>;

    // Incrémenter compteur
    pub async fn increment(&self, key: &str) -> Result<()>;

    // Reset compteur
    pub async fn reset(&self, key: &str) -> Result<()>;
}
```

#### 3. Storage Quota

```rust
impl MaildirStorage {
    // Calculer taille utilisée
    pub async fn calculate_usage(&self, email: &str) -> Result<u64>;

    // Vérifier quota avant stockage
    async fn check_quota_before_store(&self, email: &str, size: u64) -> Result<()>;
}
```

#### 4. API Endpoints

```rust
// GET /api/users/{email}/quota
pub async fn get_quota(
    Path(email): Path<String>
) -> Result<Json<UserQuota>>;

// PUT /api/users/{email}/quota
pub async fn update_quota(
    Path(email): Path<String>,
    Json(quota): Json<UserQuota>
) -> Result<StatusCode>;

// GET /api/users/{email}/usage
pub async fn get_usage(
    Path(email): Path<String>
) -> Result<Json<UsageStats>>;
```

### Configuration

**config.toml**:
```toml
[quotas]
enabled = true
default_storage_mb = 1024  # 1GB per user
default_daily_messages = 100
max_message_size_mb = 25

[quotas.rate_limiting]
enabled = true
smtp_connections_per_ip = 10  # per minute
smtp_messages_per_user = 50   # per hour
```

### Tests

- [ ] Check storage quota
- [ ] Check message limit
- [ ] Update usage
- [ ] Rate limiting
- [ ] Quota exceeded errors
- [ ] API endpoints
- [ ] 25+ tests unitaires

### Livrables

1. Module `quota/manager.rs`
2. Module `quota/rate_limiter.rs`
3. Intégration SMTP/Storage
4. API endpoints quotas
5. Tests unitaires (25+)
6. Documentation quotas

---

## 🛡️ Sprint 15: Greylisting Anti-Spam

**Durée estimée**: 2-3 jours
**Dépendances**: Aucune

### Objectifs

Implémenter greylisting pour réduire spam :
1. Temporiser emails de nouveaux expéditeurs
2. Whitelist expéditeurs légitimes
3. Blacklist spammers persistants
4. Configuration politique greylisting

### Fonctionnalités

#### 1. Greylist Manager

**Fichier**: `mail-rs/src/antispam/greylist.rs`

```rust
pub struct GreylistManager {
    db: Arc<Database>,
    config: GreylistConfig,
}

pub struct GreylistEntry {
    pub sender: String,
    pub recipient: String,
    pub client_ip: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    pub attempts: u32,
    pub status: GreylistStatus,
}

pub enum GreylistStatus {
    Greylisted,  // Temporairement rejeté
    Whitelisted, // Accepté permanent
    Blacklisted, // Rejeté permanent
}

impl GreylistManager {
    // Vérifier si email doit être greylisté
    pub async fn check(
        &self,
        sender: &str,
        recipient: &str,
        client_ip: &str
    ) -> Result<GreylistStatus>;

    // Enregistrer tentative
    pub async fn record_attempt(&self, entry: &GreylistEntry) -> Result<()>;

    // Whitelist automatique après délai
    pub async fn auto_whitelist(&self, entry: &GreylistEntry) -> Result<()>;

    // Cleanup anciennes entrées
    pub async fn cleanup_old_entries(&self) -> Result<()>;
}
```

#### 2. Integration SMTP

```rust
// Dans SmtpSession::receive_data()
if self.greylist.is_enabled() {
    let status = self.greylist.check(
        &self.from,
        &self.to,
        &self.client_ip.to_string()
    ).await?;

    match status {
        GreylistStatus::Greylisted => {
            return Err("451 Greylisted: Please try again later");
        },
        GreylistStatus::Blacklisted => {
            return Err("550 Sender blacklisted");
        },
        GreylistStatus::Whitelisted => {
            // Continue normal processing
        }
    }
}
```

#### 3. Whitelist/Blacklist Management

```rust
impl GreylistManager {
    // Ajouter à la whitelist
    pub async fn add_to_whitelist(&self, entry: WhitelistEntry) -> Result<()>;

    // Ajouter à la blacklist
    pub async fn add_to_blacklist(&self, entry: BlacklistEntry) -> Result<()>;

    // Lister whitelist
    pub async fn list_whitelist(&self) -> Result<Vec<WhitelistEntry>>;

    // Lister blacklist
    pub async fn list_blacklist(&self) -> Result<Vec<BlacklistEntry>>;
}
```

#### 4. API Endpoints

```rust
// POST /api/greylist/whitelist
pub async fn add_whitelist(
    Json(entry): Json<WhitelistEntry>
) -> Result<StatusCode>;

// POST /api/greylist/blacklist
pub async fn add_blacklist(
    Json(entry): Json<BlacklistEntry>
) -> Result<StatusCode>;

// GET /api/greylist/status/{sender}
pub async fn check_status(
    Path(sender): Path<String>
) -> Result<Json<GreylistStatus>>;
```

### Configuration

**config.toml**:
```toml
[antispam.greylist]
enabled = true
delay_seconds = 300  # 5 minutes
auto_whitelist_after_days = 7
cleanup_after_days = 30
whitelist_on_auth = true  # Whitelist authenticated users
```

### Tests

- [ ] Check greylist status
- [ ] Record attempts
- [ ] Auto-whitelist après délai
- [ ] Whitelist manual
- [ ] Blacklist
- [ ] Cleanup old entries
- [ ] SMTP integration
- [ ] 20+ tests unitaires

### Livrables

1. Module `antispam/greylist.rs`
2. Intégration SmtpSession
3. API endpoints whitelist/blacklist
4. Tests unitaires (20+)
5. Documentation greylisting

---

## 🎯 Sprint 16: Mail-in-a-Box Integration (NOUVEAU)

**Durée estimée**: 5-7 jours
**Dépendances**: Sprints 11-15 complétés

### Objectifs

Créer une intégration complète similaire à Mail-in-a-Box :
1. Installation automatisée
2. Configuration DNS automatique
3. Interface web admin complète
4. Monitoring et diagnostics
5. Backups automatiques
6. Let's Encrypt SSL auto

### Fonctionnalités

#### 1. Auto-Installation Script

**Fichier**: `scripts/install.sh`

```bash
#!/bin/bash
# Installation automatique du serveur mail

set -e

# Vérifications système
check_system() {
    # OS: Ubuntu 22.04/24.04
    # RAM: >= 1GB
    # Disk: >= 10GB
}

# Installation dépendances
install_dependencies() {
    apt update
    apt install -y build-essential pkg-config libssl-dev
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
}

# Configuration utilisateur
setup_users() {
    useradd -r -s /bin/false mail-rs
    mkdir -p /var/mail-rs/{data,logs,config}
    chown -R mail-rs:mail-rs /var/mail-rs
}

# Installation mail-rs
install_mailrs() {
    cargo build --release
    cp target/release/mail-rs /usr/local/bin/
    chmod +x /usr/local/bin/mail-rs
}

# Configuration systemd
setup_systemd() {
    cat > /etc/systemd/system/mail-rs.service <<EOF
[Unit]
Description=mail-rs SMTP/IMAP Server
After=network.target

[Service]
Type=simple
User=mail-rs
WorkingDirectory=/var/mail-rs
ExecStart=/usr/local/bin/mail-rs
Restart=always

[Install]
WantedBy=multi-user.target
EOF

    systemctl daemon-reload
    systemctl enable mail-rs
    systemctl start mail-rs
}

# Configuration DNS helper
setup_dns_helper() {
    # Détection IP publique
    PUBLIC_IP=$(curl -s https://api.ipify.org)

    echo "Configure these DNS records:"
    echo "  A     mail.yourdomain.com    $PUBLIC_IP"
    echo "  MX    yourdomain.com          mail.yourdomain.com (priority 10)"
    echo "  TXT   yourdomain.com          v=spf1 ip4:$PUBLIC_IP ~all"
    echo "  TXT   default._domainkey      (see /var/mail-rs/dkim_public.txt)"
}

main() {
    check_system
    install_dependencies
    setup_users
    install_mailrs
    setup_systemd
    setup_dns_helper

    echo "✅ Installation complete!"
    echo "Next: Configure DNS and run: systemctl status mail-rs"
}

main
```

#### 2. DNS Auto-Configuration

**Fichier**: `mail-rs/src/admin/dns.rs`

```rust
pub struct DnsConfigurator {
    domain: String,
    public_ip: IpAddr,
    dkim_selector: String,
}

impl DnsConfigurator {
    // Générer configuration DNS
    pub fn generate_dns_records(&self) -> DnsRecords;

    // Vérifier DNS
    pub async fn verify_dns(&self) -> DnsVerificationResult;

    // Configurer automatiquement (Cloudflare API, etc.)
    pub async fn auto_configure(&self, api_key: &str) -> Result<()>;
}

pub struct DnsRecords {
    pub a_record: String,
    pub mx_record: String,
    pub spf_record: String,
    pub dkim_record: String,
    pub dmarc_record: String,
}

pub struct DnsVerificationResult {
    pub a_ok: bool,
    pub mx_ok: bool,
    pub spf_ok: bool,
    pub dkim_ok: bool,
    pub dmarc_ok: bool,
    pub issues: Vec<String>,
}
```

#### 3. Interface Web Admin Complète

**Fichier**: `web-ui/src/pages/Admin.tsx`

Fonctionnalités :
- Dashboard avec métriques (emails/jour, stockage, quotas)
- Gestion utilisateurs (créer, modifier, supprimer)
- Configuration DNS (vérification, guidance)
- Monitoring temps réel (logs, erreurs)
- Whitelist/Blacklist management
- Quotas par utilisateur
- Backups management
- SSL certificate status

#### 4. Monitoring et Diagnostics

**Fichier**: `mail-rs/src/admin/diagnostics.rs`

```rust
pub struct DiagnosticsTool {
    config: Arc<Config>,
}

pub struct SystemHealth {
    pub smtp_status: ServiceStatus,
    pub imap_status: ServiceStatus,
    pub api_status: ServiceStatus,
    pub disk_usage: DiskUsage,
    pub memory_usage: MemoryUsage,
    pub queue_size: usize,
    pub errors_last_hour: usize,
}

impl DiagnosticsTool {
    // Vérifier santé système
    pub async fn check_health(&self) -> SystemHealth;

    // Tester SMTP
    pub async fn test_smtp_send(&self, to: &str) -> Result<()>;

    // Tester IMAP
    pub async fn test_imap_connect(&self) -> Result<()>;

    // Vérifier DNS
    pub async fn verify_dns(&self) -> DnsVerificationResult;

    // Tester deliverability
    pub async fn test_deliverability(&self) -> Result<DeliverabilityScore>;
}
```

#### 5. Backups Automatiques

**Fichier**: `mail-rs/src/admin/backup.rs`

```rust
pub struct BackupManager {
    backup_path: PathBuf,
    retention_days: u32,
}

impl BackupManager {
    // Créer backup complet
    pub async fn create_backup(&self) -> Result<BackupInfo>;

    // Restaurer backup
    pub async fn restore_backup(&self, backup_id: &str) -> Result<()>;

    // Lister backups
    pub async fn list_backups(&self) -> Vec<BackupInfo>;

    // Cleanup vieux backups
    pub async fn cleanup_old_backups(&self) -> Result<()>;

    // Backup automatique quotidien
    pub async fn schedule_daily_backup(&self);
}
```

#### 6. Let's Encrypt SSL Auto

**Fichier**: `mail-rs/src/admin/ssl.rs`

```rust
pub struct SslManager {
    acme_client: AcmeClient,
    domain: String,
}

impl SslManager {
    // Obtenir certificat Let's Encrypt
    pub async fn obtain_certificate(&self) -> Result<Certificate>;

    // Renouveler certificat
    pub async fn renew_certificate(&self) -> Result<()>;

    // Auto-renouvellement (cron)
    pub async fn schedule_auto_renewal(&self);

    // Vérifier validité certificat
    pub fn check_certificate_validity(&self) -> CertificateStatus;
}
```

### Configuration

**config.toml**:
```toml
[admin]
enabled = true
auto_backup = true
backup_retention_days = 30

[admin.ssl]
auto_ssl = true
acme_email = "admin@delfour.co"
acme_server = "https://acme-v02.api.letsencrypt.org/directory"

[admin.monitoring]
enabled = true
health_check_interval_secs = 60
alert_email = "admin@delfour.co"
```

### Tests

- [ ] Installation script
- [ ] DNS configuration
- [ ] DNS verification
- [ ] Health checks
- [ ] Backups create/restore
- [ ] SSL certificate obtain/renew
- [ ] Web UI admin
- [ ] 40+ tests unitaires

### Livrables

1. Script `install.sh` complet
2. Module `admin/dns.rs`
3. Module `admin/diagnostics.rs`
4. Module `admin/backup.rs`
5. Module `admin/ssl.rs`
6. Interface web admin
7. Tests unitaires (40+)
8. Documentation complète installation

---

## 📊 Résumé et Timeline

### Timeline Globale

| Sprint | Durée | Début | Fin estimée |
|--------|-------|-------|-------------|
| Sprint 11: SPF + DKIM | 4 jours | 2025-12-03 | ✅ 2025-12-06 |
| Sprint 12: DMARC | 3 jours | 2025-12-06 | 2025-12-09 |
| Sprint 13: Pièces Jointes | 4 jours | 2025-12-09 | 2025-12-13 |
| Sprint 14: Quotas | 3 jours | 2025-12-13 | 2025-12-16 |
| Sprint 15: Greylisting | 3 jours | 2025-12-16 | 2025-12-19 |
| Sprint 16: Mail-in-a-Box | 7 jours | 2025-12-19 | 2025-12-26 |

**Durée totale Phase 2**: ~24 jours (3.5 semaines)

### Statistiques Prévisionnelles

**Code**:
- Lignes de code totales: ~8000 lignes
- Tests unitaires: ~180 tests
- Modules créés: ~25 modules

**Fonctionnalités**:
- ✅ Authentication email complète (SPF, DKIM, DMARC)
- ✅ Gestion pièces jointes
- ✅ Quotas et rate limiting
- ✅ Anti-spam (greylisting)
- ✅ Installation automatisée
- ✅ Interface admin web
- ✅ Monitoring et diagnostics
- ✅ Backups automatiques
- ✅ SSL automatique

---

## 🎯 Critères de Succès Phase 2

### Critères Techniques

1. **Authentification Email**
   - ✅ SPF validation fonctionnelle
   - ✅ DKIM signing et validation
   - ⏳ DMARC policy enforcement
   - Score mail-tester.com >= 8/10

2. **Robustesse**
   - Gestion pièces jointes jusqu'à 25MB
   - Quotas respectés (storage, rate limits)
   - Greylisting réduit spam >80%
   - Uptime >99.9%

3. **Administration**
   - Installation en <30 minutes
   - Configuration DNS semi-automatique
   - Interface admin intuitive
   - Backups quotidiens automatiques
   - SSL auto-renouvelé

4. **Performance**
   - <2s validation SPF/DKIM/DMARC
   - <500ms API response
   - Support 1000+ users
   - 10,000+ emails/jour

### Critères Utilisateur

1. **Facilité d'installation**
   - Un script d'installation
   - Guide DNS clair
   - Configuration minimale

2. **Interface Admin**
   - Dashboard complet
   - Gestion utilisateurs simple
   - Monitoring en temps réel

3. **Fiabilité**
   - Emails ne vont pas en spam
   - Backups automatiques
   - Alertes en cas de problème

---

## 📚 Ressources et Références

### Standards RFC

- **SPF**: RFC 7208
- **DKIM**: RFC 6376
- **DMARC**: RFC 7489
- **MIME**: RFC 2045-2049
- **SMTP**: RFC 5321
- **IMAP**: RFC 3501

### Outils Similaires

- **Mail-in-a-Box**: https://mailinabox.email/
- **Mailcow**: https://mailcow.email/
- **iRedMail**: https://www.iredmail.org/
- **Mailu**: https://mailu.io/

### Crates Rust Utiles

- `mail-auth` - SPF/DKIM/DMARC
- `lettre` - SMTP client
- `mail-parser` - MIME parsing
- `trust-dns-resolver` - DNS
- `instant-acme` - Let's Encrypt
- `sqlx` - Database
- `axum` - Web framework

---

**Status**: 🟢 Roadmap définie - Prêt pour Sprint 12
**Prochaine étape**: Démarrer Sprint 12 (DMARC)
**ETA Production complète**: 3.5 semaines
