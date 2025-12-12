# Contributing to Kestrel

This guide helps you understand the Kestrel compiler codebase and contribute effectively.

## Quick Navigation

| Document | Purpose | Use When |
|----------|---------|----------|
| [Architecture](architecture.md) | Compilation pipeline, crate relationships, data flow | Understanding how pieces fit together |
| [Quick Reference](quick-reference.md) | File paths, common imports, task locations | Looking up where to make changes |
| [Patterns](patterns.md) | Naming conventions, code patterns, testing | Writing code that matches existing style |
| [Workflows](workflows.md) | Step-by-step guides for common tasks | Adding features, diagnostics, tests |

## Related Documentation

Detailed implementation guides (in `.claude/commands/`):
- **write-feature.md** - Complete 7-step guide for adding language features (lexer through tests)
- **write-parser.md** - Detailed parser implementation guide
- **validation-passes.md** - Creating semantic validation passes

Language semantics (in `docs/semantics/`):
- Formal definitions of modules, functions, structs, protocols, etc.
- Type resolution and name resolution rules
- Error conditions and messages

## Codebase Overview

```
kestrel/
├── lib/
│   ├── kestrel-lexer/          # Tokenization (Logos)
│   ├── kestrel-parser/         # Event-driven parsing (Chumsky)
│   ├── kestrel-syntax-tree/    # Lossless CST (Rowan)
│   ├── kestrel-semantic-tree/  # Symbols, types, behaviors
│   ├── kestrel-semantic-model/ # SemanticModel + query system
│   ├── kestrel-semantic-tree-builder/  # BUILD/lowering (SyntaxNode -> SemanticModel)
│   ├── kestrel-semantic-tree-binder/   # BIND (resolve + body resolution)
│   ├── kestrel-semantic-analyzers/     # VALIDATE (post-bind analyzers)
│   ├── kestrel-compiler/       # High-level orchestration
│   ├── kestrel-test-suite/     # Integration tests
│   ├── kestrel-span/           # Source locations
│   ├── kestrel-prelude/        # Primitive types
│   ├── kestrel-reporting/      # Diagnostics
│   └── semantic-tree/          # Language-agnostic symbol infra
├── docs/
│   ├── contributing/           # This guide
│   └── semantics/              # Language semantics
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
