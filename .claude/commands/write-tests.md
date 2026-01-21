---
description: Write comprehensive tests for a Kestrel feature. Follows test-driven development.
model: opus
---

You are writing tests for a Kestrel compiler feature.

# Reference

!`cat .claude/partials/test-api.md`

---

# Your Process

## Step 1: Understand the Feature

Read the user's request to understand what feature needs tests.

**Check these resources:**
- `docs/ai-kestrel-guide.md` - AI guidance for the language
- `docs/language/` - User-facing feature documentation
- `docs/plans/{feature}/` - Design and plan documents if they exist

Determine:
- What behaviors to test (success cases)
- What errors should be caught (failure cases)
- Edge cases to cover

---

## Step 2: Explore Existing Tests

Use Task tool with subagent_type="Explore" to:

1. **Find existing tests** for this feature or similar features
   - Search in `lib/kestrel-test-suite/tests/`
   - Check if tests already exist (avoid duplication)

2. **Find the implementation** to understand what to test
   - Search for relevant symbols, validation passes, builders, binders
   - Understand what errors can be produced

3. **Find similar test patterns** to follow
   - How are similar features tested?
   - What module structure is used?

---

## Step 3: Design Test Categories

Organize tests into modules:

```rust
mod feature_name {
    use super::*;

    mod basic {
        // Simple, happy-path cases
    }

    mod visibility {
        // public, private, internal, fileprivate (if applicable)
    }

    mod generics {
        // Generic versions (if applicable)
    }

    mod validation {
        // Error cases - one test per distinct error
    }

    mod edge_cases {
        // Unusual but valid cases
    }
}
```

---

## Step 4: Write Tests

### Principles

- **Test behavior, not implementation** - Focus on what the user sees
- **One concept per test** - Don't combine unrelated assertions
- **Minimal code** - Smallest example that tests the feature
- **Clear names** - `feature_basic`, `feature_with_modifier`, `feature_error_reason`

### Basic Tests
```rust
#[test]
fn feature_basic() {
    Test::new(r#"
module Test
// simplest case
"#)
    .expect(Compiles)
    .expect(Symbol::new("Name").is(SymbolKind::...));
}
```

### Error Tests
```rust
#[test]
fn feature_error_missing_something() {
    Test::new(r#"
module Test
// invalid case
"#)
    .expect(HasError("distinctive error substring"));
}
```

---

## Step 5: Avoid Redundancy

Before writing a test, check:
- Does this test already exist?
- Does another test already cover this case?
- Is this testing the same thing with different syntax?

**Don't test:**
- The same error message twice
- Obvious type system features unless that's the focus
- Implementation details that could change

---

## Step 6: Write the Code

1. Create or modify test file in `lib/kestrel-test-suite/tests/`
2. Add `use kestrel_test_suite::*;`
3. Organize into nested modules
4. Write each test with clear naming

---

## Step 7: Update mod.rs

If creating a new file, add to parent `mod.rs`:

```rust
mod new_feature;
```

---

## Step 8: Run Tests

```bash
cargo test -p kestrel-test-suite
```

For TDD: tests should fail initially, then pass after implementation.
For existing features: all tests should pass.

---

## Step 9: Update Documentation (if needed)

After writing tests, check if documentation needs updating:

### Check `docs/language/{feature}.md`
- Does this file exist for the feature?
- Do the tests reveal behaviors not documented?
- Are there new edge cases worth documenting?

If updates needed, add examples or clarifications.

### Check `docs/ai-kestrel-guide.md`
- Does the AI guide cover this feature adequately?
- Do the tests show patterns the AI should know about?
- Are there common mistakes the tests catch that should be warned about?

If updates needed, add to the relevant section.

**Only update docs if the tests reveal something new or undocumented.**

---

# Feature to Test

$ARGUMENTS

Begin by exploring existing tests for this feature.
