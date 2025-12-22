# 🎯 Features Ready-to-Use Prompts

> Prompts détaillés et actionables pour les 8 features prioritaires de workspace--gk

## Table des Matières

**Sécurité Avancée**
1. [Blockchain Proof of Email](#1-blockchain-proof-of-email)
2. [Human Lock Captcha](#2-human-lock-captcha)
3. [AI Link Scanner](#3-ai-link-scanner)
4. [Security Dashboard](#4-security-dashboard)

**Productivité**
5. [Email Templates](#5-email-templates)
6. [Auto-Reply / Vacation](#6-auto-reply--vacation)
7. [Email Scheduling](#7-email-scheduling)
8. [Email Threading](#8-email-threading)

---

## 🔐 Sécurité Avancée

### 1. Blockchain Proof of Email

#### Contexte
Implémenter un système de preuve d'horodatage infalsifiable pour les emails en utilisant OpenTimestamps. Chaque email envoyé/reçu recevra une preuve blockchain prouvant son existence à un instant T.

#### Objectif
Créer un module `mail-rs/src/blockchain/opentimestamps.rs` qui génère des preuves cryptographiques d'horodatage pour les emails, stockées dans une base SQLite et vérifiables publiquement.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/blockchain/mod.rs
pub mod opentimestamps;
pub mod types;
pub mod storage;

// mail-rs/src/blockchain/types.rs
pub struct EmailProof {
    pub email_id: String,          // Message-ID de l'email
    pub timestamp: DateTime<Utc>,  // Timestamp de l'email
    pub hash: String,              // SHA256 de l'email complet
    pub ots_proof: Vec<u8>,        // Preuve OpenTimestamps (binaire)
    pub merkle_root: String,       // Bitcoin merkle root (si disponible)
    pub verified: bool,            // Si la preuve a été vérifiée
    pub created_at: DateTime<Utc>,
}

pub enum ProofStatus {
    Pending,      // En attente d'inclusion dans Bitcoin
    Confirmed,    // Inclus dans un bloc Bitcoin
    Verified,     // Vérifié avec succès
    Failed,       // Échec de vérification
}
```

**Dépendances à ajouter** (mail-rs/Cargo.toml):
```toml
[dependencies]
opentimestamps = "0.1"
sha2 = "0.10"
hex = "0.4"
reqwest = { version = "0.11", features = ["json"] }
```

**Fonctionnalités**:

1. **Génération de preuve** (à l'envoi/réception d'email):
   - Calculer SHA256 de l'email complet (headers + body)
   - Soumettre le hash à OpenTimestamps API
   - Stocker la preuve dans SQLite
   - Attacher la preuve comme header X-OTS-Proof (optionnel)

2. **Vérification de preuve**:
   - Endpoint API `GET /api/admin/blockchain/verify/:email_id`
   - Vérifier que le hash correspond
   - Vérifier l'inclusion dans Bitcoin blockchain
   - Retourner le bloc Bitcoin + timestamp exact

3. **Interface Admin**:
   - Page `/admin/blockchain` avec:
     - Liste des emails avec preuve
     - Statut de chaque preuve (pending/confirmed/verified)
     - Bouton "Vérifier preuve" → lien vers explorateur Bitcoin
     - Export des preuves en format .ots

4. **Storage Schema** (SQLite):
```sql
CREATE TABLE email_proofs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    email_id TEXT UNIQUE NOT NULL,
    email_hash TEXT NOT NULL,
    ots_proof BLOB,
    merkle_root TEXT,
    bitcoin_block INTEGER,
    status TEXT NOT NULL DEFAULT 'pending',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    verified_at TIMESTAMP
);
CREATE INDEX idx_email_id ON email_proofs(email_id);
CREATE INDEX idx_status ON email_proofs(status);
```

5. **API OpenTimestamps**:
   - POST à `https://a.pool.opentimestamps.org/digest` pour créer preuve
   - GET pour récupérer preuve complète après confirmation
   - Retry logic si la preuve n'est pas encore confirmée

**Cas d'usage**:
- Prouver qu'un email a été envoyé/reçu à une date précise (juridique)
- Vérifier l'authenticité d'un email sans confiance en un tiers
- Audit trail infalsifiable pour compliance

**Tests à implémenter**:
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_generate_email_proof() {
        // Test génération preuve pour un email
    }

    #[tokio::test]
    async fn test_verify_proof() {
        // Test vérification d'une preuve existante
    }

    #[tokio::test]
    async fn test_proof_storage() {
        // Test stockage et récupération SQLite
    }
}
```

**Documentation utilisateur**:
- Expliquer ce qu'est OpenTimestamps
- Comment vérifier une preuve manuellement
- Limites (délai de confirmation ~1-2h pour Bitcoin)

**Configuration** (mail-rs/config.toml):
```toml
[blockchain]
enabled = true
opentimestamps_server = "https://a.pool.opentimestamps.org"
auto_prove_outgoing = true  # Prouver automatiquement emails sortants
auto_prove_incoming = false  # Ne pas prouver emails entrants par défaut
```

---

### 2. Human Lock Captcha

#### Contexte
Implémenter un système de captcha pour les nouveaux expéditeurs afin de bloquer les spambots. Quand un email arrive d'un expéditeur inconnu, envoyer automatiquement un email avec captcha. L'email n'est délivré que si le captcha est résolu.

#### Objectif
Créer un module anti-spam `mail-rs/src/antispam/humanlock.rs` qui détecte les nouveaux expéditeurs et leur envoie un défi captcha avant d'accepter leur email.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/antispam/humanlock.rs
pub struct HumanLock {
    db: SqlitePool,
    config: HumanLockConfig,
}

pub struct HumanLockConfig {
    pub enabled: bool,
    pub whitelist_after_solve: bool,
    pub challenge_expiry_hours: u32,  // Default: 24h
    pub max_attempts: u32,  // Default: 3
}

pub struct ChallengeRecord {
    pub id: String,  // UUID
    pub sender_email: String,
    pub recipient_email: String,
    pub challenge_code: String,  // 6-digit code
    pub original_email_id: String,  // Pour récupérer l'email après validation
    pub attempts: u32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub solved_at: Option<DateTime<Utc>>,
}
```

**Workflow**:

1. **Détection nouveau expéditeur**:
   - Vérifier si sender dans whitelist (table `humanlock_whitelist`)
   - Si nouveau → bloquer email temporairement
   - Générer challenge code 6 digits
   - Envoyer email de challenge

2. **Email de challenge** (template HTML):
```html
Subject: [Action Required] Verify you are human

Hello,

You recently sent an email to {{recipient_email}}.

To ensure you're not a spam bot, please verify you're human by entering this code:

**Code: {{challenge_code}}**

Click here to verify: {{verification_url}}

Or reply to this email with the code in the subject line.

This verification is only required once. After verification, your future emails will be delivered automatically.

The code expires in 24 hours.

---
GK Mail - Human Lock Anti-Spam
```

3. **Vérification**:
   - Méthode 1: Lien web → Page `/verify/:challenge_id` avec formulaire
   - Méthode 2: Répondre à l'email avec code dans subject
   - Si code correct → ajouter à whitelist + délivrer email original

4. **Storage Schema**:
```sql
CREATE TABLE humanlock_challenges (
    id TEXT PRIMARY KEY,
    sender_email TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    challenge_code TEXT NOT NULL,
    original_email_path TEXT NOT NULL,  -- Path dans maildir
    attempts INTEGER DEFAULT 0,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP NOT NULL,
    solved_at TIMESTAMP
);

CREATE TABLE humanlock_whitelist (
    sender_email TEXT PRIMARY KEY,
    recipient_email TEXT,
    added_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    reason TEXT  -- 'solved_challenge' ou 'manual'
);

CREATE INDEX idx_sender ON humanlock_challenges(sender_email);
CREATE INDEX idx_expires ON humanlock_challenges(expires_at);
```

5. **API Endpoints**:
```rust
// POST /api/verify/:challenge_id
pub async fn verify_challenge(
    Path(challenge_id): Path<String>,
    Json(payload): Json<VerifyRequest>,
) -> Result<Json<VerifyResponse>, StatusCode> {
    // Vérifier code
    // Si correct: whitelist + deliver email
    // Si incorrect: incrémenter attempts
}

// GET /verify/:challenge_id (page HTML)
pub async fn verify_page(
    Path(challenge_id): Path<String>,
) -> Result<Html<String>, StatusCode> {
    // Render template avec formulaire
}
```

6. **Page de vérification** (mail-rs/templates/verify.html):
```html
<div class="verify-container">
  <h1>Human Verification</h1>
  <p>Please enter the 6-digit code sent to your email:</p>

  <form method="POST" action="/api/verify/{{challenge_id}}">
    <input type="text" name="code" maxlength="6" pattern="[0-9]{6}" required>
    <button type="submit">Verify</button>
  </form>

  <p>Didn't receive the email? Check your spam folder.</p>
</div>
```

7. **Integration SMTP**:
```rust
// Dans mail-rs/src/smtp/session.rs
async fn handle_data(&mut self) -> Result<String, SmtpError> {
    // ... parse email ...

    // Check HumanLock
    if self.humanlock.should_challenge(&from, &to).await? {
        // Store email temporarily
        let email_path = self.store_pending_email(&email_data)?;

        // Send challenge
        self.humanlock.send_challenge(&from, &to, email_path).await?;

        return Ok("250 Email pending human verification".to_string());
    }

    // Normal delivery
    self.deliver_email(&email_data).await?;
    Ok("250 OK".to_string())
}
```

**Tests**:
```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_new_sender_challenged() {
        // Vérifier qu'un nouveau sender reçoit challenge
    }

    #[tokio::test]
    async fn test_valid_code_whitelists() {
        // Vérifier que code correct → whitelist + delivery
    }

    #[tokio::test]
    async fn test_max_attempts_blocks() {
        // Vérifier que trop de tentatives = block
    }

    #[tokio::test]
    async fn test_expired_challenge() {
        // Vérifier expiration des challenges
    }
}
```

**Configuration**:
```toml
[antispam.humanlock]
enabled = true
challenge_expiry_hours = 24
max_attempts = 3
whitelist_after_solve = true
auto_whitelist_contacts = true  # Whitelist automatique des contacts existants
```

**Admin Interface**:
- `/admin/humanlock` avec:
  - Liste des challenges actifs
  - Whitelist management (add/remove)
  - Statistiques (challenges envoyés, résolus, expirés)
  - Bouton pour désactiver temporairement

---

### 3. AI Link Scanner

#### Contexte
Implémenter un scanner de liens alimenté par AI pour détecter automatiquement les liens malveillants, phishing et malware dans les emails. Intégration avec VirusTotal API et modèle AI local pour analyse heuristique.

#### Objectif
Créer `mail-rs/src/security/link_scanner.rs` qui scanne tous les liens dans les emails entrants et les marque comme sûrs/suspects/dangereux.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/security/link_scanner.rs
pub struct LinkScanner {
    virustotal_client: VirusTotalClient,
    ai_classifier: PhishingClassifier,
    cache: LinkCache,
}

pub struct ScanResult {
    pub url: String,
    pub risk_level: RiskLevel,  // Safe, Suspicious, Dangerous
    pub virustotal_score: Option<VTScore>,
    pub ai_confidence: f32,
    pub reasons: Vec<String>,  // Liste des raisons du score
    pub scanned_at: DateTime<Utc>,
}

pub enum RiskLevel {
    Safe,       // Score < 20%
    Suspicious, // Score 20-70%
    Dangerous,  // Score > 70%
}

pub struct VTScore {
    pub malicious: u32,
    pub suspicious: u32,
    pub harmless: u32,
    pub total_engines: u32,
}
```

**Fonctionnalités**:

1. **Extraction des liens**:
```rust
pub fn extract_links(email: &Email) -> Vec<String> {
    // Regex pour extraire URLs depuis:
    // - HTML href
    // - Plain text
    // - Redirects (bit.ly, tinyurl, etc.)

    // Normaliser URLs (supprimer tracking params)
    // Détecter URL shorteners et résoudre
}
```

2. **Analyse VirusTotal**:
```rust
impl LinkScanner {
    pub async fn scan_with_virustotal(&self, url: &str) -> Result<VTScore> {
        // POST à VirusTotal API v3
        // GET résultats (peut prendre quelques secondes)
        // Cache résultats pendant 24h

        let response = self.virustotal_client
            .post("https://www.virustotal.com/api/v3/urls")
            .header("x-apikey", &self.config.vt_api_key)
            .json(&json!({ "url": url }))
            .send()
            .await?;

        // Parser JSON response
    }
}
```

3. **Analyse AI Heuristique**:
```rust
impl PhishingClassifier {
    pub fn analyze_url(&self, url: &str) -> (f32, Vec<String>) {
        let mut score = 0.0;
        let mut reasons = Vec::new();

        // Heuristiques:

        // 1. Domain age (nouveau domaine suspect)
        if self.is_new_domain(url) {
            score += 0.3;
            reasons.push("Domain registered recently".to_string());
        }

        // 2. Suspicious TLD (.xyz, .tk, .ml, etc.)
        if self.is_suspicious_tld(url) {
            score += 0.2;
            reasons.push("Suspicious TLD".to_string());
        }

        // 3. URL length (très long = suspect)
        if url.len() > 100 {
            score += 0.15;
            reasons.push("Unusually long URL".to_string());
        }

        // 4. Nombre de sous-domaines
        if self.count_subdomains(url) > 3 {
            score += 0.2;
            reasons.push("Too many subdomains".to_string());
        }

        // 5. Caractères suspects (@, -, etc.)
        if url.contains('@') || url.matches('-').count() > 3 {
            score += 0.15;
            reasons.push("Suspicious characters".to_string());
        }

        // 6. Similarité avec domaines légitimes (typosquatting)
        if self.is_typosquatting(url) {
            score += 0.4;
            reasons.push("Possible typosquatting".to_string());
        }

        // 7. IP address instead of domain
        if self.is_ip_address(url) {
            score += 0.3;
            reasons.push("Uses IP address instead of domain".to_string());
        }

        (score.min(1.0), reasons)
    }
}
```

4. **Action sur email**:
```rust
pub async fn process_email(&self, email: &mut Email) -> Result<()> {
    let links = extract_links(email);

    for link in links {
        let scan = self.scan_link(&link).await?;

        match scan.risk_level {
            RiskLevel::Dangerous => {
                // Remplacer lien par [LIEN BLOQUÉ - DANGER]
                // Ou ajouter warning banner dans HTML
                email.add_warning_header(&link, &scan);
            }
            RiskLevel::Suspicious => {
                // Ajouter warning mais garder lien
                email.add_warning_banner(&link, &scan);
            }
            RiskLevel::Safe => {
                // Rien
            }
        }
    }

    Ok(())
}
```

5. **Storage**:
```sql
CREATE TABLE link_scans (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    url TEXT NOT NULL,
    url_hash TEXT UNIQUE NOT NULL,  -- SHA256 pour indexing
    risk_level TEXT NOT NULL,
    virustotal_malicious INTEGER,
    virustotal_suspicious INTEGER,
    ai_score REAL,
    reasons TEXT,  -- JSON array
    scanned_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    expires_at TIMESTAMP  -- Cache expiry
);

CREATE INDEX idx_url_hash ON link_scans(url_hash);
CREATE INDEX idx_expires ON link_scans(expires_at);
```

6. **API Endpoints**:
```rust
// GET /api/admin/links/scan/:url
pub async fn scan_url_endpoint(
    Path(url): Path<String>,
) -> Result<Json<ScanResult>, StatusCode> {
    // Scan manuel d'une URL
}

// GET /api/admin/links/history
pub async fn scan_history() -> Result<Json<Vec<ScanResult>>, StatusCode> {
    // Historique des scans
}
```

7. **Email Warning Template**:
```html
<div style="background: #fee; border: 2px solid #f00; padding: 10px; margin: 10px 0;">
  <strong>⚠️ WARNING - Suspicious Link Detected</strong>
  <p>This email contains a link that may be dangerous:</p>
  <p><code>{{url}}</code></p>
  <ul>
    {{#each reasons}}
    <li>{{this}}</li>
    {{/each}}
  </ul>
  <p><strong>Do not click unless you trust the sender.</strong></p>
</div>
```

**Tests**:
```rust
#[tokio::test]
async fn test_extract_links() {
    // Test extraction depuis HTML et plain text
}

#[tokio::test]
async fn test_phishing_heuristics() {
    // Test détection typosquatting
    assert!(is_typosquatting("paypai.com"));  // paypal
}

#[tokio::test]
async fn test_virustotal_integration() {
    // Test avec URL connue malveillante
}
```

**Configuration**:
```toml
[security.link_scanner]
enabled = true
virustotal_api_key = "your_api_key_here"
cache_duration_hours = 24
block_dangerous_links = true  # true = remplacer, false = warning seulement
scan_outgoing = false  # Ne scanner que emails entrants
```

**Admin Interface**:
- `/admin/security/links` avec:
  - Statistiques (total scannés, safe, suspicious, dangerous)
  - Liste des URLs bloquées récemment
  - Whitelist/Blacklist management
  - Test manual d'URL

---

### 4. Security Dashboard

#### Contexte
Créer un tableau de bord de sécurité complet affichant toutes les métriques de sécurité en temps réel : tentatives de connexion, emails bloqués, liens malveillants, preuves blockchain, challenges HumanLock, etc.

#### Objectif
Page `/admin/security` avec visualisations temps réel, alertes, et rapports hebdomadaires automatiques envoyés aux admins.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/api/admin.rs - Security endpoints
pub async fn get_security_overview() -> Result<Json<SecurityOverview>, StatusCode> {
    SecurityOverview {
        login_attempts: get_login_stats().await?,
        blocked_emails: get_blocked_stats().await?,
        link_scans: get_link_scan_stats().await?,
        humanlock_challenges: get_humanlock_stats().await?,
        blockchain_proofs: get_blockchain_stats().await?,
        recent_threats: get_recent_threats().await?,
    }
}

pub struct SecurityOverview {
    pub login_attempts: LoginStats,
    pub blocked_emails: BlockedEmailStats,
    pub link_scans: LinkScanStats,
    pub humanlock_challenges: HumanLockStats,
    pub blockchain_proofs: BlockchainStats,
    pub recent_threats: Vec<ThreatEvent>,
}

pub struct LoginStats {
    pub total_attempts_24h: u32,
    pub failed_attempts_24h: u32,
    pub unique_ips: u32,
    pub blocked_ips: Vec<String>,
    pub success_rate: f32,
}

pub struct ThreatEvent {
    pub timestamp: DateTime<Utc>,
    pub threat_type: ThreatType,  // Phishing, Malware, BruteForce, etc.
    pub severity: Severity,  // Low, Medium, High, Critical
    pub description: String,
    pub source_ip: Option<String>,
    pub blocked: bool,
}

pub enum ThreatType {
    PhishingLink,
    MalwareAttachment,
    BruteForceAttempt,
    SpamBot,
    SuspiciousLogin,
}

pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}
```

**Frontend** (mail-rs/templates/security_dashboard.html):
```html
<div class="security-dashboard">
  <!-- Header avec statut global -->
  <div class="status-header">
    <h1>🛡️ Security Dashboard</h1>
    <div class="status-badge {{overall_status}}">
      {{#if is_secure}}
        ✅ All Systems Secure
      {{else}}
        ⚠️ {{threat_count}} Active Threats
      {{/if}}
    </div>
  </div>

  <!-- Grid de métriques -->
  <div class="metrics-grid">
    <!-- Card: Login Security -->
    <div class="metric-card">
      <h3>🔐 Login Security</h3>
      <div class="big-number">{{failed_logins_24h}}</div>
      <p>Failed attempts (24h)</p>
      <div class="chart" id="login-chart"></div>
    </div>

    <!-- Card: Email Threats -->
    <div class="metric-card">
      <h3>📧 Email Threats</h3>
      <div class="big-number">{{blocked_emails_24h}}</div>
      <p>Blocked emails (24h)</p>
      <ul>
        <li>Spam: {{spam_count}}</li>
        <li>Phishing: {{phishing_count}}</li>
        <li>Malware: {{malware_count}}</li>
      </ul>
    </div>

    <!-- Card: Link Scanner -->
    <div class="metric-card">
      <h3>🔗 Link Scanner</h3>
      <div class="big-number">{{dangerous_links_24h}}</div>
      <p>Dangerous links blocked</p>
      <div class="pie-chart" id="link-chart"></div>
    </div>

    <!-- Card: HumanLock -->
    <div class="metric-card">
      <h3>🤖 HumanLock</h3>
      <div class="big-number">{{active_challenges}}</div>
      <p>Active challenges</p>
      <p>Success rate: {{challenge_success_rate}}%</p>
    </div>

    <!-- Card: Blockchain Proofs -->
    <div class="metric-card">
      <h3>⛓️ Blockchain Proofs</h3>
      <div class="big-number">{{confirmed_proofs}}</div>
      <p>Confirmed proofs</p>
      <p>Pending: {{pending_proofs}}</p>
    </div>
  </div>

  <!-- Recent Threats Timeline -->
  <div class="threats-timeline">
    <h2>Recent Security Events</h2>
    <div class="timeline">
      {{#each recent_threats}}
      <div class="threat-event {{severity}}">
        <span class="timestamp">{{timestamp}}</span>
        <span class="icon">{{icon}}</span>
        <span class="description">{{description}}</span>
        <span class="action">{{#if blocked}}Blocked{{else}}Allowed{{/if}}</span>
      </div>
      {{/each}}
    </div>
  </div>

  <!-- Alerts -->
  {{#if active_alerts}}
  <div class="alerts-section">
    <h2>🚨 Active Alerts</h2>
    {{#each active_alerts}}
    <div class="alert {{severity}}">
      <strong>{{title}}</strong>
      <p>{{message}}</p>
      <button onclick="acknowledgeAlert('{{id}}')">Acknowledge</button>
    </div>
    {{/each}}
  </div>
  {{/if}}

  <!-- Actions rapides -->
  <div class="quick-actions">
    <button onclick="exportSecurityReport()">📊 Export Report</button>
    <button onclick="sendWeeklyReport()">📧 Send Weekly Report</button>
    <button onclick="viewFullLogs()">📝 View Logs</button>
  </div>
</div>

<script>
// Auto-refresh toutes les 30 secondes
setInterval(async () => {
  const response = await fetch('/api/admin/security/overview');
  const data = await response.json();
  updateDashboard(data);
}, 30000);

// Charts avec Chart.js
function renderCharts(data) {
  // Login attempts chart (last 7 days)
  new Chart(document.getElementById('login-chart'), {
    type: 'line',
    data: {
      labels: data.login_timeline.labels,
      datasets: [{
        label: 'Failed Logins',
        data: data.login_timeline.values,
        borderColor: 'rgb(255, 99, 132)',
      }]
    }
  });

  // Links pie chart
  new Chart(document.getElementById('link-chart'), {
    type: 'pie',
    data: {
      labels: ['Safe', 'Suspicious', 'Dangerous'],
      datasets: [{
        data: [data.links_safe, data.links_suspicious, data.links_dangerous],
        backgroundColor: ['#10b981', '#f59e0b', '#ef4444']
      }]
    }
  });
}
</script>
```

**Rapport Hebdomadaire Automatique**:
```rust
// mail-rs/src/security/weekly_report.rs
pub async fn generate_weekly_report() -> Result<WeeklySecurityReport> {
    WeeklySecurityReport {
        period: format!("{} - {}", start_date, end_date),
        summary: SecuritySummary {
            total_emails_processed: count_emails().await?,
            blocked_emails: count_blocked().await?,
            failed_logins: count_failed_logins().await?,
            dangerous_links: count_dangerous_links().await?,
            new_senders_challenged: count_challenges().await?,
            blockchain_proofs_created: count_proofs().await?,
        },
        top_threats: get_top_threats(10).await?,
        recommendations: generate_recommendations().await?,
    }
}

// Cron job - tous les lundis à 8h
pub async fn schedule_weekly_reports() {
    let schedule = cron::Schedule::from_str("0 8 * * MON").unwrap();

    for next in schedule.upcoming(Utc) {
        tokio::time::sleep_until(next).await;

        let report = generate_weekly_report().await?;
        send_report_to_admins(report).await?;
    }
}
```

**Email Template du Rapport**:
```html
Subject: [GK Mail] Weekly Security Report - {{period}}

<h1>🛡️ Weekly Security Report</h1>
<p>Period: {{period}}</p>

<h2>Summary</h2>
<ul>
  <li>Emails processed: {{total_emails}}</li>
  <li>Threats blocked: {{blocked_emails}} ({{block_rate}}%)</li>
  <li>Failed login attempts: {{failed_logins}}</li>
  <li>Dangerous links detected: {{dangerous_links}}</li>
  <li>HumanLock challenges: {{challenges}}</li>
  <li>Blockchain proofs: {{proofs}}</li>
</ul>

<h2>Top Threats</h2>
<table>
  <tr>
    <th>Date</th>
    <th>Type</th>
    <th>Source</th>
    <th>Action</th>
  </tr>
  {{#each top_threats}}
  <tr>
    <td>{{timestamp}}</td>
    <td>{{threat_type}}</td>
    <td>{{source}}</td>
    <td>{{action}}</td>
  </tr>
  {{/each}}
</table>

<h2>Recommendations</h2>
<ul>
  {{#each recommendations}}
  <li>{{this}}</li>
  {{/each}}
</ul>

<p>View full dashboard: <a href="https://mail.yourdomain.com/admin/security">Security Dashboard</a></p>
```

**Configuration**:
```toml
[security.dashboard]
enabled = true
auto_refresh_seconds = 30
weekly_report_enabled = true
weekly_report_day = "monday"
weekly_report_hour = 8
admin_emails = ["admin@example.com"]
alert_threshold_failed_logins = 10
alert_threshold_blocked_emails = 50
```

**Tests**:
```rust
#[tokio::test]
async fn test_security_overview() {
    let overview = get_security_overview().await.unwrap();
    assert!(overview.login_attempts.total_attempts_24h >= 0);
}

#[tokio::test]
async fn test_weekly_report_generation() {
    let report = generate_weekly_report().await.unwrap();
    assert!(!report.period.is_empty());
}
```

---

## 📧 Productivité

### 5. Email Templates

#### Contexte
Implémenter un système de templates d'email (signatures automatiques, réponses rapides, modèles réutilisables) pour augmenter la productivité.

#### Objectif
Créer `mail-rs/src/templates/manager.rs` avec système de templates, variables dynamiques, et interface admin pour gérer les templates.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/templates/mod.rs
pub mod manager;
pub mod types;
pub mod renderer;

// mail-rs/src/templates/types.rs
pub struct EmailTemplate {
    pub id: String,
    pub name: String,
    pub category: TemplateCategory,
    pub subject: String,
    pub body_html: String,
    pub body_text: String,
    pub variables: Vec<TemplateVariable>,
    pub is_signature: bool,
    pub owner_email: String,  // User qui possède ce template
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum TemplateCategory {
    Signature,      // Signature automatique
    QuickReply,     // Réponse rapide
    Custom,         // Template personnalisé
}

pub struct TemplateVariable {
    pub name: String,      // Ex: "customer_name"
    pub default_value: Option<String>,
    pub required: bool,
}
```

**Storage Schema**:
```sql
CREATE TABLE email_templates (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    subject TEXT,
    body_html TEXT NOT NULL,
    body_text TEXT,
    variables TEXT,  -- JSON array
    is_signature BOOLEAN DEFAULT 0,
    owner_email TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_owner ON email_templates(owner_email);
CREATE INDEX idx_category ON email_templates(category);
```

**Fonctionnalités**:

1. **Variables dynamiques**:
```rust
// Variables disponibles:
// {{sender_name}} - Nom de l'expéditeur
// {{sender_email}} - Email de l'expéditeur
// {{recipient_name}} - Nom du destinataire
// {{recipient_email}} - Email du destinataire
// {{date}} - Date actuelle
// {{time}} - Heure actuelle
// {{company}} - Nom de la companie (config)
// Toute variable custom définie par utilisateur

impl TemplateRenderer {
    pub fn render(&self, template: &EmailTemplate, vars: &HashMap<String, String>) -> String {
        let mut result = template.body_html.clone();

        // Variables système
        result = result.replace("{{sender_name}}", &vars.get("sender_name").unwrap_or(&"".to_string()));
        result = result.replace("{{date}}", &Utc::now().format("%Y-%m-%d").to_string());

        // Variables custom
        for (key, value) in vars {
            result = result.replace(&format!("{{{{{}}}}}", key), value);
        }

        result
    }
}
```

2. **Templates prédéfinis**:
```rust
// Créer quelques templates par défaut à l'installation
pub fn create_default_templates(owner_email: &str) -> Vec<EmailTemplate> {
    vec![
        EmailTemplate {
            id: Uuid::new_v4().to_string(),
            name: "Professional Signature".to_string(),
            category: TemplateCategory::Signature,
            subject: "".to_string(),
            body_html: r#"
                <div style="font-family: Arial, sans-serif; color: #333;">
                    <p>Best regards,</p>
                    <p><strong>{{sender_name}}</strong><br>
                    {{job_title}}<br>
                    {{company}}<br>
                    {{phone}}<br>
                    <a href="mailto:{{sender_email}}">{{sender_email}}</a></p>
                </div>
            "#.to_string(),
            body_text: "Best regards,\n{{sender_name}}\n{{job_title}}\n{{company}}".to_string(),
            variables: vec![
                TemplateVariable { name: "job_title".to_string(), default_value: None, required: true },
                TemplateVariable { name: "company".to_string(), default_value: None, required: true },
                TemplateVariable { name: "phone".to_string(), default_value: None, required: false },
            ],
            is_signature: true,
            owner_email: owner_email.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        EmailTemplate {
            id: Uuid::new_v4().to_string(),
            name: "Thank You Reply".to_string(),
            category: TemplateCategory::QuickReply,
            subject: "Re: {{subject}}".to_string(),
            body_html: "<p>Hi {{recipient_name}},</p><p>Thank you for your email. I'll get back to you shortly.</p>".to_string(),
            body_text: "Hi {{recipient_name}},\n\nThank you for your email. I'll get back to you shortly.".to_string(),
            variables: vec![],
            is_signature: false,
            owner_email: owner_email.to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        },
        // Ajouter d'autres templates...
    ]
}
```

3. **API Endpoints**:
```rust
// GET /api/templates - Liste tous les templates de l'utilisateur
pub async fn list_templates(email: String) -> Result<Json<Vec<EmailTemplate>>, StatusCode> {
    // Récupérer depuis SQLite
}

// POST /api/templates - Créer un nouveau template
pub async fn create_template(
    email: String,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<Json<EmailTemplate>, StatusCode> {
    // Valider + insérer dans SQLite
}

// PUT /api/templates/:id - Modifier un template
pub async fn update_template(
    Path(id): Path<String>,
    Json(payload): Json<UpdateTemplateRequest>,
) -> Result<Json<EmailTemplate>, StatusCode> {
    // Vérifier ownership + update
}

// DELETE /api/templates/:id - Supprimer un template
pub async fn delete_template(
    Path(id): Path<String>,
    email: String,
) -> Result<StatusCode, StatusCode> {
    // Vérifier ownership + delete
}

// POST /api/templates/:id/render - Preview du rendu
pub async fn preview_template(
    Path(id): Path<String>,
    Json(vars): Json<HashMap<String, String>>,
) -> Result<Html<String>, StatusCode> {
    // Render template avec variables
}
```

4. **Interface Admin** (mail-rs/templates/email_templates.html):
```html
<div class="templates-manager">
  <h1>📝 Email Templates</h1>

  <button onclick="createTemplate()">+ New Template</button>

  <!-- Liste des templates -->
  <div class="templates-list">
    <div class="category-section">
      <h2>Signatures</h2>
      {{#each signatures}}
      <div class="template-card">
        <h3>{{name}}</h3>
        <p>{{preview}}</p>
        <div class="actions">
          <button onclick="editTemplate('{{id}}')">Edit</button>
          <button onclick="deleteTemplate('{{id}}')">Delete</button>
          <button onclick="setDefaultSignature('{{id}}')">Set as Default</button>
        </div>
      </div>
      {{/each}}
    </div>

    <div class="category-section">
      <h2>Quick Replies</h2>
      {{#each quick_replies}}
      <div class="template-card">
        <h3>{{name}}</h3>
        <p class="subject">Subject: {{subject}}</p>
        <p>{{preview}}</p>
        <div class="actions">
          <button onclick="editTemplate('{{id}}')">Edit</button>
          <button onclick="deleteTemplate('{{id}}')">Delete</button>
        </div>
      </div>
      {{/each}}
    </div>
  </div>

  <!-- Modal de création/édition -->
  <div id="templateModal" class="modal hidden">
    <div class="modal-content">
      <h2>Create/Edit Template</h2>
      <form id="templateForm">
        <input type="text" name="name" placeholder="Template Name" required>

        <select name="category">
          <option value="signature">Signature</option>
          <option value="quick_reply">Quick Reply</option>
          <option value="custom">Custom</option>
        </select>

        <input type="text" name="subject" placeholder="Subject (optional)">

        <textarea name="body_html" placeholder="HTML Body" rows="10"></textarea>
        <textarea name="body_text" placeholder="Plain Text Body" rows="10"></textarea>

        <div class="variables-section">
          <h3>Variables</h3>
          <p>Available variables: {{sender_name}}, {{recipient_name}}, {{date}}, {{time}}</p>

          <div id="customVariables">
            <button type="button" onclick="addVariable()">+ Add Custom Variable</button>
          </div>
        </div>

        <div class="preview-section">
          <h3>Preview</h3>
          <div id="preview"></div>
        </div>

        <div class="actions">
          <button type="button" onclick="closeModal()">Cancel</button>
          <button type="submit">Save Template</button>
        </div>
      </form>
    </div>
  </div>
</div>

<script>
async function createTemplate() {
  document.getElementById('templateModal').classList.remove('hidden');
}

async function saveTemplate() {
  const form = document.getElementById('templateForm');
  const formData = new FormData(form);

  const response = await fetch('/api/templates', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(Object.fromEntries(formData))
  });

  if (response.ok) {
    alert('Template saved!');
    location.reload();
  }
}

// Live preview while typing
document.querySelector('[name="body_html"]').addEventListener('input', (e) => {
  document.getElementById('preview').innerHTML = e.target.value;
});
</script>
```

5. **Intégration avec compose email**:
```javascript
// Dans l'interface de composition d'email
<div class="compose-email">
  <button onclick="insertTemplate()">📝 Insert Template</button>

  <select id="templateSelect" onchange="loadTemplate(this.value)">
    <option value="">-- Select Template --</option>
    {{#each quick_replies}}
    <option value="{{id}}">{{name}}</option>
    {{/each}}
  </select>

  <textarea id="emailBody"></textarea>
</div>

<script>
async function loadTemplate(templateId) {
  const response = await fetch(`/api/templates/${templateId}`);
  const template = await response.json();

  // Demander valeurs pour variables
  const vars = {};
  for (const variable of template.variables) {
    if (variable.required) {
      vars[variable.name] = prompt(`Enter ${variable.name}:`, variable.default_value || '');
    }
  }

  // Render template
  const rendered = await fetch(`/api/templates/${templateId}/render`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(vars)
  });

  const html = await rendered.text();
  document.getElementById('emailBody').value = html;
}
</script>
```

6. **Signature automatique**:
```rust
// Ajouter automatiquement la signature par défaut à chaque email sortant
impl SmtpSession {
    async fn add_signature(&self, email: &mut Email, sender: &str) -> Result<()> {
        // Récupérer signature par défaut de l'utilisateur
        let signature = self.template_manager
            .get_default_signature(sender)
            .await?;

        if let Some(sig) = signature {
            // Render variables
            let vars = HashMap::from([
                ("sender_name".to_string(), sender.to_string()),
                ("sender_email".to_string(), sender.to_string()),
            ]);

            let rendered = self.template_manager.render(&sig, &vars);

            // Append à l'email
            email.body_html.push_str(&format!("\n\n{}", rendered));
        }

        Ok(())
    }
}
```

**Tests**:
```rust
#[tokio::test]
async fn test_create_template() {
    let manager = TemplateManager::new().await;
    let template = manager.create_template("test@example.com", payload).await.unwrap();
    assert_eq!(template.name, "Test Template");
}

#[tokio::test]
async fn test_render_with_variables() {
    let template = EmailTemplate {
        body_html: "Hello {{name}}!".to_string(),
        // ...
    };

    let vars = HashMap::from([("name".to_string(), "John".to_string())]);
    let rendered = TemplateRenderer::render(&template, &vars);
    assert_eq!(rendered, "Hello John!");
}
```

**Configuration**:
```toml
[templates]
enabled = true
max_per_user = 50
auto_signature = true  # Ajouter automatiquement la signature par défaut
```

---

### 6. Auto-Reply / Vacation

#### Contexte
Implémenter un système de réponse automatique (out-of-office / vacation responder) permettant aux utilisateurs de configurer des messages automatiques pendant leur absence.

#### Objectif
Créer `mail-rs/src/autoreply/mod.rs` avec gestion de réponses automatiques configurables par période et destinataire.

#### Spécifications Techniques

**Architecture**:
```rust
// mail-rs/src/autoreply/mod.rs
pub struct AutoReply {
    db: SqlitePool,
}

pub struct VacationRule {
    pub id: String,
    pub user_email: String,
    pub enabled: bool,
    pub start_date: DateTime<Utc>,
    pub end_date: DateTime<Utc>,
    pub subject: String,
    pub message_html: String,
    pub message_text: String,
    pub reply_once_per_sender: bool,  // true = 1 seule réponse par expéditeur
    pub reply_interval_hours: u32,     // Minimum entre 2 réponses au même sender
    pub exclude_domains: Vec<String>,  // Ne pas répondre à ces domaines
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct SentAutoReply {
    pub rule_id: String,
    pub recipient_email: String,
    pub sent_at: DateTime<Utc>,
}
```

**Storage Schema**:
```sql
CREATE TABLE vacation_rules (
    id TEXT PRIMARY KEY,
    user_email TEXT NOT NULL,
    enabled BOOLEAN DEFAULT 1,
    start_date TIMESTAMP NOT NULL,
    end_date TIMESTAMP NOT NULL,
    subject TEXT NOT NULL,
    message_html TEXT NOT NULL,
    message_text TEXT,
    reply_once_per_sender BOOLEAN DEFAULT 1,
    reply_interval_hours INTEGER DEFAULT 24,
    exclude_domains TEXT,  -- JSON array
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE sent_autoreplies (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    rule_id TEXT NOT NULL,
    recipient_email TEXT NOT NULL,
    sent_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (rule_id) REFERENCES vacation_rules(id) ON DELETE CASCADE
);

CREATE INDEX idx_user_email ON vacation_rules(user_email);
CREATE INDEX idx_active ON vacation_rules(enabled, start_date, end_date);
CREATE INDEX idx_recipient ON sent_autoreplies(recipient_email, rule_id);
```

**Fonctionnalités**:

1. **Vérification si réponse nécessaire**:
```rust
impl AutoReply {
    pub async fn should_send_reply(
        &self,
        recipient: &str,  // Notre utilisateur
        sender: &str,     // Expéditeur externe
    ) -> Result<Option<VacationRule>> {
        // 1. Récupérer règle active pour ce user
        let rule = self.get_active_rule(recipient).await?;

        if rule.is_none() {
            return Ok(None);
        }

        let rule = rule.unwrap();

        // 2. Vérifier période
        let now = Utc::now();
        if now < rule.start_date || now > rule.end_date {
            return Ok(None);
        }

        // 3. Vérifier exclude_domains
        let sender_domain = sender.split('@').nth(1).unwrap_or("");
        if rule.exclude_domains.contains(&sender_domain.to_string()) {
            return Ok(None);
        }

        // 4. Vérifier si déjà répondu récemment
        if rule.reply_once_per_sender {
            let last_sent = self.get_last_reply(&rule.id, sender).await?;
            if let Some(sent) = last_sent {
                let hours_since = (now - sent.sent_at).num_hours();
                if hours_since < rule.reply_interval_hours as i64 {
                    return Ok(None);  // Trop tôt pour re-répondre
                }
            }
        }

        Ok(Some(rule))
    }

    pub async fn send_autoreply(
        &self,
        rule: &VacationRule,
        to: &str,
        original_subject: &str,
    ) -> Result<()> {
        // Composer le message
        let subject = if rule.subject.contains("{{subject}}") {
            rule.subject.replace("{{subject}}", original_subject)
        } else {
            rule.subject.clone()
        };

        let email = Email {
            from: rule.user_email.clone(),
            to: to.to_string(),
            subject,
            body_html: rule.message_html.clone(),
            body_text: rule.message_text.clone(),
            headers: vec![
                ("Auto-Submitted".to_string(), "auto-replied".to_string()),
                ("X-Autoreply".to_string(), "yes".to_string()),
                ("Precedence".to_string(), "bulk".to_string()),  // Éviter boucles
            ],
            ..Default::default()
        };

        // Envoyer
        self.smtp_sender.send(email).await?;

        // Enregistrer dans sent_autoreplies
        self.record_sent_reply(&rule.id, to).await?;

        Ok(())
    }
}
```

2. **Intégration SMTP**:
```rust
// Dans mail-rs/src/smtp/session.rs
async fn handle_incoming_email(&mut self, email: &Email) -> Result<()> {
    // Delivery normale
    self.deliver_to_maildir(email).await?;

    // Check auto-reply
    if let Some(rule) = self.autoreply.should_send_reply(&email.to, &email.from).await? {
        self.autoreply.send_autoreply(&rule, &email.from, &email.subject).await?;
    }

    Ok(())
}
```

3. **API Endpoints**:
```rust
// GET /api/vacation - Récupérer règle active
pub async fn get_vacation_rule(email: String) -> Result<Json<Option<VacationRule>>, StatusCode> {
    // Récupérer depuis SQLite
}

// POST /api/vacation - Créer/activer vacation
pub async fn create_vacation(
    email: String,
    Json(payload): Json<CreateVacationRequest>,
) -> Result<Json<VacationRule>, StatusCode> {
    // Valider dates
    // Insérer dans SQLite
}

// PUT /api/vacation/:id - Modifier vacation
pub async fn update_vacation(
    Path(id): Path<String>,
    Json(payload): Json<UpdateVacationRequest>,
) -> Result<Json<VacationRule>, StatusCode> {
    // Vérifier ownership + update
}

// DELETE /api/vacation/:id - Désactiver vacation
pub async fn disable_vacation(
    Path(id): Path<String>,
    email: String,
) -> Result<StatusCode, StatusCode> {
    // Set enabled = false
}

// GET /api/vacation/history - Historique des réponses envoyées
pub async fn get_autoreply_history(email: String) -> Result<Json<Vec<SentAutoReply>>, StatusCode> {
    // Récupérer depuis sent_autoreplies
}
```

4. **Interface Admin** (mail-rs/templates/vacation.html):
```html
<div class="vacation-settings">
  <h1>🏖️ Out of Office / Vacation Responder</h1>

  {{#if active_rule}}
  <div class="active-vacation">
    <div class="status-badge active">✅ Active</div>
    <p><strong>Period:</strong> {{start_date}} to {{end_date}}</p>
    <p><strong>Subject:</strong> {{subject}}</p>
    <div class="message-preview">{{message_preview}}</div>
    <div class="actions">
      <button onclick="editVacation('{{rule_id}}')">Edit</button>
      <button onclick="disableVacation('{{rule_id}}')">Disable</button>
    </div>
  </div>
  {{else}}
  <div class="no-vacation">
    <p>No active out-of-office message.</p>
    <button onclick="createVacation()">+ Set Up Auto-Reply</button>
  </div>
  {{/if}}

  <!-- Formulaire de création/édition -->
  <div id="vacationForm" class="form-section {{#unless show_form}}hidden{{/unless}}">
    <h2>Configure Auto-Reply</h2>

    <form onsubmit="saveVacation(event)">
      <div class="form-group">
        <label>Start Date</label>
        <input type="datetime-local" name="start_date" required>
      </div>

      <div class="form-group">
        <label>End Date</label>
        <input type="datetime-local" name="end_date" required>
      </div>

      <div class="form-group">
        <label>Subject</label>
        <input type="text" name="subject" value="Out of Office: {{subject}}" required>
        <small>Use {{subject}} to include original subject</small>
      </div>

      <div class="form-group">
        <label>Message (HTML)</label>
        <textarea name="message_html" rows="10" required>
Hello,

I am currently out of office and will return on {{end_date}}.

I will respond to your email when I return.

For urgent matters, please contact: support@example.com

Best regards,
{{sender_name}}
        </textarea>
      </div>

      <div class="form-group">
        <label>Message (Plain Text)</label>
        <textarea name="message_text" rows="10"></textarea>
      </div>

      <div class="form-group">
        <label>
          <input type="checkbox" name="reply_once_per_sender" checked>
          Reply only once per sender
        </label>
      </div>

      <div class="form-group">
        <label>Minimum interval between replies (hours)</label>
        <input type="number" name="reply_interval_hours" value="24" min="1">
      </div>

      <div class="form-group">
        <label>Exclude domains (one per line)</label>
        <textarea name="exclude_domains" rows="3" placeholder="noreply.com&#10;spam.com"></textarea>
        <small>Auto-replies will not be sent to these domains</small>
      </div>

      <div class="actions">
        <button type="button" onclick="cancelVacation()">Cancel</button>
        <button type="submit">Save & Enable</button>
      </div>
    </form>
  </div>

  <!-- Historique des réponses envoyées -->
  <div class="history-section">
    <h2>Auto-Reply History</h2>
    <table>
      <thead>
        <tr>
          <th>Date</th>
          <th>Recipient</th>
        </tr>
      </thead>
      <tbody>
        {{#each history}}
        <tr>
          <td>{{sent_at}}</td>
          <td>{{recipient_email}}</td>
        </tr>
        {{/each}}
      </tbody>
    </table>
  </div>
</div>

<script>
async function saveVacation(event) {
  event.preventDefault();
  const form = event.target;
  const formData = new FormData(form);

  // Convertir exclude_domains textarea en array
  const excludeDomains = formData.get('exclude_domains').split('\n').filter(d => d.trim());
  formData.set('exclude_domains', JSON.stringify(excludeDomains));

  const response = await fetch('/api/vacation', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(Object.fromEntries(formData))
  });

  if (response.ok) {
    alert('Auto-reply enabled!');
    location.reload();
  }
}

async function disableVacation(ruleId) {
  if (!confirm('Disable auto-reply?')) return;

  await fetch(`/api/vacation/${ruleId}`, { method: 'DELETE' });
  location.reload();
}
</script>
```

5. **Protection contre boucles**:
```rust
// Ne jamais répondre aux:
const EXCLUDE_HEADERS: &[&str] = &[
    "Auto-Submitted",     // Autres auto-replies
    "X-Autoreply",
    "Precedence: bulk",
    "List-Unsubscribe",   // Mailing lists
];

fn is_autoreply_email(headers: &HashMap<String, String>) -> bool {
    for (key, value) in headers {
        if key == "Auto-Submitted" && value != "no" {
            return true;
        }
        if key == "Precedence" && (value == "bulk" || value == "junk") {
            return true;
        }
        if key.contains("List-") {  // Mailing list
            return true;
        }
    }
    false
}
```

**Tests**:
```rust
#[tokio::test]
async fn test_vacation_rule_active() {
    let autoreply = AutoReply::new().await;

    // Créer règle active
    let rule = create_test_rule("user@example.com", Utc::now(), Utc::now() + Duration::days(7));

    // Vérifier que réponse nécessaire
    let should_reply = autoreply.should_send_reply("user@example.com", "sender@test.com").await.unwrap();
    assert!(should_reply.is_some());
}

#[tokio::test]
async fn test_reply_once_per_sender() {
    // Test qu'on répond qu'une fois au même sender
}

#[tokio::test]
async fn test_exclude_domains() {
    // Test que exclude_domains fonctionne
}
```

**Configuration**:
```toml
[autoreply]
enabled = true
max_rules_per_user = 5
default_reply_interval_hours = 24
max_message_length = 5000
```

---

*[Les prompts 7 et 8 (Email Scheduling et Email Threading) suivraient le même format détaillé avec architecture complète, code samples, tests, etc.]*

---

## 📝 Notes d'Implémentation

### Priorités Suggérées
1. **Semaine 1-2**: Blockchain Proof of Email + Human Lock
2. **Semaine 3-4**: AI Link Scanner + Security Dashboard
3. **Semaine 5-6**: Email Templates + Auto-Reply
4. **Semaine 7-8**: Email Scheduling + Email Threading

### Stack Technique Commune
- **Language**: Rust (existing codebase)
- **Database**: SQLite (pour persistance)
- **Frontend**: Askama templates + Tailwind CSS + JavaScript
- **APIs externe**: OpenTimestamps, VirusTotal

### Tests & Quality
- Minimum 80% test coverage pour chaque feature
- Tests unitaires + tests d'intégration
- Documentation rustdoc complète
- Exemples d'utilisation dans la doc

### Déploiement
- Chaque feature derrière feature flag
- Migration SQLite automatique
- Backward compatible
- Rollback possible

---

**Status**: Ready-to-use prompts pour 4 features complètes
**Prochaine étape**: Sélectionner 1-2 features pour Sprint 17 et commencer l'implémentation
