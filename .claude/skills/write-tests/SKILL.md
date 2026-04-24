---
name: write-tests
description: Write tests for a Kestrel compiler feature. Use when the user asks to add a test, write tests for a feature, cover a bug with a regression test, or when a diagnostic/behavior change needs testdata updates. Covers the lib2 `.ks` testdata format (`diagnostics` and `execution` kinds), the `// ERROR:` annotation style, where to place files under `lib2/kestrel-test-suite/testdata/`, and how to run the tests via the `triage` skill (never `cargo test` directly).
---

# Writing Kestrel Tests (lib2)

lib2 tests are **`.ks` files under `lib2/kestrel-test-suite/testdata/`** — not Rust code.
There is no `Test::new(...)` builder anymore; each file is a full Kestrel program with
a header that tells the harness what to check.

> **lib2 only.** Do not write tests against `lib/kestrel-test-suite/` (lib1). lib1 is dead.

## File format

Every test file begins with:

```
// test: <kind>
// stdlib: <true|false>

module <Name>

...
```

- `// test:` — either `diagnostics` or `execution`. Required.
- `// stdlib:` — `false` for unit-like diagnostic tests that don't need stdlib
  (faster, fewer moving parts); `true` when the test uses stdlib types
  (`Int64`, `String`, `Array`, `Char`, etc.). Required.
- Module declaration follows a blank line. Any name works; single-module tests are
  conventional.

### `diagnostics` kind

Static checks only — the harness runs BIND → infer → VALIDATE and compares emitted
diagnostics against `// ERROR:` annotations. No code is executed.

```ks
// test: diagnostics
// stdlib: false

module Test

func add(x: lang.i64, y: lang.i64) -> lang.i64 {
    x  // no error expected here; `x` is i64, function returns i64
}
```

With an expected error:

```ks
// test: diagnostics
// stdlib: false

module Test

struct Foo { var x: lang.i64; var y: lang.i64; }

func test() -> Foo {
    Foo(x: 1) // ERROR: struct 'Foo' has 2 field(s), but 1 argument(s) were provided
}
```

### `execution` kind

The harness compiles the program, runs it, and checks the process exit code.

```ks
// test: execution
// stdlib: true
// expect-exit: 0

module Test

import std.num.Int64

func main() -> lang.i64 {
    if 2 + 2 != 4 { return 1 }
    0
}
```

- `// expect-exit: <N>` — required for `execution` tests. `0` = pass; other values
  are useful when the test is verifying a specific failure/assertion branch.
- `main() -> lang.i64` is the entrypoint. Return non-zero from branches you want
  to signal "this case failed" — then cross-reference the exit code in the test
  name or a comment.

## `// ERROR:` annotation style

- **Substring match** against the diagnostic message.
- **Place on the same line** as the offending token.
- **Write the full expected message**, not a minimal substring. A single word like
  `// ERROR: label` keeps passing if the diagnostic changes to unrelated text —
  the test silently stops verifying what it was meant to verify.
- For long or churn-prone messages, use a long distinctive prefix (enough that no
  other diagnostic would match), not one word.

Good:
```
let x = foo(a: 1); // ERROR: struct 'Foo' has 2 field(s), but 1 argument(s) were provided
```

Avoid:
```
let x = foo(a: 1); // ERROR: label
```

See `lib2/kestrel-test-suite/AGENTS.md` for the authoritative rule.

## Where to put the file

`lib2/kestrel-test-suite/testdata/` is organized by feature category. Pick the
directory that best describes **what the test exercises**, not where the bug was:

```
attributes/      declarations/   execution/         inference/
builtins/        diagnostics/    execution_graph/   instantiation/
codegen/         expressions/    memory_model/      mir/
patterns/        statements/     stdlib/            types/
validation/
```

Inside a category, group by feature (e.g., `expressions/calls/`,
`patterns/range_matchable/`). Add a new subfolder if none fits.

Test filenames are **snake_case and descriptive** — they show up in triage output
and should read on their own:

```
call_function_with_single_param.ks
char_range_inclusive.ks
struct_init_wrong_arg_count.ks
```

Avoid generic names like `test1.ks` or `basic.ks`.

No `mod.rs` update is needed — the harness discovers files by walking
`testdata/`.

## Process

1. **Understand the feature.** Read the user's request; if needed, consult
   `docs/language/{feature}.md` and any `docs/plans/{feature}/` design docs.
   If you're writing the Kestrel program itself, use the `write-kestrel` skill
   for syntax/idiom guidance.

2. **Search for existing coverage.** Before writing, grep `testdata/` for the
   feature name and related symbols. Do not duplicate existing tests; extend or
   add adjacent cases instead.

3. **Find a similar test to mirror.** Look in the same category directory for a
   test with a similar shape (same kind, similar header, similar module layout).
   Mirror its style unless you have a reason to deviate.

4. **Decide kind and stdlib.** Static rule / diagnostic message / symbol shape →
   `diagnostics`, usually `stdlib: false`. Runtime behavior, codegen, pattern
   matching semantics → `execution`, usually `stdlib: true`.

5. **Write the smallest program that exercises the case.** One concept per file.
   If the bug has several facets, write several files — each with a name that
   describes exactly the case it covers.

6. **Run via triage.** Never invoke `cargo test -p kestrel-test-suite2` or the
   `file_tests-*` binary directly — the `/triage` skill records runs in
   `.triage/triage.db`, handles background execution, and is safe alongside
   other agents. Pass a targeted pattern while iterating; save the full suite
   for pre-commit.

7. **Read the triage results.** Pass/fail counts and failure messages — a
   started run with ignored output is not a test run.

## What not to do

- Don't write Rust test code under `tests/` — the harness runs `.ks` files. New
  Rust tests are almost never the right answer for feature coverage.
- Don't test the same diagnostic message twice across files. One canonical
  reproduction is enough; additional cases should cover *different* trigger
  paths.
- Don't rely on line-number-insensitive assertions like "has some error" when a
  specific `// ERROR:` would pin the exact site.
- Don't set `stdlib: true` by default — `false` is faster when the test doesn't
  import from `std.*`.
- Don't invent new test kinds — only `diagnostics` and `execution` exist today.

## Quick checklist

- [ ] File lives under the right `testdata/<category>/<subfolder>/` path
- [ ] Filename is descriptive snake_case ending in `.ks`
- [ ] Header: `// test: ...` + `// stdlib: ...` (+ `// expect-exit:` for execution)
- [ ] Module declaration after blank line
- [ ] Every expected error has `// ERROR: <full distinctive message>` on the right line
- [ ] No duplicate coverage of the same case
- [ ] Ran via `/triage` with a targeted pattern, read the results
