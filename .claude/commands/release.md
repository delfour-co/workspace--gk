# /release - Release Agent

Prepare a new release.

## Usage
```
/release [major|minor|patch]
```

## Instructions

You are the Release Agent. Prepare a release.

### 1. PRE-RELEASE CHECKS
```bash
# All tests pass
cargo test

# No warnings
cargo clippy -- -D warnings

# Format check
cargo fmt --all -- --check

# Security audit
cargo audit

# Build release
cargo build --release
```

### 2. DETERMINE VERSION
Based on changes since last release:
- **MAJOR**: Breaking API changes
- **MINOR**: New features, backward compatible
- **PATCH**: Bug fixes only

```bash
# Current version
grep "^version" Cargo.toml | head -1

# Changes since last tag
git log $(git describe --tags --abbrev=0)..HEAD --oneline
```

### 3. UPDATE VERSION
Update in:
- `Cargo.toml` (workspace and members)
- Any version constants in code

### 4. GENERATE CHANGELOG
```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New features

### Changed
- Modifications

### Fixed
- Bug fixes

### Security
- Security fixes

### Deprecated
- Deprecations

### Removed
- Removed features
```

### 5. COMMIT AND TAG
```bash
git add -A
git commit -m "chore(release): prepare vX.Y.Z

- Update version to X.Y.Z
- Update CHANGELOG.md"

git tag -a vX.Y.Z -m "Release vX.Y.Z"
```

### 6. PUSH RELEASE
```bash
git push origin main
git push origin vX.Y.Z
```

### 7. CREATE GITHUB RELEASE
```bash
gh release create vX.Y.Z \
  --title "vX.Y.Z" \
  --notes "## What's Changed

### Added
- ...

### Fixed
- ...

**Full Changelog**: https://github.com/OWNER/REPO/compare/vPREV...vX.Y.Z"
```

## Output
- Updated version numbers
- Updated CHANGELOG.md
- Git tag created
- GitHub release created

## Exit Criteria
- [ ] All checks pass
- [ ] Version bumped correctly
- [ ] CHANGELOG updated
- [ ] Commit created
- [ ] Tag created
- [ ] GitHub release published
