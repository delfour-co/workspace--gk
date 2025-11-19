# Règles et Conventions du Projet GK

## Règles générales

### Langage et Stack
- **100% Rust** - Aucun code dans d'autres langages
- **Tokio** pour l'async/await
- **Axum** pour les serveurs HTTP
- **thiserror** pour la gestion d'erreurs
- **tracing** pour le logging structuré

### Qualité du code

#### Formatage
```bash
# Toujours formater avant de commiter
cargo fmt
```

#### Linting
```bash
# Pas de warnings clippy acceptés
cargo clippy -- -D warnings
```

#### Documentation
- Toutes les APIs publiques doivent avoir des commentaires rustdoc
- Inclure des exemples d'utilisation
- Documenter les erreurs possibles
- Documenter les considérations de sécurité

#### Tests
- Tests unitaires pour chaque module
- Tests d'intégration pour les workflows complets
- Couverture minimale : 80%
- Tous les tests doivent passer avant commit

### Sécurité

#### Validation des inputs
- **TOUJOURS** valider les inputs externes
- Vérifier les longueurs, formats, caractères spéciaux
- Rejeter les null bytes et control characters
- Valider selon les RFCs (RFC 5321 pour emails, etc.)

#### Timeouts
- **TOUJOURS** mettre des timeouts sur les opérations I/O
- Timeout par défaut : 5 minutes pour les commandes
- Timeout DATA : 10 minutes
- Timeout DNS : 5 secondes

#### Limites de ressources
- Taille max message : 10MB (configurable)
- Nombre max recipients : 100 par message
- Longueur max ligne : 1000 caractères
- Nombre max erreurs : 10 avant déconnexion

#### Pas de unsafe
- Éviter `unsafe` sauf si absolument nécessaire
- Si `unsafe` utilisé, documenter pourquoi et comment
- Réviser en code review

### Gestion d'erreurs

#### Types d'erreurs
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    
    #[error("Invalid email: {0}")]
    InvalidEmail(String),
    
    #[error("SMTP protocol error: {0}")]
    SmtpProtocol(String),
}
```

#### Propagation
- Utiliser `?` pour propager les erreurs
- Ne jamais utiliser `unwrap()` ou `expect()` en production
- Logger les erreurs avec le niveau approprié

### Logging

#### Niveaux
- **DEBUG** : Détails protocolaires, commandes reçues
- **INFO** : Opérations normales (connexions, envois réussis)
- **WARN** : Comportements suspects (trop de recipients, timeouts)
- **ERROR** : Erreurs réelles (échecs de stockage, erreurs réseau)

#### Données sensibles
- **NE JAMAIS** logger les mots de passe
- **NE JAMAIS** logger les tokens d'authentification
- **NE JAMAIS** logger le contenu complet des emails en DEBUG

### Structure des modules

#### Organisation
```
src/
├── main.rs           # Point d'entrée uniquement
├── lib.rs            # Exports publics
├── config.rs         # Configuration
├── error.rs          # Types d'erreurs
├── [module]/
│   ├── mod.rs        # Exports du module
│   ├── [submodule].rs # Implémentation
│   └── tests.rs      # Tests du module (optionnel)
```

#### Taille des fichiers
- Maximum 500 lignes par fichier
- Diviser les gros modules en sous-modules
- Un fichier = une responsabilité

### Tests

#### Structure
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_name() {
        // Arrange
        // Act
        // Assert
    }
}
```

#### Tests d'intégration
- Placer dans `tests/` à la racine du composant
- Utiliser des fichiers temporaires pour l'isolation
- Nettoyer après chaque test

#### Tests async
```rust
#[tokio::test]
async fn test_async_function() {
    // Test code
}
```

### Commits Git

#### Format
```
type(scope): description

Corps du commit (optionnel)
- Détail 1
- Détail 2
```

#### Types
- `feat` : Nouvelle fonctionnalité
- `fix` : Correction de bug
- `docs` : Documentation
- `test` : Tests
- `refactor` : Refactoring
- `perf` : Performance
- `chore` : Maintenance

#### Exemples
```
feat(mail-rs): add IMAP server implementation

- Implement LOGIN, SELECT, FETCH commands
- Add Maildir mailbox support
- Add integration tests

fix(smtp): handle timeout correctly

The timeout was not being reset after each command,
causing premature disconnections.

docs: update architecture diagram

Add ai-runtime component to the diagram.
```

### Performance

#### Async I/O
- Utiliser `async` pour toutes les opérations I/O
- Utiliser `tokio::fs` au lieu de `std::fs`
- Utiliser `tokio::net` pour le réseau

#### Gestion mémoire
- Éviter les allocations inutiles
- Réutiliser les buffers quand possible
- Utiliser `Arc` pour le partage de données

### Configuration

#### Fichiers de config
- Format TOML
- Fichier d'exemple : `config.example.toml`
- Charger depuis fichier ou variables d'environnement
- Valeurs par défaut sensées

#### Variables d'environnement
- Préfixe : `GK_` ou `MAIL_RS_`
- Exemple : `GK_LOG_LEVEL=debug`

### Documentation

#### README
- Chaque composant doit avoir un README.md
- Inclure : description, installation, usage, configuration

#### CHANGELOG
- Maintenir un CHANGELOG.md
- Format : [Keep a Changelog](https://keepachangelog.com/)
- Entrées pour chaque version

### Code Review

#### Checklist
- [ ] Code compile sans warnings
- [ ] Tous les tests passent
- [ ] Nouveaux tests ajoutés
- [ ] Documentation à jour
- [ ] Pas de clippy warnings
- [ ] Sécurité vérifiée
- [ ] Performance acceptable

## Règles spécifiques par composant

### mail-rs
- Respecter strictement RFC 5321 (SMTP) et RFC 3501 (IMAP)
- Validation email selon RFC 5321
- Support SPF, DKIM, DMARC
- Stockage Maildir atomique

### ai-runtime
- Protocole MCP JSON-RPC 2.0 strict
- Support français pour le LLM
- Tool calling asynchrone
- Registry des MCP servers

### proxy-rs
- Reverse proxy HTTP/HTTPS
- Support Let's Encrypt automatique
- Rate limiting
- Routing configurable

## Exceptions

Les exceptions à ces règles doivent être :
1. Documentées dans le code
2. Justifiées dans le commit
3. Approuvées en code review

## Références

- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [Tokio Best Practices](https://tokio.rs/tokio/tutorial)
- [RFC 5321](https://www.rfc-editor.org/rfc/rfc5321) - SMTP
- [RFC 3501](https://www.rfc-editor.org/rfc/rfc3501) - IMAP
- [MCP Protocol](https://modelcontextprotocol.io/)

