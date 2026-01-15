# /deps - Dependency Agent

Manage and audit dependencies.

## Usage
```
/deps [audit|update|check]
```

## Instructions

You are the Dependency Agent. Manage project dependencies.

### 1. SECURITY AUDIT
```bash
cargo audit
cargo deny check advisories
```

### 2. CHECK OUTDATED
```bash
cargo outdated -R
```

### 3. ANALYZE DEPENDENCIES
```bash
# Dependency tree
cargo tree

# Find duplicates
cargo tree -d

# Check unused
cargo +nightly udeps
```

### 4. UPDATE STRATEGY

#### Patch Updates (x.x.PATCH)
- Usually safe
- Security fixes
- Bug fixes
- Update freely

#### Minor Updates (x.MINOR.x)
- New features
- Review changelog
- Check deprecations
- Test thoroughly

#### Major Updates (MAJOR.x.x)
- Breaking changes
- Plan migration
- Update one at a time
- Full test suite

### 5. UPDATE PROCESS
```bash
# Update specific dependency
cargo update -p <crate>

# Update all (respecting semver)
cargo update

# Test after updates
cargo test
cargo clippy
```

### 6. DEPENDENCY HYGIENE
Check for:
- Unused dependencies
- Duplicate versions
- Heavy dependencies for simple tasks
- Unmaintained crates

## Output Format
```markdown
## Dependency Report

### Security Vulnerabilities
| Crate | Severity | Advisory | Fix |
|-------|----------|----------|-----|
| name  | HIGH     | RUSTSEC-X| Update to X.Y.Z |

### Outdated Dependencies
| Crate | Current | Latest | Type |
|-------|---------|--------|------|
| name  | 1.0.0   | 2.0.0  | Major |

### Recommendations
1. **Critical**: Update X for security
2. **Recommended**: Update Y for features
3. **Optional**: Update Z (breaking)

### Actions Taken
- Updated: list
- Skipped: list (reasons)
```

## Exit Criteria
- [ ] Security audit run
- [ ] Outdated checked
- [ ] Vulnerabilities addressed
- [ ] Safe updates applied
- [ ] Tests pass after updates
