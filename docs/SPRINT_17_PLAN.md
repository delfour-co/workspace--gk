# Sprint 17 Plan - IMAP Complete + Productivity First

> Phase 3 Kickoff - Focus sur compatibilité clients email + expérience utilisateur

## 🎯 Objectifs du Sprint

**Durée** : 4 semaines (Décembre 2024 - Janvier 2025)

**Priorités** :
1. 📮 **IMAP Complete** - Support complet des clients email standards
2. 📧 **Productivité** - Templates + Auto-Reply pour usage professionnel

**Résultat attendu** :
- Thunderbird, Apple Mail, Outlook 100% fonctionnels
- Expérience utilisateur moderne et productive
- Feature parity avec Gmail/Outlook pour productivité
- Système prêt pour adoption utilisateur réel

## 🎯 Pourquoi ce choix ?

**IMAP Complete d'abord** :
✅ Permet d'utiliser n'importe quel client email standard
✅ Essentiel pour adoption utilisateur
✅ Pas de dépendance externe
✅ Testable facilement
✅ Impact utilisateur immédiat

**Productivité ensuite** :
✅ Features très demandées (templates, vacation)
✅ Essentiel pour usage professionnel
✅ Différentiateur vs serveurs basiques
✅ UX moderne

**Blockchain → Plus tard** :
⏸️ Innovation cool mais pas essentiel
⏸️ Peut attendre Sprint 18-19
⏸️ Mieux après avoir des vrais utilisateurs

---

## 📦 Feature 1 : IMAP Complete (Semaines 1-2)

### Contexte Actuel

**État actuel IMAP** (mail-rs/src/imap/):
- ✅ SELECT (sélectionner mailbox)
- ✅ FETCH (lire messages)
- ✅ SEARCH (chercher messages)
- ✅ STATUS (info mailbox)
- ❌ STORE (modifier flags)
- ❌ COPY (copier messages)
- ❌ EXPUNGE (supprimer définitivement)
- ❌ IDLE (push notifications)

**Ce qui manque pour clients standards** :
- Marquer comme lu/non-lu (STORE \\Seen)
- Supprimer emails (STORE \\Deleted + EXPUNGE)
- Marquer important (STORE \\Flagged)
- Copier vers dossiers (COPY)
- Push notifications temps réel (IDLE)

### Objectif

Implémenter toutes les commandes IMAP write manquantes pour rendre Thunderbird, Apple Mail, et autres clients 100% fonctionnels.

### Architecture Technique

**Fichiers à modifier** :
```
mail-rs/src/imap/
├── commands.rs      - Ajouter STORE, COPY, EXPUNGE
├── session.rs       - Handler pour nouvelles commandes
├── mailbox.rs       - Logique maildir (flags, copy, delete)
└── idle.rs          - Nouveau fichier pour IDLE support
```

**RFC à suivre** :
- RFC 3501 - IMAP4rev1 (base)
- RFC 2177 - IDLE command
- RFC 4551 - CONDSTORE (optionnel pour optimisation)

### Tasks Breakdown

#### Semaine 1 : Write Operations

**Jour 1-2 : STORE command**
- [ ] Parser `STORE <seq> +FLAGS (\Seen \Flagged \Deleted)`
- [ ] Implémenter modification flags dans maildir
  - [ ] Renommer fichier maildir avec flags (`:2,S` pour Seen, `:2,F` pour Flagged)
  - [ ] Support `:2,FS` (multiple flags)
- [ ] Tests STORE avec Thunderbird
- **Deliverable** : Marquer lu/non-lu fonctionne

