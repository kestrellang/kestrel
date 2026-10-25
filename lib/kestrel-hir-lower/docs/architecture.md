# kestrel-hir-lower Architecture

HIR lowering for the Kestrel compiler. Lowers a declaration's body **straight
from its CST** (the typed views of `kestrel-syntax-tree`) plus name-resolution
results into `HirBody` — a desugared, partially-resolved IR that type inference
can process — and records a `BodySourceMap` linking HIR ids back to syntax.
There is no intermediate body AST.

## Pipeline Position

```
Source Text → Tokens → CST ─┬─ Decl Build (ECS) → Name Res ─┐
                            │                                ▼
                            └──── body syntax (Valued ptr) → HIR Lowering → Type Infer → …
                                                             ^^^
                                                          this crate
```

A body entity carries `Valued(SyntaxNodePtr)`; `LowerBodyWithSourceMap`
resolves it against the file's `FileSyntax` green tree **on demand**, inside the
query, so a body is lowered the first time something asks for it.

## Three Kinds of Work

1. **Path resolution** — locals by scope (`lookup_local`), everything else through
   name resolution queries, to `LocalId` or `Entity`. Whether `a.b` is a value
   path or a member access is decided here, by scope.
2. **Desugaring** — operators to `ProtocolCall`, `for`/`while`/`if let`/`guard
   let`/`try`/`throw`/string interpolation to core HIR, sugar types to `Named`.
3. **Local variable allocation** — `LocalId` slots for parameters, let bindings,
   pattern bindings and desugaring temporaries.

What this crate does **not** do: method/field resolution, overload resolution,
type checking. Those are deferred to type inference.

## Core Types

| Type | Description |
|------|-------------|
| `LowerCtx` | Lowering context: arenas, scope stack, owner entity, file id, source map being built |
| `BodySourceMap` | HIR expr/pat/stmt ids ↔ `SyntaxNodePtr`, `LocalId` → declaring identifier (`LocalSource`), identifier position → local / path-segment expression |
| `LoweredBody` | `{ body: Arc<HirBody>, source_map: Arc<BodySourceMap> }` (both `Send + Sync`) |
| `syntax::BlockSyntax` / `Cond` / `PathSyntax` / `ExprSrc` / `PatSrc` | The few multi-child syntax shapes lowering consumes (blocks, condition lists, paths vs member chains, "a node or the span of a missing one") |

## Queries

| Query | Input | Output |
|-------|-------|--------|
| `LowerBodyWithSourceMap` | Entity with `Valued` | `Option<Arc<LoweredBody>>`; files the body's diagnostics |
| `LowerBody` | Entity with `Valued` | `Option<Arc<HirBody>>` — projects the above (type inference, analyzers, MIR) |
| `LowerTypeAnnotation` | Entity with `TypeAnnotation` | `HirTy` |
| `LowerCallableTypes` | Entity with `Callable` | Parameter types, `None` per unannotated param |
| `LowerCallableReturnType` | Callable entity | `HirTy` — explicit `-> T` if annotated, else unit `()` |
| `LowerExtensionTargetTypeArgs` | Entity with `ExtensionTarget` | Target type args as `HirTy` |

## Module Map

| File | Responsibility |
|------|---------------|
| `lib.rs` | Queries, body-node dispatch (`CodeBlock` / `FunctionBody` / `DefaultValue` / initializer `Expression`), parameters |
| `syntax.rs` | Reading the CST: block items, closure items, conditions, paths, arguments, closure params, tuple patterns, operator tokens, implicit `it` |
| `source_map.rs` | `BodySourceMap`, `LocalSource`, `token_at` |
| `ctx.rs` | `LowerCtx`: arenas, scopes, local allocation, init-effect wrapping |
| `expr.rs` | Expression lowering, path resolution, call shape detection, blocks, closures, condition chains |
| `stmt.rs` | Statement lowering (let, expr, guard, deinit) |
| `pat.rs` | Pattern lowering, literal parsing utilities |
| `desugar.rs` | Operator/loop/try/throw/interpolation desugaring |
| `literal.rs`, `string_token.rs` | String literal classification, indentation stripping, escape decoding |
| `format_spec.rs` | Interpolation format specifiers |
| `ty.rs` | Type lowering (sugar resolution, path types) |

## Design Decisions

See [design.md](design.md) for detailed rationale on:

- Lowering from the CST, and what missing syntax lowers to
- The source map and what it does (not) record
- Call shape detection: method vs direct (scope of the first path segment)
- Self type resolution walking the owner hierarchy
- Type alias transparency for simple aliases

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | ECS world and query context |
| `kestrel-syntax-tree`, `rowan` | Typed CST views, `SyntaxNodePtr`, text ranges |
| `kestrel-hir` | `HirBody`, `HirExpr`, `HirStmt`, `HirPat`, `HirTy` |
| `kestrel-ast` | `AstType` (type syntax), operator enums, escape table, `Arena` |
| `kestrel-ast-builder` | Components (`Valued`, `Callable`, `TypeAnnotation`, `FileSyntax`, …), `ast_type_from_cst` |
| `kestrel-name-res` | Resolution queries (`ResolveValuePath`, `ResolveTypePath`, `ResolveBuiltin`, …) |
| `kestrel-span` | `Span` for source locations |
| `kestrel-debug` | `ktrace!` for debug tracing |
