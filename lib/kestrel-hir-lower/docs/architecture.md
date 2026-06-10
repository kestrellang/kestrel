# kestrel-hir-lower Architecture

HIR lowering for the Kestrel compiler. Transforms AST entities + name resolution results into `HirBody` — a desugared, partially-resolved IR that type inference can process.

## Pipeline Position

```
Source Text → Tokens → CST → AST Build → Name Res → HIR Lowering → Type Infer → Codegen
                                                        ^^^
                                                     this crate
```

## Three Kinds of Work

1. **Path resolution** — calls name resolution queries to resolve names to `Entity` or `LocalId`
2. **Desugaring** — rewrites operators to `ProtocolCall`, for/while to `Loop`, sugar types to `Named`
3. **Local variable allocation** — assigns `LocalId` slots for parameters, let bindings, pattern bindings

What this crate does **not** do: method/field resolution, overload resolution, type checking. Those are deferred to type inference.

## Core Types

| Type | Description |
|------|-------------|
| `LowerCtx` | Lowering context: arenas, scope stack, current entity, references |
| `LowerBody` | Query: entity → `Arc<HirBody>` (main entry point) |
| `LowerTypeAnnotation` | Query: entity → `HirTy` (type annotation lowering) |
| `LowerCallableTypes` | Query: entity → per-param annotation types |
| `LowerCallableReturnType` | Query: callable entity → declared return type |
| `LowerExtensionTargetTypeArgs` | Query: extension entity → target type args |

## Queries

| Query | Input | Output |
|-------|-------|--------|
| `LowerBody` | Entity with `Valued` component | `Option<Arc<HirBody>>` (expressions, statements, patterns, locals; Arc-wrapped so memo cache hits share one allocation) |
| `LowerTypeAnnotation` | Entity with `TypeAnnotation` component | `HirTy` |
| `LowerCallableTypes` | Entity with `Callable` component | Parameter types, `None` per unannotated param |
| `LowerCallableReturnType` | Callable entity | `HirTy` — explicit `-> T` if annotated, else unit `()` (the single fallback rule; initializers count as unit) |
| `LowerExtensionTargetTypeArgs` | Entity with `ExtensionTarget` component | Target type args as `HirTy` (`Some(vec![])` when none; excess args beyond target arity become `HirTy::Error`, left to E453) |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Query definitions, `LowerCtx`, public API |
| `expr.rs` | Expression lowering (19+ AST variants → HIR) |
| `stmt.rs` | Statement lowering (let, expr, guard-let) |
| `pat.rs` | Pattern lowering (11 variants) |
| `ty.rs` | Type lowering (sugar resolution, path types) |
| `desugar.rs` | Operator → protocol call mapping, loop desugaring |

## Design Decisions

See [design.md](design.md) for detailed rationale on:

- Operator precedence via Pratt parser (applied here, not in the parser)
- Call shape detection: method vs direct (heuristic on first path segment)
- Self type resolution walking the owner hierarchy
- Type alias transparency for simple aliases
- Known limitations (incomplete destructuring, string interpolation, overloads)

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | ECS world and query context |
| `kestrel-hir` | `HirBody`, `HirExpr`, `HirStmt`, `HirPat`, `HirTy` |
| `kestrel-ast` | `AstBody`, `AstExpr`, `AstType`, operator enums |
| `kestrel-ast-builder` | Components (`Valued`, `Callable`, `TypeAnnotation`, etc.) |
| `kestrel-name-res` | Resolution queries (`ResolveName`, `ResolveTypePath`, etc.) |
| `kestrel-span` | `Span` for source locations |
| `kestrel-debug` | `ktrace!` for debug tracing |
