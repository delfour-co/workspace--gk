# /docs - Documentation Agent

Generate and maintain documentation.

## Usage
```
/docs [generate|update|audit] [module|api|readme|all]
```

## Instructions

You are the Documentation Agent. Maintain project documentation.

### 1. AUDIT CURRENT DOCUMENTATION
```bash
# Check for missing docs
cargo doc 2>&1 | grep "warning: missing documentation"

# Count documented items
grep -r "///" --include="*.rs" | wc -l
```

### 2. RUSTDOC STANDARDS

Every public item must have:
```rust
/// Brief description.
///
/// # Arguments
/// * `param` - Description
///
/// # Returns
/// Description of return value
///
/// # Errors
/// When this can fail
///
/// # Examples
/// ```
/// let result = function(arg)?;
/// ```
pub fn function(param: Type) -> Result<T, E>
```

### 3. MODULE DOCUMENTATION
```rust
//! Module name
//!
//! Description of what this module provides.
//!
//! # Overview
//! High-level explanation
//!
//! # Examples
//! Usage examples
```

### 4. README STRUCTURE
- Project title and badges
- Brief description
- Features list
- Quick start / Installation
- Configuration
- Usage examples
- API reference link
- Contributing
- License

### 5. API DOCUMENTATION
For each endpoint:
- HTTP method and path
- Description
- Authentication requirements
- Request/response examples
- Error codes
- curl example

### 6. GENERATE DOCS
```bash
cargo doc --no-deps --open
```

## Output
- Updated documentation files
- Coverage report
- List of missing docs

## Exit Criteria
- [ ] All public items documented
- [ ] Examples compile and run
- [ ] README is complete
- [ ] No broken doc links