**Code example** :
```rust
// mail-rs/src/imap/commands.rs
pub async fn handle_store(
    &mut self,
    sequence_set: &str,
    flags_action: FlagsAction,  // Add, Remove, Replace
    flags: Vec<Flag>,
) -> Result<String> {
    // 1. Parser sequence set (1:*, 1,2,3, etc.)
    let messages = self.mailbox.get_messages_by_sequence(sequence_set)?;

    // 2. Pour chaque message
    for msg in messages {
        match flags_action {
            FlagsAction::Add => self.mailbox.add_flags(&msg.filename, &flags)?,
            FlagsAction::Remove => self.mailbox.remove_flags(&msg.filename, &flags)?,
            FlagsAction::Replace => self.mailbox.set_flags(&msg.filename, &flags)?,
        }
    }

    // 3. Retourner confirmation
    Ok("* OK STORE completed\r\n")
}

// mail-rs/src/imap/mailbox.rs
impl Mailbox {
    pub fn add_flags(&mut self, filename: &str, flags: &[Flag]) -> Result<()> {
        // Lire flags actuels depuis filename (/path/to/email:2,S)
        let current_flags = self.parse_flags_from_filename(filename)?;

        // Merger avec nouveaux flags
        let new_flags = current_flags.union(flags);

        // Renommer fichier avec nouveaux flags
        let new_filename = self.filename_with_flags(filename, &new_flags);
        fs::rename(filename, new_filename)?;

        Ok(())
    }

    fn filename_with_flags(&self, base: &str, flags: &HashSet<Flag>) -> String {
        // Convertir flags en format maildir ":2,DFRS"
        // D = Draft, F = Flagged, R = Replied, S = Seen
        let mut flag_str = String::from(":2,");

        if flags.contains(&Flag::Draft) { flag_str.push('D'); }
        if flags.contains(&Flag::Flagged) { flag_str.push('F'); }
        if flags.contains(&Flag::Answered) { flag_str.push('R'); }
        if flags.contains(&Flag::Seen) { flag_str.push('S'); }

        // Remplacer l'ancien suffix de flags
        let parts: Vec<&str> = base.split(":2,").collect();
        format!("{}{}",  parts[0], flag_str)
    }
}
```

**Jour 3-4 : COPY command**
- [ ] Parser `COPY <seq> <destination_mailbox>`
- [ ] Implémenter copie fichier maildir
  - [ ] Hard link si même filesystem
  - [ ] Copy si différent filesystem
- [ ] Gérer dossiers (INBOX, Sent, Trash, etc.)
- [ ] Tests COPY
- **Deliverable** : Copier vers dossiers fonctionne

**Code example** :
```rust
pub async fn handle_copy(
    &mut self,
    sequence_set: &str,
    dest_mailbox: &str,
) -> Result<String> {
    let messages = self.mailbox.get_messages_by_sequence(sequence_set)?;

    // Obtenir chemin destination
    let dest_path = self.mailbox.get_mailbox_path(dest_mailbox)?;

    for msg in messages {
        // Générer nouveau nom fichier (nouveau timestamp)
        let new_filename = format!(
            "{}.{}.{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
            rand::random::<u32>(),
            hostname()
        );

        let dest_file = dest_path.join("cur").join(&new_filename);

        // Hard link si possible (plus efficace)
        if hard_link(&msg.path, &dest_file).is_err() {
            // Sinon copie
            fs::copy(&msg.path, &dest_file)?;
        }
    }

    Ok("* OK COPY completed\r\n")
}
```

**Jour 5 : EXPUNGE command**
- [ ] Parser `EXPUNGE`
- [ ] Supprimer tous messages avec flag `\Deleted`
- [ ] Retourner liste des messages supprimés
- [ ] Tests EXPUNGE
- **Deliverable** : Suppression définitive fonctionne

**Code example** :
```rust
pub async fn handle_expunge(&mut self) -> Result<String> {
    let mut response = String::new();
    let messages = self.mailbox.get_all_messages()?;

    let mut delete_count = 0;
    for (seq, msg) in messages.iter().enumerate() {
        if msg.flags.contains(&Flag::Deleted) {
            // Supprimer le fichier
            fs::remove_file(&msg.path)?;

            // Notifier client
            response.push_str(&format!("* {} EXPUNGE\r\n", seq + 1));
            delete_count += 1;
        }
    }

    response.push_str(&format!("* OK {} messages expunged\r\n", delete_count));
    Ok(response)
}
```

