---
description: Debug failing tests by creating minimal reproductions and using CLI debugging tools.
model: opus
---

You are debugging a failing test in the Kestrel compiler.

# Reference

!`cat .claude/partials/cli-debug.md`

---

# Your Process

## Step 1: Identify the Failure

Run the failing test:

```bash
cargo test -p kestrel-test-suite test_name -- --nocapture 2>&1
```

Parse output to identify:
- Test name and location
- Expected vs actual behavior
- Error messages or panics

---

## Step 2: Create Minimal Reproduction

Create a minimal `.ks` file in `temp/` that reproduces the issue:

```bash
mkdir -p temp
```

Write `temp/repro.ks` with the smallest code that triggers the issue.

**Principle**: Remove everything not needed to reproduce. Simpler = easier to debug.

---

## Step 3: Use CLI Debug Tools

Run the reproduction through different compilation stages:

```bash
# 1. Check parsing (syntax tree)
cargo run -- parse temp/repro.ks --tree

# 2. Check symbols (are they created?)
cargo run -- temp/repro.ks --symbols

# 3. Check semantic tree (are behaviors attached?)
cargo run -- temp/repro.ks --tree=full

# 4. Check execution graph (codegen issues)
cargo run -- temp/repro.ks --xgraph
```

Compare outputs at each stage. The stage where things go wrong narrows the problem.

---

## Step 4: Deep Investigation

### If issue is in parsing:
- Check `lib/kestrel-parser/src/` for the relevant parser
- Verify token recognition in lexer
- Check event emission order

### If issue is in BUILD phase:
- Check builder registration in `lowerer.rs`
- Add debug prints in builder: `eprintln!("DEBUG: building {:?}", syntax.kind());`

### If issue is in BIND phase:
- Check binder registration in `declaration_binder.rs`
- Check `body_resolver/mod.rs` for expression/statement issues
- Check `type_resolver.rs` for type resolution issues

### If issue is in VALIDATE phase:
- Check analyzer registration in `kestrel-semantic-analyzers/src/lib.rs`
- Verify analyzer is running (add debug print)

### Using LLDB:

```bash
cargo build
lldb target/debug/kestrel -- temp/repro.ks

# Set breakpoint at suspicious location
b kestrel_semantic_tree_binder::body_resolver::resolve_expr
r
```

---

## Step 5: Explore Codebase

Use Task tool with subagent_type="Explore" to:
- Find where the error is produced
- Understand the code path
- Find similar working cases to compare

---

## Step 6: Report Findings

Present to user:

```markdown
# Test Failure Analysis

## Test
**Name**: test_name
**Location**: lib/kestrel-test-suite/tests/file.rs:line

## Reproduction
**File**: temp/repro.ks
```kestrel
// Minimal reproduction code
```

## Investigation

### Stage Analysis
- Parsing: [OK/ISSUE]
- BUILD: [OK/ISSUE]
- BIND: [OK/ISSUE]
- VALIDATE: [OK/ISSUE]

### Issue Location
**Phase**: [parsing/build/bind/validate/codegen]
**File**: path/to/file.rs
**Function**: function_name
**Line**: ~123

## Root Cause
[Detailed explanation of what's going wrong]

## Suggested Fix
[Specific code changes needed with file paths]
```

---

## Step 7: Ask User

If you run into issues, consult `docs/common-issues.md` for solutions to common problems.

"I've identified the root cause above. Would you like me to:
1. **Fix the bug** - Implement the suggested fix
2. **Update the test** - If the test expectation is wrong
3. **Investigate further** - Dig deeper into this issue"

---

## Step 8: Clean Up

After fixing, remove debug statements:
- Search for `eprintln!("DEBUG` and remove
- Delete `temp/repro.ks` or keep for reference

Verify the fix:
```bash
cargo test -p kestrel-test-suite test_name
cargo test  # Run all tests to check for regressions
```

---

# Test to Debug

$ARGUMENTS

Begin by running the test and analyzing the failure.
