# Contributing to Kestrel

This guide helps you understand the Kestrel compiler codebase and contribute effectively.

## Quick Navigation

| Document | Purpose | Use When |
|----------|---------|----------|
| [Architecture](architecture.md) | Compilation pipeline, crate relationships, data flow | Understanding how pieces fit together |
| [Quick Reference](quick-reference.md) | File paths, common imports, task locations | Looking up where to make changes |
| [Patterns](patterns.md) | Naming conventions, code patterns, testing | Writing code that matches existing style |
| [Workflows](workflows.md) | Step-by-step guides for common tasks | Adding features, diagnostics, tests |
| [Type Inference](type-inference.md) | Constraint solver, type substitutions, TyKind | Working on types, inference, generics |
| [Git](git.md) | Branching strategy, PRs, issues | Contributing code changes |

## Related Documentation

Detailed implementation guides (in `.claude/commands/`):
- **feature.md** - Complete workflow for adding language features (brainstorm, design, plan, implement)

Compiler internals (in `docs/internals/`):
- Parser architecture, execution graph, validation passes
- Type inference design, monomorphization
- Formal language semantics (modules, functions, structs, protocols, etc.)
- Type resolution and name resolution rules

Implementation plans (in `docs/plans/`):
- Feature-specific implementation plans organized by topic

## Codebase Overview

```
kestrel/
├── lib/
│   ├── kestrel-lexer/          # Tokenization (Logos)
│   ├── kestrel-parser/         # Event-driven parsing (Chumsky)
│   ├── kestrel-syntax-tree/    # Lossless CST (Rowan)
│   ├── kestrel-semantic-tree/  # Symbols, types, behaviors
│   ├── kestrel-semantic-model/ # SemanticModel + query system
│   ├── kestrel-semantic-tree-builder/  # BUILD (SyntaxNode -> SemanticModel)
│   ├── kestrel-semantic-tree-binder/   # BIND (resolve + body resolution)
│   ├── kestrel-semantic-analyzers/     # VALIDATE (post-bind analyzers)
│   ├── kestrel-semantic-type-inference/  # Hindley-Milner type inference
│   ├── kestrel-semantic-pattern-matching/ # Exhaustiveness & usefulness checking
│   ├── kestrel-execution-graph/          # MIR (mid-level IR)
│   ├── kestrel-execution-graph-lowering/ # Semantic model -> MIR lowering
│   ├── kestrel-codegen/        # Backend-agnostic codegen utilities (layout, mangling)
│   ├── kestrel-codegen-cranelift/ # Cranelift JIT backend (MIR -> native code)
│   ├── kestrel-compiler/       # High-level orchestration
│   ├── kestrel-test-suite/     # Integration tests
│   ├── kestrel-span/           # Source locations
│   ├── kestrel-prelude/        # Primitive types
│   ├── kestrel-reporting/      # Diagnostics
│   └── semantic-tree/          # Language-agnostic symbol infra
├── lang/
│   └── std/                    # Standard library (Kestrel source)
├── docs/
│   ├── language/               # User-facing language guide
│   ├── contributing/           # This guide
│   ├── internals/              # Compiler architecture & semantics
│   ├── memory-model/           # Runtime memory semantics
│   └── plans/                  # Implementation plans
└── src/main.rs                 # CLI entry point
```

## Getting Started

1. **Understand the pipeline**: Read [Architecture](architecture.md) first
2. **Find your task**: Use [Quick Reference](quick-reference.md) to locate relevant files
3. **Follow conventions**: Check [Patterns](patterns.md) before writing code
4. **Follow the workflow**: Use [Workflows](workflows.md) for step-by-step guidance

## Running Tests

```bash
# All tests
cargo test

# Specific crate
cargo test -p kestrel-parser
cargo test -p kestrel-test-suite

# Specific test file
cargo test -p kestrel-test-suite --test body_resolution

# Specific test
cargo test -p kestrel-test-suite call_instance_method
```