#### Semaine 2 : IDLE + Polish

**Jour 1-3 : IDLE command**
- [ ] Créer `mail-rs/src/imap/idle.rs`
- [ ] Implémenter IDLE (RFC 2177)
  - [ ] Entrer en mode IDLE
  - [ ] Watcher filesystem pour nouveaux emails
  - [ ] Notifier client si nouveau message
  - [ ] Sortir IDLE sur commande DONE
- [ ] Tests avec inotify/fswatch
- **Deliverable** : Push notifications fonctionnent

**Code example** :
```rust
// mail-rs/src/imap/idle.rs
use notify::{Watcher, RecursiveMode, RawEvent};

pub struct IdleWatcher {
    watcher: RecommendedWatcher,
    rx: Receiver<RawEvent>,
}

impl IdleWatcher {
    pub fn new(maildir_path: &Path) -> Result<Self> {
        let (tx, rx) = channel();
        let mut watcher = notify::watcher(tx, Duration::from_secs(1))?;
        watcher.watch(maildir_path.join("new"), RecursiveMode::NonRecursive)?;

        Ok(Self { watcher, rx })
    }

    pub async fn wait_for_changes(&self) -> Result<Vec<String>> {
        // Attendre événement filesystem
        match self.rx.recv() {
            Ok(event) => {
                // Nouveau fichier dans /new
                Ok(vec!["* EXISTS".to_string()])
            }
            Err(_) => Ok(vec![]),
        }
    }
}

// Dans session.rs
pub async fn handle_idle(&mut self) -> Result<()> {
    // Envoyer confirmation
    self.write("+ idling\r\n").await?;

    let watcher = IdleWatcher::new(&self.mailbox.path)?;

    loop {
        tokio::select! {
            // Nouveau message détecté
            changes = watcher.wait_for_changes() => {
                for change in changes? {
                    self.write(&format!("{}\r\n", change)).await?;
                }
            }

            // Client envoie DONE
            line = self.read_line() => {
                if line?.trim() == "DONE" {
                    break;
                }
            }

            // Timeout après 29 minutes (RFC recommande < 30min)
            _ = tokio::time::sleep(Duration::from_secs(29 * 60)) => {
                self.write("* OK IDLE timeout\r\n").await?;
                break;
            }
        }
    }

    self.write("OK IDLE terminated\r\n").await?;
    Ok(())
}
```

**Jour 4-5 : Tests & Polish**
- [ ] Tests complets avec Thunderbird
  - [ ] Marquer lu/non-lu
  - [ ] Supprimer emails
  - [ ] Copier vers dossiers
  - [ ] Push notifications
- [ ] Tests avec Apple Mail
- [ ] Tests avec autres clients (Outlook, K-9 Mail)
- [ ] Documentation IMAP compliance
- **Deliverable** : Tous clients fonctionnent parfaitement

### Definition of Done (IMAP Complete)

- [ ] ✅ STORE command implémenté (flags: Seen, Flagged, Deleted, Answered)
- [ ] ✅ COPY command implémenté (copie vers dossiers)
- [ ] ✅ EXPUNGE command implémenté (suppression définitive)
- [ ] ✅ IDLE command implémenté (push notifications)
- [ ] ✅ Thunderbird fonctionne 100%
- [ ] ✅ Apple Mail fonctionne 100%
- [ ] ✅ Tests automatisés pour toutes commandes
- [ ] ✅ Documentation RFC compliance
- [ ] ✅ 0 regressions sur IMAP read-only existant

### Risques & Mitigation

**Risque 1** : Flags maildir mal gérés (corruption)
- **Mitigation** : Tests exhaustifs, validation format
- **Impact** : Critique - doit être parfait

