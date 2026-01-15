# /review - Code Review Agent

Review a pull request.

## Usage
```
/review <pr-number>
```

## Instructions

You are the Code Review Agent. Review PR #$ARGUMENTS.

### 1. FETCH PR INFO
```bash
gh pr view $ARGUMENTS
gh pr diff $ARGUMENTS
gh pr checks $ARGUMENTS
```

### 2. REVIEW CHECKLIST

#### Correctness
- [ ] Logic is correct
- [ ] Edge cases handled
- [ ] Error handling appropriate
- [ ] No obvious bugs

#### Security
- [ ] Input validated
- [ ] No injection risks
- [ ] Auth checks in place
- [ ] No sensitive data exposed

#### Performance
- [ ] Efficient algorithms
- [ ] No N+1 queries
- [ ] No unnecessary allocations
- [ ] Async used appropriately

#### Code Quality
- [ ] Follows conventions
- [ ] Functions are focused
- [ ] Names are clear
- [ ] No duplication

#### Testing
- [ ] Tests included
- [ ] Tests are meaningful
- [ ] Edge cases tested
- [ ] All tests pass

#### Documentation
- [ ] Public APIs documented
- [ ] Complex logic explained

### 3. FEEDBACK FORMAT

#### Blocker
```
**BLOCKER**: [Category]
This will cause [problem] because [reason].
Suggestion: ...
```

#### Important
```
**IMPORTANT**: [Category]
Consider [suggestion] because [reason].
```

#### Minor
```
**SUGGESTION**: Nit: [improvement]
```

#### Positive
```
**NICE**: Good use of [pattern]!
```

### 4. SUBMIT REVIEW
```bash
# Approve
gh pr review $ARGUMENTS --approve --body "LGTM! ..."

# Request changes
gh pr review $ARGUMENTS --request-changes --body "Please address..."

# Comment only
gh pr review $ARGUMENTS --comment --body "Some suggestions..."
```

## Output Format
```markdown
## PR Review: #$ARGUMENTS

### Summary
[What this PR does]

### Recommendation
- [ ] APPROVE
- [ ] REQUEST CHANGES
- [ ] COMMENT

### Must Fix (Blockers)
1. [file:line] Issue

### Should Fix
1. [file:line] Issue

### Suggestions
1. [file:line] Suggestion

### Positive
1. Good point
```

## Exit Criteria
- [ ] All changes reviewed
- [ ] Security considered
- [ ] Performance considered
- [ ] Clear recommendation
- [ ] Review submitted
