# /tests - Test Coverage Agent

Improve test coverage and quality.

## Usage
```
/tests [analyze|generate|improve] [module|file|all]
```

## Instructions

You are the Test Coverage Agent. Improve testing.

### 1. MEASURE COVERAGE
```bash
# If cargo-tarpaulin is installed
cargo tarpaulin --out Html --output-dir coverage/ 2>&1 | tail -20

# Or with llvm-cov
cargo llvm-cov --html
```

### 2. IDENTIFY UNTESTED CODE
- Functions without tests
- Branches not covered
- Error paths not tested
- Edge cases missing

### 3. TEST GENERATION RULES

#### Naming Convention
```
test_<function_name>_<scenario>
```
Examples: `test_login_success`, `test_login_invalid_password`

#### AAA Pattern
```rust
#[test]
fn test_function_scenario() {
    // Arrange
    let input = create_test_input();

    // Act
    let result = function_under_test(input);

    // Assert
    assert_eq!(result, expected);
}
```

#### Error Testing
```rust
#[test]
fn test_function_error() {
    let result = function(invalid_input);
    assert!(result.is_err());
    assert!(matches!(result, Err(Error::Expected)));
}
```

#### Async Testing
```rust
#[tokio::test]
async fn test_async_function() {
    let result = async_function().await;
    assert!(result.is_ok());
}
```

### 4. COVERAGE TARGETS
- Line coverage: > 80%
- Branch coverage: > 70%
- Critical paths: 100%

### 5. PRIORITY
1. Security-critical code
2. Business logic
3. Error handling paths
4. Edge cases
5. Integration points

### 6. VERIFY
```bash
cargo test
```

## Output
- Coverage report
- Generated test files
- Recommendations

## Exit Criteria
- [ ] Coverage measured
- [ ] Critical paths tested
- [ ] Error conditions tested
- [ ] Edge cases covered
- [ ] All tests pass
