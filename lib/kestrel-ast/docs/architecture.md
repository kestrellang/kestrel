# kestrel-ast Architecture

Syntax-level data shared across the front end: type syntax (`AstType`), the
operator enums, the escape table, and the arena HIR stores its nodes in.

Function bodies have **no AST**: `kestrel-hir-lower` lowers them straight from the
CST (`docs/design/frontend.md`). What remains here is what declarations, HIR and
lowering share and what the deepest common crate must own.

## Pipeline Position

```
Source Text → Tokens → CST (rowan) → Decl Build → Name Res → HIR Lower → Type Infer
                                      ^^^^^^^^^^              ^^^^^^^^^
                   `AstType` for signatures / annotations;   operators, escapes,
                                                             `Arena` for HIR nodes
```

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `AstType` | `ast_type.rs` | Type syntax (named, tuple, function, optional, result, `some`, refs, …) — stored in `TypeAnnotation`/`Callable` components, lowered to `HirTy` |
| `PathSegment` | `ast_type.rs` | Segment of a qualified type path: name + type args |
| `BinaryOp` / `UnaryOp` / `PostfixOp` / `CompoundAssignOp` | `ops.rs` | Operators; `symbol()` is the one place a spelling is written. Keys of HIR's operator→protocol tables |
| `decode_escape`, `EscapeErrorKind` | `escape.rs` | **The** backslash-escape table (strings, chars, interpolation segments) |
| `Arena<T>` / `Idx<T>` | `arena.rs` | Flat storage indexed by typed `u32` ids — `HirBody`'s exprs/pats/stmts/locals |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Crate root, re-exports |
| `ast_type.rs` | `AstType`, `FnTypeKind`, `ParamConvention`, `PathSegment` |
| `ops.rs` | Operator enums and their spellings |
| `escape.rs` | Escape decoding, error kinds |
| `pretty.rs` | `format_type` — renders an `AstType` back to source (doc generator, diagnostics) |
| `arena.rs` | `Arena<T>`, `Idx<T>` |

## Detailed Type Documentation

| Document | Contents |
|----------|----------|
| [ast-types.md](ast-types.md) | `AstType` — type variants with syntax examples |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-span` | `Span` on type syntax |
