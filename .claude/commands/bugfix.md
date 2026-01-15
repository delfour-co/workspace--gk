# /bugfix - Bug Fix Agent

Fix a bug from a GitHub issue with mandatory regression tests.

## Usage
```
/bugfix <issue-number>
```

## Instructions

You are the Bug Fix Agent. Follow this workflow:

### 1. UNDERSTAND THE BUG
```bash
gh issue view $ARGUMENTS
```
- Identify symptoms and expected behavior
- Find reproduction steps

### 2. CREATE FIX BRANCH
```bash
git checkout main && git pull origin main
git checkout -b fix/issue-$ARGUMENTS-<short-description>
```

### 3. REPRODUCE THE BUG (MANDATORY)
Write a failing test BEFORE fixing:
```rust
#[test]
fn test_issue_$ARGUMENTS_regression() {
    // This test must FAIL before the fix
    // and PASS after the fix
}
```

### 4. FIX THE BUG
- Minimal fix only - don't refactor unrelated code
- Fix the root cause, not symptoms

### 5. VERIFY FIX
```bash
cargo test  # Regression test now passes
cargo fmt --all -- --check
cargo clippy -- -D warnings
cargo build --release
```

### 6. COMMIT
```
fix(<scope>): <description>

<explanation of the root cause and fix>

Closes #$ARGUMENTS
```

### 7. CREATE PR
```bash
git push -u origin HEAD
gh pr create --title "fix: <description>" --body "## Bug
<description of the bug>

## Root Cause
<what caused it>

## Fix
<how it was fixed>

## Regression Test
<test added to prevent recurrence>

Closes #$ARGUMENTS"
```

## Exit Criteria
- [ ] Bug reproduced with failing test
- [ ] Root cause identified
- [ ] Minimal fix applied
- [ ] Regression test passes
- [ ] All quality gates pass
- [ ] PR created
