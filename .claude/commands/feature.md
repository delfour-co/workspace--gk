# /feature - Feature Development Agent

Develop a new feature from a GitHub issue.

## Usage
```
/feature <issue-number>
```

## Instructions

You are the Feature Development Agent. Follow this workflow:

### 1. UNDERSTAND THE ISSUE
```bash
gh issue view $ARGUMENTS
```
- Extract requirements, acceptance criteria
- Identify affected components

### 2. CREATE FEATURE BRANCH
```bash
git checkout main && git pull origin main
git checkout -b feature/issue-$ARGUMENTS-<short-description>
```

### 3. IMPLEMENT THE FEATURE
- Follow existing code patterns
- Write clean, documented code
- Add necessary tests

### 4. QUALITY GATES (MANDATORY)
All must pass before PR:
```bash
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo test
cargo build --release
```

### 5. COMMIT WITH CONVENTIONAL FORMAT
```
feat(<scope>): <description>

<body explaining what and why>

Closes #$ARGUMENTS
```

### 6. PREPARE PR
```bash
git push -u origin HEAD
gh pr create --title "feat: <description>" --body "## Summary
- What this PR does

## Changes
- List of changes

## Testing
- How it was tested

Closes #$ARGUMENTS"
```

## Exit Criteria
- [ ] Feature fully implemented
- [ ] All quality gates pass
- [ ] Tests added and passing
- [ ] PR created and linked to issue
