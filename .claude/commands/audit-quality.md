# /audit-quality - Code Quality Audit Agent

Audit code quality and identify improvements.

## Usage
```
/audit-quality [module|file|all]
```

## Instructions

You are the Code Quality Agent. Analyze the specified scope.

### 1. RUN STATIC ANALYSIS
```bash
cargo clippy --all-targets --all-features -- -W clippy::all 2>&1
cargo fmt --all -- --check
```

### 2. ANALYZE CODE SMELLS
Look for:
- Functions > 50 lines
- Files > 500 lines
- Cyclomatic complexity > 10
- Deep nesting (> 4 levels)
- Magic numbers/strings
- Dead code
- Duplicate code

### 3. CHECK SOLID PRINCIPLES
- **S**ingle Responsibility: One reason to change
- **O**pen/Closed: Extensible without modification
- **L**iskov Substitution: Subtypes substitutable
- **I**nterface Segregation: Small, focused traits
- **D**ependency Inversion: Depend on abstractions

### 4. RUST BEST PRACTICES
- Use `?` over `.unwrap()` in production code
- Prefer `&str` over `String` in parameters
- Use `impl Trait` appropriately
- Leverage type system for safety
- Follow naming conventions

### 5. OUTPUT REPORT

```markdown
## Code Quality Report

### Summary
- Critical issues: N
- Warnings: N
- Suggestions: N

### Critical Issues
1. **[file:line]** Description
   - Problem: ...
   - Suggestion: ...

### Warnings
1. **[file:line]** Description

### Suggestions
1. **[file:line]** Description

### Metrics
- Lines of code: N
- Functions > 50 lines: N
- Clippy warnings: N
```

### 6. CREATE ISSUES FOR CRITICAL ITEMS
```bash
gh issue create --title "refactor: <description>" --label "tech-debt" --body "..."
```

## Exit Criteria
- [ ] Static analysis run
- [ ] Code smells identified
- [ ] SOLID violations noted
- [ ] Report generated
- [ ] Critical issues filed
