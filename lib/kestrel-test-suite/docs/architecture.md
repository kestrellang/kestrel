# kestrel-test-suite Architecture

The compiler's test framework: file-based `.ks` tests under `testdata/`, plus a
Rust API for tests that need several files or custom setup.

**Run it only through the `triage` skill**, never with `cargo test` directly
(see the root `CLAUDE.md`). How to write a test is in the `write-tests` skill
and `../AGENTS.md`; this page is about how the harness is built.

## Pipeline Position

```
testdata/**/*.ks ──► annotation parser ──► TestCompiler ──► whole pipeline ──► diagnostic matcher
                                                     └──► codegen + link + run (execution tests)
```

The suite sits on top of everything: it drives `kestrel-compiler` and
`kestrel-compiler-driver` exactly as the CLI does, then checks the result.

## Test Kinds

| Header | Checks |
|--------|--------|
| `// test: diagnostics` | Every emitted error and warning matches an `// ERROR:` / `// WARN:` annotation on its line, and every annotation matches something. No code runs |
| `// test: execution` | The program builds, runs, and exits with `// expect-exit:` (and prints `// expect-stdout:` / `// stdout-contains:`) |
| `// test: mir` | The MIR matches a golden snapshot (`// mir-snapshot:`, `// mir-filter:`) |

Other headers: `stdlib`, `executable`, `warnings`, `include`, `c-source`,
`skip`, `host`, and `backends` (run one trial per listed backend). An unknown
`test:` kind silently falls back to `diagnostics`.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `TestConfig` | `annotation.rs` | Parsed file headers |
| `TestMode` | `annotation.rs` | `Diagnostics`, `Mir`, `Execution` |
| `TestCompiler` | `compiler.rs` | Wraps the compiler with test assertions |
| Unified diagnostic + matcher | `diagnostic_matcher.rs` | Two-way matching of emitted diagnostics against annotations (`ERROR`, `ERROR(Exxx)`, `ERROR(Exxx): message`, `WARN`) |

## Key Design Decisions

- **The std is built once per process.** It is cloned into each test with
  `World::snapshot()`, so 1,500+ std bodies aren't re-inferred per test.
- **Only diagnostics located in the test file count.** Diagnostics inside the
  std are ignored by the matcher.
- **The backend comes from `KESTREL_BACKEND`** (`llvm` selects LLVM; anything
  else is Cranelift), unless the file pins `// backends:`.

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Shared std cache, public API |
| `annotation.rs` | Header and inline annotation parsing |
| `compiler.rs` | `TestCompiler` |
| `diagnostic_matcher.rs` | Annotation matching |
| `runner.rs` | Compile, link and run execution tests on the chosen backend |
| `mir_snapshot.rs` | MIR golden files |
| `tests/file_tests.rs` | Auto-discovers `testdata/**/*.ks` (datatest-stable) |
| `tests/api/` | Rust-API tests (imports, visibility) |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-compiler`, `kestrel-compiler-driver` | Running the pipeline |
| `kestrel-codegen-cranelift`, `kestrel-codegen-llvm` | Execution tests |
| `kestrel-mir` | MIR snapshots |
| `kestrel-analyze`, `kestrel-type-infer`, `kestrel-reporting`, `codespan-reporting` | Collecting and printing diagnostics |
| `kestrel-ast-builder`, `kestrel-hecs`, `kestrel-span` | World access, spans |
