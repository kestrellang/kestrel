---
name: fuzz
description: Stress-test a Kestrel language feature by generating many edge-case programs, compiling and running them, and reporting what breaks. Use when the user says "fuzz X", "stress test X", "try to break X", or wants exhaustive edge-case coverage of a feature. Generates `.ks` files in `./temp/fuzz/`, runs them through the `kestrel` CLI, and produces a structured pass/fail report.
---

# Fuzz — Feature Stress Tester

Generate a large battery of Kestrel programs that push a feature to its limits,
compile and run each one, then report what works and what breaks.

## Before you start

**Consult `write-kestrel`** for syntax, idioms, and gotchas. Every `.ks` file
you generate must be valid Kestrel syntax (unless the test intentionally probes
an error path). Re-read the skill each session — don't guess syntax from memory.

## Step 1 — Identify the target feature and its interaction surface

Ask the user which feature to fuzz if not obvious. Then think deeply about:

1. **The feature itself** — every form, every edge, every degenerate input.
2. **Features that interact with it** — what other language constructs compose
   with this one? For example, if fuzzing `match`:
   - Pattern kinds: literals, enum destructuring, ranges, wildcards, bindings, nested
   - Guards (`if` clauses)
   - Exhaustiveness (missing cases, overlapping cases, unreachable arms)
   - Control flow in arms (return, break, continue, throw)
   - Type interactions (generics, protocols, associated types, optionals)
   - Value categories (copyable vs move-only types in patterns)
   - Nesting (match inside match, match inside if-let, match inside loop)

3. **Boundary conditions** — zero, one, max, overflow, empty, deeply nested,
   self-referential, mutually recursive.

4. **Error paths** — what should produce a compile error? What should panic at
   runtime? What should be a clean rejection?

Write out a **test matrix** before generating code. Each row = one test case
with: name, what it tests, expected outcome (compile-ok + exit 0, compile-ok +
nonzero exit, compile-error, or crash/hang). Share this matrix with the user
for review before proceeding.

## Step 2 — Generate the test programs

Create files in `./temp/fuzz/<feature>/`:

```
temp/fuzz/<feature>/
  001_basic_usage.ks
  002_empty_input.ks
  003_nested_deeply.ks
  ...
```

### File format

Each file is a standalone Kestrel program:

```kestrel
module FuzzTest

import std.num.Int64
// ... other imports as needed

func main() -> lang.i64 {
    // Test logic here
    // Return 0 for success, non-zero for failure
    0
}
```

### Generation principles

- **Be thorough.** Generate 20–60+ test cases per feature. Cover:
  - Happy path (basic usage, typical patterns)
  - Edge cases (empty, single-element, boundary values)
  - Combinatorial interactions (feature × feature)
  - Deep nesting (5+ levels where applicable)
  - Type variety (all integer widths, floats, strings, bools, structs, enums, optionals, arrays)
  - Access mode interactions (borrowing, mutating, consuming)
  - Generic instantiations (concrete, constrained, nested generics)
  - Protocol conformance interactions
  - Error handling interactions (try, throws, Result)
  - Closure interactions (capturing, trailing syntax)
  - Move semantics (Copyable vs `not Copyable`)

- **Each file tests ONE thing.** The filename says what. If a file tests two
  orthogonal things, split it.

- **Include intentional error cases.** Some files should NOT compile — name them
  with an `err_` prefix and document the expected error in a comment at the top:
  ```kestrel
  // EXPECT: compile-error
  // Should reject: ...reason...
  module FuzzTest
  ...
  ```

- **Include runtime assertion cases.** Programs that compile but verify behavior:
  ```kestrel
  func main() -> lang.i64 {
      let result = /* feature under test */;
      if result != expected { return 1; }
      0
  }
  ```

- **Stress the compiler itself.** Include cases that might cause:
  - Slow compilation (large match, many generics)
  - Stack overflow in the compiler (deeply recursive types)
  - Incorrect codegen (subtle lowering bugs)
  - Inference ambiguity or failure

## Step 3 — Run all tests

Use this execution loop in bash:

