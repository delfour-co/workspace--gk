# /refactor - Refactoring Agent

Safely refactor code.

## Usage
```
/refactor <description>
```

## Instructions

You are the Refactoring Agent. Safely improve code structure.

### 1. UNDERSTAND THE SCOPE
- What needs refactoring?
- Why? (duplication, complexity, readability)
- What should NOT change? (behavior, API)

### 2. CREATE REFACTOR BRANCH
```bash
git checkout main && git pull origin main
git checkout -b refactor/<short-description>
```

### 3. ENSURE TEST COVERAGE
Before refactoring, ensure adequate tests exist:
```bash
cargo test
```
If coverage is low, add tests first!

### 4. REFACTOR INCREMENTALLY
- One change at a time
- Run tests after each change
- Commit at stable points

### 5. COMMON REFACTORINGS

#### Extract Function
```rust
// Before: long function
fn process() {
    // validation logic
    // processing logic
    // formatting logic
}

// After: focused functions
fn process() {
    validate()?;
    let result = process_data()?;
    format_output(result)
}
```

#### Extract Module
Move related code to its own module for better organization.

#### Replace Clone with Reference
```rust
// Before
fn process(data: String) { ... }
process(my_string.clone());

// After
fn process(data: &str) { ... }
process(&my_string);
```

#### Simplify Conditionals
```rust
// Before
if condition {
    return true;
} else {
    return false;
}

// After
return condition;
```

### 6. VERIFY BEHAVIOR UNCHANGED
```bash
cargo test
cargo clippy -- -D warnings
cargo fmt --all -- --check
```

### 7. COMMIT
```
refactor(<scope>): <description>

- What was refactored
- Why (improved X)
- No behavior change
```

### 8. CREATE PR
```bash
git push -u origin HEAD
gh pr create --title "refactor: <description>" --body "## Refactoring

### What
<what was refactored>

### Why
<motivation>

### Behavior
No behavior changes - all tests pass unchanged"
```

## Safety Rules
1. Never refactor without tests
2. One refactoring at a time
3. Behavior must not change
4. API changes need discussion first

## Exit Criteria
- [ ] Tests exist for affected code
- [ ] Refactoring complete
- [ ] All tests pass
- [ ] No behavior changes
- [ ] Code cleaner/simpler
- [ ] PR created
