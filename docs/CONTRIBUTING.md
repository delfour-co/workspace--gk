# Contributing Guidelines

## Code Quality Standards

This project follows strict quality and security standards. All contributions must meet these requirements.

## Code Style

### Rust Best Practices

#### 1. Code Formatting
```bash
# Format code before committing
just fmt

# Check formatting
just fmt-check

# Or use cargo directly
cargo fmt
cargo fmt -- --check
```

#### 2. Linting
```bash
# Run clippy
just lint

# Auto-fix issues
just lint-fix

# Or use cargo directly
cargo clippy -- -D warnings
cargo clippy -- -W clippy::all -W clippy::pedantic
```

#### 3. Documentation
All public APIs must have rustdoc comments:

```rust
/// Validates an email address according to RFC 5321
///
/// # Security
/// - Prevents null byte injection
/// - Enforces length limits
/// - Validates domain structure
///
/// # Examples
/// ```
/// use mail_rs::utils::validate_email;
/// assert!(validate_email("user@example.com").is_ok());
/// ```
///
/// # Errors
/// Returns `MailError::InvalidEmail` if validation fails
pub fn validate_email(email: &str) -> Result<()> {
    // implementation
}
```

### Error Handling

#### Use Result<T> for fallible operations
```rust
// ❌ BAD: Panicking on errors
fn read_config(path: &str) -> Config {
    std::fs::read_to_string(path).unwrap() // DON'T DO THIS
}

// ✅ GOOD: Proper error handling
fn read_config(path: &str) -> Result<Config> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| MailError::Config(e.to_string()))
}
```

#### Use custom error types
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MailError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid email: {0}")]
    InvalidEmail(String),
}
```

### Security Guidelines

#### 1. Input Validation
**ALWAYS** validate external inputs:

```rust
// ✅ GOOD: Validate before use
pub async fn handle_command(&mut self, cmd: SmtpCommand) -> Result<String> {
    match cmd {
        SmtpCommand::MailFrom(from) => {
            validate_email(&from)?; // Validate first
            self.from = Some(from);
            Ok("250 OK\r\n".to_string())
        }
    }
}
```

#### 2. Resource Limits
**ALWAYS** enforce limits:

```rust
// ✅ GOOD: Check size limits
if line.len() > MAX_LINE_LENGTH {
    return Err(MailError::SmtpProtocol("Line too long".to_string()));
}

// ✅ GOOD: Use timeouts
timeout(TIMEOUT_DURATION, async_operation()).await?;
```

#### 3. No Unsafe Code
Avoid `unsafe` unless absolutely necessary and documented:

```rust
// ❌ BAD: Unnecessary unsafe
let s = unsafe { std::str::from_utf8_unchecked(bytes) };

// ✅ GOOD: Safe alternative
let s = std::str::from_utf8(bytes)?;
```

### Testing Requirements

#### 1. Unit Tests
Every module must have unit tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_email_valid() {
        assert!(validate_email("user@example.com").is_ok());
    }

    #[test]
    fn test_validate_email_invalid() {
        assert!(validate_email("invalid").is_err());
    }

    #[test]
    fn test_validate_email_injection() {
        // Test security: null bytes
        assert!(validate_email("test\0@example.com").is_err());
    }
}
```

#### 2. Integration Tests
Test end-to-end scenarios:

```rust
#[tokio::test]
async fn test_smtp_complete_flow() {
    // Setup test server
    let server = start_test_server().await;

    // Execute complete SMTP transaction
    // Assert expected behavior
}
```

#### 3. Property-Based Testing (Future)
```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_email_validation_doesnt_panic(email in "\\PC*@\\PC*") {
        let _ = validate_email(&email);
    }
}
```

### Code Organization

#### Module Structure
```
src/
├── main.rs           # Entry point only
├── config.rs         # Configuration types
├── error.rs          # Error types
├── smtp/
│   ├── mod.rs        # Public exports
│   ├── server.rs     # Server logic
│   ├── session.rs    # Session handler
│   └── commands.rs   # Command parsing
├── storage/
│   ├── mod.rs
│   └── maildir.rs
└── utils/
    ├── mod.rs
    └── email.rs      # Validation helpers
```

#### File Size
- Keep files under 500 lines
- Split large modules into sub-modules
- One responsibility per module

### Logging

#### Structured Logging
Use appropriate log levels:

```rust
use tracing::{debug, info, warn, error};

// DEBUG: Detailed protocol info
debug!("Received command: {}", cmd);

// INFO: Normal operations
info!("New connection from {}", addr);

// WARN: Suspicious behavior
warn!("Too many recipients: {}", count);

// ERROR: Actual errors
error!("Failed to store email: {}", e);
```

#### Security-Sensitive Logging
Don't log passwords or sensitive data:

```rust
// ❌ BAD: Logging passwords
info!("User {} logged in with password {}", user, password);

// ✅ GOOD: Omit sensitive data
info!("User {} authenticated successfully", user);
```

### Performance Guidelines

#### Async/Await
Use async for I/O operations:

```rust
// ✅ GOOD: Async I/O
pub async fn read_email(&self, id: &str) -> Result<Vec<u8>> {
    tokio::fs::read(path).await
}
```

#### Resource Management
Clean up resources properly:

```rust
// ✅ GOOD: RAII pattern
{
    let _guard = resource.lock().await;
    // Resource automatically released when guard drops
}
```

## Pull Request Process

### 1. Before Submitting

```bash
# Quick workflow with Just
just fmt && just lint && just test

# Or individually
just fmt          # Format code
just lint         # Run linter
just test         # Run all tests
just audit        # Check for vulnerabilities
just docs         # Build documentation

