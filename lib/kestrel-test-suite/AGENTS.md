# kestrel-test-suite — patterns

## `// ERROR:` comments — match semantics and style

Diagnostic tests (`// test: diagnostics`) use `// ERROR: <text>` annotations on the line where an error is expected.

**Match rule (current harness):** the harness does a **substring** match — the annotation text must appear somewhere in the diagnostic message. It does not need to be the full message.

**Preferred style:** even though substring matching is permissive, **write the full expected error message** (or a substantial, distinctive slice of it) rather than a minimal substring. Reasons:
- A minimal substring like `// ERROR: label` will silently keep passing if the diagnostic changes to unrelated text that happens to contain "label" (e.g., a different feature's error) — the test stops verifying what it was meant to verify.
- Full messages make intent obvious to the next reader and to future diffs when diagnostics are reworded.
- Search-for-an-error across `testdata` is only useful when the expected text is the real text.

Good:
```
let x = foo(a: 1); // ERROR: struct 'Foo' has 2 field(s), but 1 argument(s) were provided
```

Avoid:
```
let x = foo(a: 1); // ERROR: label
```

When the exact wording is long or churns often, use a long distinctive prefix, not a single word.

## Test file format

- Header line 1: `// test: <kind>` — one of `diagnostics`, `mir`, or `execution` (`src/annotation.rs`).
  - `diagnostics` — checks `// ERROR:` annotations against emitted diagnostics.
  - `mir` — requires a clean compile, then compares lowered MIR against a snapshot (see the `mir-filter` / `mir-snapshot` headers and `src/mir_snapshot.rs`).
  - `execution` — requires a clean compile, then builds and runs the binary; exit code must match `// expect-exit:` (default 0), and `// expect-stdout:` / `// stdout-contains:` check output.
  - **WARNING: unknown kinds silently fall back to `diagnostics`.** `// test: runs` or `// test: compiles` won't error — the file just runs as a diagnostics test (and with no `// ERROR:` annotations, it typically passes vacuously). Double-check the kind spelling.
- Header line 2: `// stdlib: <true|false>` — opt out of stdlib for unit-like diagnostic tests that don't need it.
- Module declaration: `module <Name>` (any name; single-module tests are conventional).
- Place `// ERROR:` annotations on the same line as the offending token.

## Running tests

See the root `CLAUDE.md` and the `triage` skill — do not invoke `cargo test -p kestrel-test-suite` directly.
