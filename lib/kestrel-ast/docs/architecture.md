# kestrel-ast Architecture

AST data types for the Kestrel compiler. Defines the typed representations of expressions, statements, patterns, and types that are extracted from the CST during AST building and consumed by HIR lowering.

## Pipeline Position

```
Source Text → Tokens → CST (rowan) → AST Build → Name Res → HIR Lower → Type Infer
                                      ^^^
                              this crate's types are produced here
```

The AST builder creates entities with components, and expressions/statements/patterns are stored as `AstBody` values inside `Valued` components. HIR lowering reads these types and produces desugared HIR.

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `AstExpr` | `expr.rs` | 19+ expression variants (literals, calls, operators, control flow, closures) |
| `AstStmt` | `stmt.rs` | 4 statement variants (let, expr, guard-let, deinit) |
| `AstPat` | `pat.rs` | 11 pattern variants (wildcard, binding, tuple, enum, struct, ...) |
| `AstType` | `ty.rs` | 10 type variants (named, tuple, function, optional, result, ...) |
| `AstBody` | `body.rs` | Container: arena-stored exprs/stmts/pats with top-level statements + tail expr |
| `Arena<T>` | `arena.rs` | Flat storage indexed by `Idx<T>` — no heap-allocated trees |
| `PathSegment` | `ty.rs` | Segment in a qualified type path: name + optional type args |

## Arena-Based Storage

All AST nodes are stored in flat arenas and addressed by typed indices:

```
AstBody {
    exprs: Arena<AstExpr>,     // indexed by Idx<AstExpr>
    stmts: Arena<AstStmt>,     // indexed by Idx<AstStmt>
    pats:  Arena<AstPat>,      // indexed by Idx<AstPat>
    statements: Vec<AstStmtId>,
    tail_expr: Option<AstExprId>,
}
```

Nodes reference each other by index, not pointers. This is cache-friendly and makes the body trivially cloneable.

## Detailed Type Documentation

| Document | Contents |
|----------|----------|
| [ast-types.md](ast-types.md) | `AstType` — 10 type variants with syntax examples |
| [ast-expressions.md](ast-expressions.md) | `AstExpr` — 19+ expression variants, operator enums |
| [ast-statements.md](ast-statements.md) | `AstStmt` — 4 statement types, tail expressions |
| [ast-patterns.md](ast-patterns.md) | `AstPat` — 11 pattern types, destructuring syntax |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Crate root, re-exports |
| `expr.rs` | `AstExpr` (19+ variants), `BinaryOp`, `UnaryOp`, `CompoundAssignOp`, `CallArg` |
| `stmt.rs` | `AstStmt` (4 variants) |
| `pat.rs` | `AstPat` (11 variants), `LitPatKind`, `EnumPatArg`, `StructPatField` |
| `ty.rs` | `AstType` (10 variants), `PathSegment` |
| `body.rs` | `AstBody` container with arenas |
| `arena.rs` | `Arena<T>`, `Idx<T>` — typed arena storage |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-span` | `Span` on all AST nodes |
