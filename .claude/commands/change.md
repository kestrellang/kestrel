---
description: Make a semantic change to the compiler, update affected tests, and update documentation. For behavioral changes to existing features.
model: opus
---

You are making a semantic change to the Kestrel compiler. This workflow ensures tests and documentation stay in sync with the change.

# Reference

!`cat .claude/partials/compiler-architecture.md`

---

# Your Process

## Step 1: Understand the Change

Clarify with the user:
- What behavior is changing?
- Why is it changing? (bug fix, design improvement, new capability)
- What should the new behavior be?

---

## Step 2: Explore Impact

Use Task tool with subagent_type="Explore" to find:

1. **Implementation location**: Where in the compiler does this behavior live?
   - Parser? Builder? Binder? Analyzer?

2. **Affected tests**: What tests exercise this behavior?
   - Search `lib/kestrel-test-suite/tests/` for related tests
   - Identify tests that will need updating

3. **Documentation**: What docs describe this behavior?
   - Check `docs/language/` for user-facing docs
   - Check `docs/ai-kestrel-guide.md` for AI guidance

Present findings to user before proceeding.

---

## Step 3: Plan the Change

Create a checklist:

```markdown
## Change: [Brief description]

### Implementation
- [ ] File: [path] - [what changes]
- [ ] File: [path] - [what changes]

### Tests to Update
- [ ] [test_name] - [why it needs updating]
- [ ] [test_name] - [why it needs updating]

### Tests to Add
- [ ] [new test description]

### Documentation
- [ ] docs/language/[file].md - [what to update]
- [ ] docs/ai-kestrel-guide.md - [what to update]
```

**CONFIRMATION**: Ask user to confirm the plan before implementing.

---

## Step 4: Implement the Change

Make the semantic change in the compiler:

1. Edit the relevant files
2. Run tests to see what breaks:
   ```bash
   cargo test -p kestrel-test-suite 2>&1 | head -100
   ```

---

## Step 5: Update Tests

For each affected test:

### If test expectation is now wrong:
Update the test to expect the new behavior.

```rust
// Old (wrong expectation)
.expect(HasError("old error message"))

// New (correct expectation)
.expect(Compiles)
```

### If test is no longer relevant:
Remove or modify the test. Add a comment explaining why if not obvious.

### If new behavior needs testing:
Add new tests covering the changed behavior.

**IMPORTANT**: Never delete a test just to make things pass. Only update tests when the OLD behavior was wrong or the test was incorrect.

---

## Step 6: Verify All Tests Pass

```bash
cargo test
```

All tests must pass before proceeding to documentation.

If you run into issues, consult `docs/common-issues.md` for solutions to common problems.

For common implementation patterns, refer to `docs/contributing/patterns.md`.

---

## Step 7: Update Documentation

### `docs/language/{feature}.md`

Update if:
- Syntax changed
- Behavior changed in a user-visible way
- Error messages changed
- New capabilities added

### `docs/ai-kestrel-guide.md`

Update if:
- Code patterns changed
- Common mistakes changed
- Best practices changed
- New idioms needed

**Be precise**: Only update what actually changed. Don't rewrite unrelated sections.

---

## Step 8: Summary

Present to user:

```markdown
# Change Complete

## Semantic Change
[Brief description of what changed]

## Files Modified
- [path]: [what changed]

## Tests Updated
- [test_name]: [old expectation] → [new expectation]

## Tests Added
- [test_name]: [what it tests]

## Documentation Updated
- [path]: [what changed]
```

---

# Guidelines

- **Tests document behavior**: If you change behavior, tests MUST change
- **Docs follow tests**: Documentation should match what tests verify
- **Atomic changes**: One semantic change at a time
- **Explain why**: Comments in tests and docs should explain the change rationale

# Change to Make

$ARGUMENTS

Begin by understanding the change and exploring its impact.
