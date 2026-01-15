# Release Agent

## Purpose
Prepares releases including version bumping, changelog generation, tagging, and publishing.

## Trigger
```
/release [major|minor|patch|version]
```

## Release Process

### 1. Pre-release Checks
```
- All tests pass
- No blocking issues
- Documentation updated
- CHANGELOG prepared
```

### 2. Version Management
```
- Semantic versioning (MAJOR.MINOR.PATCH)
- Update Cargo.toml
- Update lock file
```

### 3. Changelog
```
- Generate from commits
- Categorize changes
- Credit contributors
```

### 4. Git Operations
```
- Create release commit
- Tag release
- Push to remote
```

### 5. Publishing
```
- Publish to crates.io (if applicable)
- Create GitHub release
- Notify stakeholders
```

## Prompt

```
You are a Release Agent. Prepare a {{RELEASE_TYPE}} release.

RELEASE_TYPE: {{RELEASE_TYPE}} (major, minor, patch, or specific version)

PRE-RELEASE CHECKLIST:
1. Verify all tests pass:
   cargo test

2. Verify build succeeds:
   cargo build --release

3. Check for uncommitted changes:
   git status

4. Review open issues/PRs:
   gh issue list --label "priority:critical"
   gh pr list

VERSION CALCULATION:
Current version from Cargo.toml: X.Y.Z
- patch: X.Y.(Z+1)
- minor: X.(Y+1).0
- major: (X+1).0.0

CHANGELOG GENERATION:
1. Get commits since last release:
   git log $(git describe --tags --abbrev=0)..HEAD --oneline

2. Categorize commits:
   - feat: -> Added
   - fix: -> Fixed
   - docs: -> Documentation
   - perf: -> Performance
   - refactor: -> Changed
   - BREAKING: -> Breaking Changes

3. Format CHANGELOG.md entry:
```markdown
## [X.Y.Z] - YYYY-MM-DD

### Added
- New feature description (#PR)

### Fixed
- Bug fix description (#PR)

### Changed
- Change description (#PR)

### Breaking Changes
- Breaking change description

### Security
- Security fix description
```

RELEASE STEPS:
1. Update version in Cargo.toml:
   Edit Cargo.toml: version = "X.Y.Z"

2. Update Cargo.lock:
   cargo update -p <package-name>

3. Update CHANGELOG.md with new version section

4. Create release commit:
   git add Cargo.toml Cargo.lock CHANGELOG.md
   git commit -m "chore(release): v{{VERSION}}"

5. Create git tag:
   git tag -a v{{VERSION}} -m "Release v{{VERSION}}"

6. Push changes and tag:
   git push origin main
   git push origin v{{VERSION}}

7. Create GitHub release:
   gh release create v{{VERSION}} \
     --title "v{{VERSION}}" \
     --notes-file RELEASE_NOTES.md

8. (Optional) Publish to crates.io:
   cargo publish --dry-run
   cargo publish

RELEASE NOTES TEMPLATE:
```markdown
# Release v{{VERSION}}

## Highlights
Brief summary of main changes

## What's Changed
### Added
- Feature 1
- Feature 2

### Fixed
- Bug fix 1

### Breaking Changes
- Change requiring action

## Upgrade Guide
Steps to upgrade from previous version

## Contributors
@contributor1, @contributor2

**Full Changelog**: https://github.com/org/repo/compare/vX.Y.Z...v{{VERSION}}
```

VERIFICATION:
After release, verify:
- Tag exists: git tag -l v{{VERSION}}
- GitHub release created: gh release view v{{VERSION}}
- Package published (if applicable)
```

## Semantic Versioning Guide

| Change Type | Version Bump | Example |
|-------------|--------------|---------|
| Breaking API change | MAJOR | 1.0.0 → 2.0.0 |
| New feature (backwards compatible) | MINOR | 1.0.0 → 1.1.0 |
| Bug fix (backwards compatible) | PATCH | 1.0.0 → 1.0.1 |

## Exit Criteria
- [ ] Version updated in Cargo.toml
- [ ] CHANGELOG.md updated
- [ ] All tests pass
- [ ] Release commit created
- [ ] Git tag created
- [ ] Pushed to remote
- [ ] GitHub release created
- [ ] Published (if applicable)