**Risque 2** : IDLE consomme trop de resources
- **Mitigation** : Timeout 29min, limite connexions simultanées
- **Impact** : Moyen - gérable

**Risque 3** : Compatibilité clients email variés
- **Mitigation** : Tests avec top 3 clients (Thunderbird, Apple Mail, Outlook)
- **Impact** : Moyen - itérer selon feedback

---

## 📦 Feature 2 : Email Templates (Semaine 3)

### Objectif

Système de templates d'email avec :
- Signatures automatiques
- Réponses rapides (quick replies)
- Templates personnalisés
- Variables dynamiques ({{sender_name}}, {{date}}, etc.)

### Architecture Technique

**Modules à créer** :
```
mail-rs/src/templates/
├── mod.rs           - Module exports
├── types.rs         - EmailTemplate, TemplateVariable
├── manager.rs       - CRUD templates
└── renderer.rs      - Variable substitution
```

**Spec complète disponible** :
Voir `docs/FEATURES_PROMPTS.md` section "Email Templates" pour specs détaillées (1,200 lignes).

### Tasks Breakdown

**Jour 1-2 : Core Implementation**
- [ ] Créer modules `templates/`
- [ ] Implémenter types (EmailTemplate, TemplateVariable)
- [ ] Schema SQLite (`email_templates`)
- [ ] CRUD API (create, read, update, delete)
- [ ] Renderer avec variables ({{var}})
- **Deliverable** : Backend templates fonctionnel

**Jour 3-4 : Admin UI**
- [ ] Template `mail-rs/templates/email_templates.html`
- [ ] Liste templates (signatures, quick replies, custom)
- [ ] Modal création/édition template
- [ ] Preview temps réel
- [ ] Gestion variables custom
- **Deliverable** : UI admin complète

**Jour 5 : Integration**
- [ ] Signature automatique sur emails sortants
- [ ] Bouton "Insert Template" dans compose
- [ ] Templates par défaut (Professional Signature, Thank You, etc.)
- [ ] Tests end-to-end
- **Deliverable** : Feature complète et testée

### Definition of Done

- [ ] ✅ CRUD templates fonctionnel (API + UI)
- [ ] ✅ Variables dynamiques fonctionnent
- [ ] ✅ Signature auto ajoutée aux emails
- [ ] ✅ 5+ templates par défaut fournis
- [ ] ✅ UI intuitive et rapide
- [ ] ✅ Tests unitaires + intégration
- [ ] ✅ Documentation utilisateur

---

## 📦 Feature 3 : Auto-Reply / Vacation (Semaine 4)

### Objectif

Système de réponse automatique (out-of-office) avec :
- Configuration par période (dates début/fin)
- Message personnalisable
- Reply once per sender (éviter spam)
- Exclude domains (ne pas répondre aux newsletters)

### Architecture Technique

**Modules à créer** :
```
mail-rs/src/autoreply/
├── mod.rs           - Module exports
├── types.rs         - VacationRule, SentAutoReply
└── manager.rs       - Auto-reply logic
```

**Spec complète disponible** :
Voir `docs/FEATURES_PROMPTS.md` section "Auto-Reply / Vacation" pour specs détaillées (1,300 lignes).

### Tasks Breakdown

**Jour 1-2 : Core Implementation**
- [ ] Créer modules `autoreply/`
- [ ] Implémenter types (VacationRule)
- [ ] Schema SQLite (`vacation_rules`, `sent_autoreplies`)
- [ ] Logique should_send_reply()
- [ ] Protection anti-boucles (headers Auto-Submitted, etc.)
- **Deliverable** : Backend auto-reply fonctionnel

