# Kestrel Compiler Architecture

## Compilation Pipeline

```
Source Code
    |
[LEXER]  -> Tokens                           [kestrel-lexer]
    |
[PARSER] -> Events -> TreeBuilder -> CST     [kestrel-parser]
    |
[BUILD]  -> Symbols from syntax              [kestrel-semantic-tree-builder]
    |
[BIND]   -> Resolve types, bodies            [kestrel-semantic-tree-binder]
    |
[VALIDATE] -> Semantic checks                [kestrel-semantic-analyzers]
    |
[CODEGEN] -> Execution graph -> Cranelift    [kestrel-compiler]
```

## Phase Responsibilities

| Phase | Crate | Purpose | Mutations Allowed |
|-------|-------|---------|-------------------|
| BUILD | kestrel-semantic-tree-builder | Create symbols from syntax | Symbol creation, parent/child, syntax_map |
| BIND | kestrel-semantic-tree-binder | Resolve references, types, bodies | Type resolution, callable signatures |
| VALIDATE | kestrel-semantic-analyzers | Check semantic constraints | Diagnostics only (read-only) |

## Key File Locations

| Task | File Path |
|------|-----------|
| Add token/keyword | `lib/kestrel-lexer/src/lib.rs` |
| Add syntax node kind | `lib/kestrel-syntax-tree/src/lib.rs` |
| Add parser for feature | `lib/kestrel-parser/src/{feature}/mod.rs` |
| Add to declaration items | `lib/kestrel-parser/src/declaration_item/mod.rs` |
| Add semantic symbol | `lib/kestrel-semantic-tree/src/symbol/{name}.rs` |
| Add symbol kind | `lib/kestrel-semantic-tree/src/symbol/kind.rs` |
| Add builder (BUILD) | `lib/kestrel-semantic-tree-builder/src/builders/{name}.rs` |
| Register builder | `lib/kestrel-semantic-tree-builder/src/lowerer.rs` |
| Add binder (BIND) | `lib/kestrel-semantic-tree-binder/src/binders/{name}.rs` |
| Register binder | `lib/kestrel-semantic-tree-binder/src/declaration_binder.rs` |
| Body resolution | `lib/kestrel-semantic-tree-binder/src/body_resolver/mod.rs` |
| Add analyzer (VALIDATE) | `lib/kestrel-semantic-analyzers/src/analyzers/{name}/mod.rs` |
| Add test | `lib/kestrel-test-suite/tests/{name}.rs` |

For detailed architecture, see: `docs/contributing/architecture.md`
For step-by-step workflows, see: `docs/contributing/workflows.md`