# Or use cargo directly
cargo fmt
cargo clippy -- -D warnings
cargo test
cargo audit
cargo doc --no-deps
```

### 2. PR Requirements

- [ ] Code compiles without warnings
- [ ] All tests pass
- [ ] New features have tests
- [ ] Public APIs have rustdoc
- [ ] No clippy warnings
- [ ] Security review completed
- [ ] Updated CHANGELOG.md

### 3. PR Description Template

```markdown
## Description
Brief description of changes

## Motivation
Why is this change needed?

## Changes
- Change 1
- Change 2

## Testing
How was this tested?

## Security Impact
Any security implications?

## Breaking Changes
Any breaking changes?

## Checklist
- [ ] Tests added/updated
- [ ] Documentation updated
- [ ] No clippy warnings
- [ ] Follows code style
```

## Review Process

### Code Review Checklist

#### Functionality
- [ ] Does it work as intended?
- [ ] Edge cases handled?
- [ ] Error handling appropriate?

#### Security
- [ ] Input validation present?
- [ ] Resource limits enforced?
- [ ] No information leakage?
- [ ] Timeouts on I/O operations?

#### Code Quality
- [ ] Clear variable names?
- [ ] Comments where needed?
- [ ] No unnecessary complexity?
- [ ] Follows project style?

#### Testing
- [ ] Adequate test coverage?
- [ ] Tests are meaningful?
- [ ] Security tests included?

### Review Comments Format

```rust
// FIXME: Potential security issue
// This doesn't validate input

// TODO: Add timeout
// This I/O operation should have a timeout

// NOTE: Performance consideration
// Consider using a buffer pool here
```

## Development Workflow

### 1. Setup
```bash
# Clone repository
git clone https://github.com/yourusername/gk.git
cd gk

# Install Just command runner
cargo install just

# Complete automated setup
just setup

# This will:
# - Check prerequisites (Rust, Ollama, SQLite)
# - Pull LLM model
# - Build the project
# - Create default admin user
```

### 2. Make Changes
```bash
# Create feature branch
git checkout -b feature/my-feature

# Start development server
just dev

# Make changes...
# Tests run automatically with:
just test

# Format and lint
just fmt && just lint
```

### 3. Commit
```bash
# Use conventional commits
git commit -m "feat(smtp): add STARTTLS support"

# Types: feat, fix, docs, test, refactor, perf, chore
```

### 4. Submit PR
```bash
git push origin feature/my-feature
# Create PR on GitHub
```

## Just Command Reference

The project uses Just as a command runner for common development tasks. Run `just` or `just --list` to see all available commands.

### Quick Start Commands
```bash
just setup       # Complete initial setup
just dev         # Start all services
just test        # Run all tests
just admin       # Open admin panel
just chat        # Open chat interface
```

### Development Commands
```bash
just dev              # Start all services
just dev-mail         # Start mail server only
just dev-mcp          # Start MCP server
just dev-ai           # Start AI runtime
just dev-mail-only    # Mail server for admin testing
```

### Build Commands
```bash
just build            # Debug build
just build-release    # Release build
just build-verbose    # Verbose build
just build-mail       # Build mail-rs only
just check            # Type check only
```

### Testing Commands
```bash
just test             # Run all tests
just test-verbose     # Tests with output
just test-mail        # Mail server tests
just test-smtp        # SMTP integration tests
just test-mcp         # MCP integration tests
just test-ai          # AI runtime tests
just test-e2e         # End-to-end tests
```

### Code Quality Commands
```bash
just fmt              # Format code
just fmt-check        # Check formatting
just lint             # Run clippy
just lint-fix         # Auto-fix issues
just check            # Type check
just audit            # Security audit
```

### User Management Commands
```bash
just create-user EMAIL PASSWORD    # Create user
just list-users                    # List all users
just delete-user EMAIL             # Delete user
just create-admin                  # Create default admin
```

### Database Commands
```bash
just reset-db         # Reset all databases (WARNING: deletes data)
just backup-db        # Backup databases
just clean-maildir    # Clean mailboxes only
```

### Utility Commands
```bash
just clean            # Clean build artifacts
just clean-all        # Clean everything
just stats            # Project statistics
just docs             # Generate documentation
just update           # Update dependencies
just logs             # Show all logs
just logs-mail        # Show mail server logs
```

### Quick Access Commands
```bash
just admin            # Open admin panel in browser
just chat             # Open chat interface in browser
```

## Common Patterns

### Reading Configuration
```rust
let config = if Path::new("config.toml").exists() {
    Config::from_file("config.toml")?
} else {
    Config::default()
};
```

### Spawning Tasks
```rust
tokio::spawn(async move {
    if let Err(e) = session.handle(socket).await {
        error!("Session error: {}", e);
    }
});
```

### Timeout Pattern
```rust
match timeout(duration, operation()).await {
    Ok(Ok(result)) => Ok(result),
    Ok(Err(e)) => Err(e),
    Err(_) => Err(MailError::Timeout),
}
```

## Performance Testing

### Benchmarking (Future)
```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_email_validation(c: &mut Criterion) {
    c.bench_function("validate_email", |b| {
        b.iter(|| validate_email(black_box("test@example.com")))
    });
}

criterion_group!(benches, benchmark_email_validation);
criterion_main!(benches);
```

### Load Testing
```bash
# Use swaks for SMTP load testing
for i in {1..1000}; do
    swaks --to test@localhost --server localhost:2525 &
done
```

## Questions?

- Open a GitHub Discussion
- Check docs/
- Read the code (it's well-documented!)

## License

By contributing, you agree that your contributions will be licensed under the AGPL-3.0 License.