**Jour 3-4 : SMTP Integration + UI**
- [ ] Hook dans `smtp/session.rs` après réception
- [ ] Envoyer auto-reply si conditions remplies
- [ ] Template email auto-reply professionnel
- [ ] Page admin `/admin/vacation`
- [ ] Formulaire configuration (dates, message, options)
- **Deliverable** : Integration SMTP + UI admin

**Jour 5 : Tests & Polish**
- [ ] Tests anti-boucles (ne jamais répondre aux auto-replies)
- [ ] Tests reply once per sender
- [ ] Tests expiration règles
- [ ] Tests exclude domains
- [ ] Documentation utilisateur
- **Deliverable** : Feature production-ready

### Definition of Done

- [ ] ✅ Configuration vacation via UI admin
- [ ] ✅ Auto-reply envoyé automatiquement
- [ ] ✅ Reply once per sender fonctionne
- [ ] ✅ 0 boucles infinies (tests exhaustifs)
- [ ] ✅ Exclude domains fonctionne
- [ ] ✅ Message personnalisable (HTML + text)
- [ ] ✅ Tests anti-boucles passent 100%
- [ ] ✅ Documentation utilisateur complète

---

## 📊 Timeline Globale

```
Semaine 1 (Déc 9-15)
├─ IMAP: STORE, COPY, EXPUNGE
└─ Tests clients email

Semaine 2 (Déc 16-22)
├─ IMAP: IDLE implementation
├─ Tests Thunderbird/Apple Mail
└─ IMAP Complete ✅

Semaine 3 (Déc 23-29)
├─ Email Templates core + UI
└─ Email Templates ✅

Semaine 4 (Déc 30 - Jan 5)
├─ Auto-Reply implementation
├─ Tests anti-boucles
└─ Auto-Reply ✅

Sprint Review (Jan 6)
├─ Demo avec Thunderbird
├─ Demo templates + vacation
└─ Planning Sprint 18
```

## 🎯 Success Metrics

**IMAP Complete**
- [ ] Thunderbird 100% fonctionnel (read + write)
- [ ] Apple Mail 100% fonctionnel
- [ ] 0 crashes sur 1000 operations
- [ ] Push notifications < 1s latence

**Email Templates**
- [ ] 5+ templates par défaut créés
- [ ] Signature auto sur 100% emails sortants
- [ ] UI création template < 2min
- [ ] 0 bugs variables substitution

**Auto-Reply**
- [ ] 0 boucles infinies (critique)
- [ ] Reply sent < 1min après réception
- [ ] Exclude domains 100% respecté
- [ ] UI configuration < 3min

**Global Sprint**
- [ ] 175+ tests → 210+ tests
- [ ] 0 regressions SMTP/IMAP
- [ ] Documentation complète (3 features)
- [ ] CI/CD vert

## 🚀 Post-Sprint 17

**Sprint 18 Preview** (Janvier 2025):
- Email Scheduling (envoi différé)
- Email Threading (conversations groupées)
- Full-text search (Tantivy)

**Milestone** :
- Fin Sprint 17 → Système utilisable quotidiennement
- Feature parity avec Gmail/Outlook pour basiques
- Prêt pour beta testing avec vrais utilisateurs

**Sprint 19 Preview** (Février 2025):
- AI Link Scanner (détection phishing)
- Security Dashboard
- Blockchain Proof (si demandé)

## 📋 Notes

**Dependencies** :
- `notify` crate pour IDLE filesystem watching
- `askama` déjà présent pour templates
- Aucune dépendance externe (APIs)

**Compatibilité** :
- Backward compatible (IMAP read existant conservé)
- Pas de breaking changes
- Feature flags optionnelles

**Performance** :
- STORE/COPY/EXPUNGE : O(n) avec n = nombre messages
- IDLE : 0 overhead sauf polling filesystem
- Templates : < 10ms rendering
- Auto-reply : async, 0 impact delivery

---

**Status** : Ready to start ✅
**Prochaine action** : Créer branche `feature/sprint-17-imap-productivity` et commencer IMAP STORE
