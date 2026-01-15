# Claude Code Agents

This directory contains 12 specialized AI agents for software development workflows.

## Quick Reference

| Command | Agent | Purpose |
|---------|-------|---------|
| `/feature <issue>` | [Feature Dev](./feature-dev.md) | Develop features from GitHub issues |
| `/bugfix <issue>` | [Bug Fix](./bug-fix.md) | Fix bugs with regression tests |
| `/audit-quality` | [Code Quality](./code-quality.md) | Audit code quality and patterns |
| `/audit-security` | [Security](./security-audit.md) | Security vulnerability analysis |
| `/docs` | [Documentation](./documentation.md) | Generate and maintain docs |
| `/tests` | [Test Coverage](./test-coverage.md) | Improve test coverage |
| `/review <pr>` | [Code Review](./code-review.md) | Review pull requests |
| `/perf` | [Performance](./performance.md) | Performance analysis |
| `/deps` | [Dependency](./dependency.md) | Manage dependencies |
| `/release` | [Release](./release.md) | Prepare releases |
| `/refactor` | [Refactoring](./refactoring.md) | Safe code refactoring |
| `/devops` | [DevOps](./devops.md) | CI/CD and infrastructure |

## Agent Categories

### Development Agents
- **Feature Dev** - Takes GitHub issues, creates branches, implements features, prepares PRs
- **Bug Fix** - Fixes bugs with mandatory regression tests

### Quality Agents
- **Code Quality** - Analyzes code smells, SOLID violations, maintainability
- **Security Audit** - OWASP vulnerabilities, crypto issues, input validation
- **Code Review** - Reviews PRs before merge

### Testing Agents
- **Test Coverage** - Identifies gaps, generates tests, improves coverage

### Maintenance Agents
- **Documentation** - Generates and updates all documentation
- **Performance** - Identifies bottlenecks, suggests optimizations
- **Dependency** - Updates deps, checks for vulnerabilities
- **Refactoring** - Safely restructures code

### Operations Agents
- **Release** - Version bumping, changelog, tagging, publishing
- **DevOps** - Docker, CI/CD, deployment configurations

## Workflow Integration

```
┌─────────────────────────────────────────────────────────────────┐
│                     DEVELOPMENT WORKFLOW                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   GitHub Issue                                                   │
│        │                                                         │
│        ▼                                                         │
│   ┌─────────────┐    ┌─────────────┐                            │
│   │ Feature Dev │ or │  Bug Fix    │                            │
│   │   Agent     │    │   Agent     │                            │
│   └──────┬──────┘    └──────┬──────┘                            │
│          │                  │                                    │
│          └────────┬─────────┘                                    │
│                   ▼                                              │
│          ┌─────────────────┐                                     │
│          │  Test Coverage  │                                     │
│          │     Agent       │                                     │
│          └────────┬────────┘                                     │
│                   │                                              │
│    ┌──────────────┼──────────────┐                              │
│    ▼              ▼              ▼                              │
│ ┌───────┐   ┌──────────┐   ┌───────────┐                        │
│ │Quality│   │ Security │   │   Docs    │                        │
│ │ Audit │   │  Audit   │   │  Agent    │                        │
│ └───┬───┘   └────┬─────┘   └─────┬─────┘                        │
│     │            │               │                               │
│     └────────────┼───────────────┘                               │
│                  ▼                                               │
│         ┌─────────────────┐                                      │
│         │   Code Review   │                                      │
│         │     Agent       │                                      │
│         └────────┬────────┘                                      │
│                  │                                               │
│                  ▼                                               │
│         ┌─────────────────┐                                      │
│         │   PR Merged     │                                      │
│         └────────┬────────┘                                      │
│                  │                                               │
│                  ▼                                               │
│         ┌─────────────────┐                                      │
│         │ Release Agent   │                                      │
│         └─────────────────┘                                      │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Usage Examples

### Implement a Feature
```bash
# Start feature development
/feature 42

# Agent will:
# 1. Fetch issue #42 details
# 2. Create feature/issue-42-description branch
# 3. Implement the feature
# 4. Run quality gates
# 5. Create PR
```

### Fix a Bug
```bash
# Start bug fix
/bugfix 123

# Agent will:
# 1. Analyze the bug
# 2. Write failing regression test
# 3. Fix the bug
# 4. Verify test passes
# 5. Create PR
```

### Security Audit
```bash
# Audit entire codebase
/audit-security all

# Audit specific module
/audit-security mail-rs/src/smtp
```

### Prepare Release
```bash
# Patch release
/release patch

# Minor release
/release minor

# Specific version
/release 2.0.0
```

## Quality Gates

All development agents enforce these quality gates:

| Gate | Command | Required |
|------|---------|----------|
| Format | `cargo fmt -- --check` | ✅ |
| Lint | `cargo clippy -- -D warnings` | ✅ |
| Test | `cargo test` | ✅ |
| Build | `cargo build --release` | ✅ |
| Security | `cargo audit` | ⚠️ (warnings) |

## Configuration

Agents can be customized by editing their respective `.md` files.

### Common Customizations
- Quality gate thresholds
- Branch naming conventions
- PR templates
- Commit message format
- Test coverage targets

## Best Practices

1. **Always use Feature Dev for new features** - Ensures consistent quality
2. **Always use Bug Fix for bugs** - Mandatory regression tests
3. **Run Security Audit before releases** - Catch vulnerabilities early
4. **Use Code Review for all PRs** - Even self-reviewed code benefits
5. **Keep Documentation updated** - Run docs agent after major changes
