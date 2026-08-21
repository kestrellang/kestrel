# Contributing to Kestrel

Welcome. This guide orients new contributors to the Kestrel compiler — the active codebase lives in `lib/` and is organized as a hierarchical entity-component system (hECS) with memoized queries.

## Where to start

| Goal | Read |
|------|------|
| Understand how compilation works end-to-end | [Architecture](architecture.md) |
| Find the file that handles the thing you're changing | [Quick Reference](quick-reference.md) |
| Match the conventions of the existing code | [Patterns](patterns.md) |
| Follow a step-by-step guide for common tasks | [Workflows](workflows.md) |
| Work on the type system, inference, generics | [Type Inference](type-inference.md) |
| Understand or change how symbols are mangled | [Mangling](mangling.md) |
| Branches, the release train, the Project board, PRs, issues | [Workflow](git.md) |

## Codebase overview

```
kestrel/
├── lib/                               # Active compiler — work here
│   ├── kestrel-lexer/                  # Tokenization (logos)
│   ├── kestrel-parser/                 # Event-driven parser
│   ├── kestrel-syntax-tree/            # Lossless CST (rowan)
│   ├── kestrel-ast/                    # Arena-allocated AST types
│   ├── kestrel-ast-builder/            # CST → hECS entities + components
│   ├── kestrel-hecs/                   # Entity/component/query runtime
│   ├── kestrel-name-res/               # Name resolution queries
│   ├── kestrel-hir/                    # Body HIR (HirExpr, HirPat, HirStmt)
│   ├── kestrel-hir-lower/              # AST body → HIR body
│   ├── kestrel-type-infer/             # Constraint-based inference
│   ├── kestrel-semantics/              # Higher-level semantic queries
│   ├── kestrel-copy-fold/              # Copyable/Cloneable/NotCopyable decision kernel
│   ├── kestrel-analyze/                # Roslyn-style analyzers
│   ├── kestrel-pattern-matching/       # Exhaustiveness checking
│   ├── kestrel-mir/                    # MIR types, passes, mono, layout, mangling
│   ├── kestrel-mir-lower/              # Entities → MIR
│   ├── kestrel-codegen/                # Backend-agnostic target configuration
│   ├── kestrel-codegen-cranelift/      # Cranelift backend
│   ├── kestrel-codegen-llvm/           # LLVM backend (inkwell / LLVM 18)
│   ├── kestrel-compiler/               # Query engine / World owner
│   ├── kestrel-compiler-driver/        # High-level orchestration
│   ├── kestrel-debug/                  # Introspection helpers
│   ├── kestrel-doc/                    # Stdlib/API doc extraction (JSON)
│   ├── kestrel-lsp/                    # Language server (LSP over stdio)
│   ├── kestrel-reporting/              # Diagnostic formatting
│   ├── kestrel-span/                   # Source locations
│   ├── kestrel-test-suite/             # .ks-file test runner
│   └── AGENTS.md                       # Per-crate docs convention
│
├── lang/std/                           # Standard library (Kestrel source)
│
├── docs/
│   ├── contributing/                   # You are here
│   └── language/                       # User-facing language guide
│
└── .claude/skills/                     # Detailed internal references
    ├── hecs/                           #   — hECS concepts and rationale
    └── …                               #   — inference, pipeline routing, etc.
```

Each lib crate has its own `docs/` folder with an `architecture.md` entry point; some have `AGENTS.md` files with invariants and "watch-out-for" notes. When you need details deeper than this guide offers, those are the next stop.

## Getting started

1. **Read [Architecture](architecture.md)** to understand the pipeline and the hECS vocabulary.
2. **Browse the crate you'll be touching** — each has `docs/architecture.md`.
3. **Scan [Patterns](patterns.md)** so your code matches existing style.
4. **Find the right files** via [Quick Reference](quick-reference.md).
5. **Follow a workflow** from [Workflows](workflows.md) if your task is covered there.

## Running tests

Kestrel uses a dedicated test harness for `.ks` files. **Do not call `cargo test` directly** for the test suite — the harness integrates with the `triage` CLI, which records results in `.triage/triage.db`, supports background runs, and is safe alongside other developers or agents working in the same tree.

```bash
# Run the whole suite
triage

# Run a targeted subset
triage <pattern>

# Rerun only failures from the last run
triage --failures
```

`cargo test -p <crate>` is fine for individual crates' unit tests (e.g. the mangler's tests), but not for `kestrel-test-suite`. See `lib/kestrel-test-suite/AGENTS.md` for details on the test file format and annotations.

## Debugging

Debug tracing is the `ktrace!` macro from `kestrel-debug`, gated on `KESTREL_DEBUG` with comma-separated categories:

```bash
KESTREL_DEBUG=infer,solver triage <pattern>
KESTREL_DEBUG=all triage <pattern>
```

Categories are free-form strings chosen at each call site. In use today: `infer`, `solver`, `hir-lower`, `copyable`, `static-wf`, `ref-gate`, `op-shape`, `dangle`, `arm-decay`. When you need to trace something new, add `ktrace!` calls — not `eprintln!`/`println!` — so the output stays filterable.

## Asking for help

- `.claude/skills/` holds deeper skill documents for specialized topics (hECS internals, the inference pipeline, debugging playbook, etc.).
- Each lib crate's `AGENTS.md` and `docs/` folder is the first stop for that crate's nuances.
- File an issue before starting non-trivial work (see [Workflow](git.md)).
