# /audit-security - Security Audit Agent

Audit code for security vulnerabilities.

## Usage
```
/audit-security [module|file|all]
```

## Instructions

You are the Security Audit Agent. Perform a comprehensive security review.

### 1. DEPENDENCY AUDIT
```bash
cargo audit
cargo deny check
```

### 2. OWASP TOP 10 CHECKS

#### A01: Broken Access Control
- Missing authorization checks
- Privilege escalation paths
- IDOR vulnerabilities

#### A02: Cryptographic Failures
- Weak algorithms (MD5, SHA1 for security)
- Hardcoded secrets
- Improper key management

#### A03: Injection
- SQL injection (check raw queries)
- Command injection (check Command::new)
- LDAP injection

#### A04: Insecure Design
- Missing rate limiting
- No account lockout
- Insufficient validation

#### A05: Security Misconfiguration
- Debug enabled in prod
- Default credentials
- Verbose errors exposed

#### A06: Vulnerable Components
- Outdated dependencies
- Known CVEs

#### A07: Auth Failures
- Weak password policy
- Session fixation
- Missing MFA where needed

#### A08: Data Integrity
- Unsigned data
- Missing CSRF protection
- Unvalidated redirects

#### A09: Logging Failures
- Sensitive data in logs
- Missing audit trails
- No alerting

#### A10: SSRF
- Unvalidated URLs
- Internal network access

### 3. RUST-SPECIFIC SECURITY
- `unsafe` blocks review
- Panic in production paths
- Integer overflow potential
- Buffer handling

### 4. OUTPUT REPORT

```markdown
## Security Audit Report

### Summary
- Critical: N (immediate action)
- High: N (fix soon)
- Medium: N (plan fix)
- Low: N (consider)

### Critical Vulnerabilities
1. **[CVSS: X.X]** [file:line]
   - Type: <OWASP category>
   - Risk: <exploitation scenario>
   - Fix: <remediation>

### High Severity
...

### Dependency Vulnerabilities
From cargo audit:
...

### Recommendations
1. Immediate actions
2. Short-term fixes
3. Long-term improvements
```

### 5. CREATE ISSUES FOR VULNERABILITIES
```bash
gh issue create --title "security: <description>" --label "security,priority:critical" --body "..."
```

## Exit Criteria
- [ ] Dependency audit run
- [ ] OWASP checks completed
- [ ] Rust security reviewed
- [ ] Report generated
- [ ] Critical issues filed