```bash
cd /Users/dino/Documents/Projects/kestrel
mkdir -p temp/fuzz/<feature>/out
RESULTS_FILE="temp/fuzz/<feature>/results.txt"
echo "Fuzz results for <feature> — $(date)" > "$RESULTS_FILE"
echo "========================================" >> "$RESULTS_FILE"

pass=0; fail=0; error=0; crash=0

for f in temp/fuzz/<feature>/*.ks; do
    name=$(basename "$f" .ks)
    outbin="temp/fuzz/<feature>/out/${name}"

    # Check if this is an expected-error case
    expect_error=false
    if head -1 "$f" | grep -q "EXPECT: compile-error"; then
        expect_error=true
    fi

    # Compile
    compile_out=$(kestrel build "$f" -o "$outbin" 2>&1)
    compile_rc=$?

    if [ "$expect_error" = true ]; then
        if [ $compile_rc -ne 0 ]; then
            echo "PASS  $name (expected compile error)" >> "$RESULTS_FILE"
            pass=$((pass + 1))
        else
            echo "FAIL  $name (expected compile error but compiled OK)" >> "$RESULTS_FILE"
            fail=$((fail + 1))
        fi
        continue
    fi

    if [ $compile_rc -ne 0 ]; then
        echo "ERROR $name (compile failed)" >> "$RESULTS_FILE"
        echo "      $compile_out" | head -20 >> "$RESULTS_FILE"
        error=$((error + 1))
        continue
    fi

    # Run with timeout
    run_out=$(timeout 10 "$outbin" 2>&1)
    run_rc=$?

    if [ $run_rc -eq 0 ]; then
        echo "PASS  $name" >> "$RESULTS_FILE"
        pass=$((pass + 1))
    elif [ $run_rc -ge 128 ]; then
        sig=$((run_rc - 128))
        echo "CRASH $name (signal $sig)" >> "$RESULTS_FILE"
        echo "      $run_out" | head -10 >> "$RESULTS_FILE"
        crash=$((crash + 1))
    else
        echo "FAIL  $name (exit $run_rc)" >> "$RESULTS_FILE"
        echo "      $run_out" | head -10 >> "$RESULTS_FILE"
        fail=$((fail + 1))
    fi
done

echo "" >> "$RESULTS_FILE"
echo "Summary: $pass pass, $fail fail, $error compile-error, $crash crash" >> "$RESULTS_FILE"
```

Run the script with a 10-minute timeout. For large batches, consider parallel
execution with `xargs -P4`.

## Step 4 — Analyze and report

Read `results.txt` and produce a structured report:

```markdown
# Fuzz Report: <feature>

## Summary
- Total: N tests
- Pass: N
- Fail: N (unexpected runtime failure)
- Compile Error: N (unexpected compile failure)
- Crash: N (signal / hang)

## Failures

### Compile errors (unexpected)
| File | Error |
|------|-------|
| `003_nested_deeply.ks` | type mismatch: expected Int64, got ... |

### Runtime failures
| File | Exit code | Notes |
|------|-----------|-------|
| `017_overflow_boundary.ks` | 1 | Integer overflow not detected |

### Crashes
| File | Signal | Notes |
|------|--------|-------|
| `022_recursive_type.ks` | 11 (SIGSEGV) | Possible codegen bug |

## Patterns
<Group failures by root cause. Identify which compiler subsystem is likely at fault.>

## Interaction issues
<Note which feature combinations triggered failures.>

## Recommendations
<Suggest which failures are bugs worth filing vs. known limitations.>
```

## Step 5 — Triage interesting failures

For each failure that looks like a genuine compiler bug (not an intentional
error case), do a quick investigation:

1. Minimize the reproducer — strip the failing `.ks` to the smallest program
   that still triggers the issue.
2. Classify: is this a parser bug, inference bug, MIR lowering bug, or codegen bug?
3. Check if it's a known issue (grep memory files, check `CLAUDE.md`).
4. Report findings to the user with the minimized reproducer.

## Cleanup

Don't delete `temp/fuzz/` — leave it for the user to inspect. The directory is
already in `.gitignore` (or should be; warn if not).

## Anti-patterns

- **Don't generate invalid syntax unintentionally.** Every file should either
  be valid Kestrel or explicitly marked as an error case. Use `write-kestrel`.
- **Don't run the full test suite.** This skill uses `kestrel build` + direct
  execution, not triage. The temp files are not testdata.
- **Don't modify compiler source.** This skill is read-only on the compiler —
  it only generates and runs `.ks` files.
- **Don't skip the matrix step.** Generating without planning produces shallow
  coverage with gaps. Think first, generate second.
- **Don't ignore crashes.** A SIGSEGV is always a bug. Investigate it.
