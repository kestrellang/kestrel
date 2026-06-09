# Quick Reference

File paths and commands for common tasks. All paths relative to the repo root.

## Pipeline tasks

| Task | File |
|------|------|
| Add a token / keyword | `lib/kestrel-lexer/src/lib.rs` |
| Add a `SyntaxKind` | `lib/kestrel-syntax-tree/src/` |
| Add a parser for a declaration / expression | `lib/kestrel-parser/src/` |
| Add an AST type or body node | `lib/kestrel-ast/src/` |
| Add a component to declaration entities | `lib/kestrel-ast-builder/src/components.rs` |
| Register a new `NodeKind` | `lib/kestrel-ast-builder/src/components.rs` (`NodeKind` enum) |
| Build an entity from a CST node | `lib/kestrel-ast-builder/src/` |
| Resolve a name from scope | `lib/kestrel-name-res/src/` |
| Extend HIR (new `HirExpr` / `HirStmt` / `HirPat`) | `lib/kestrel-hir/src/body.rs` |
| Lower AST body → HIR body | `lib/kestrel-hir-lower/src/` |
| Lower a type annotation | `lib/kestrel-hir-lower/src/` (`LowerTypeAnnotation`) |
| Add a type-inference `Constraint` | `lib/kestrel-type-infer/src/constraint.rs` |
| Add a solver rule for a constraint | `lib/kestrel-type-infer/src/solver.rs` |
| Add an `InferError` variant | see `lib/kestrel-type-infer/AGENTS.md` (updates **5** files) |
| Add a body-level analyzer | `lib/kestrel-analyze/src/body/<name>.rs` |
| Add a declaration-level analyzer | `lib/kestrel-analyze/src/decl/<name>.rs` |
| Add a whole-compilation analyzer | `lib/kestrel-analyze/src/compilation/<name>.rs` |
| Register an analyzer | `lib/kestrel-analyze/src/lib.rs` (`default_analyzers`) |
| MIR types (`Place`, `Rvalue`, `Terminator`) | `lib/kestrel-mir/src/` |
| Lower entities → MIR | `lib/kestrel-mir-lower/src/` |
| Type layout | `lib/kestrel-codegen/src/layout.rs` |
| Symbol mangling | `lib/kestrel-codegen/src/mangle.rs` |
| Cranelift codegen | `lib/kestrel-codegen-cranelift/src/` |
| Monomorphization | `lib/kestrel-codegen-cranelift/src/` |
| Diagnostic formatting | `lib/kestrel-reporting/src/` |

## Tests and stdlib

| Task | Location |
|------|----------|
| Add a `.ks` test | `lib/kestrel-test-suite/testdata/<category>/<subdir>/<name>.ks` |
| Test harness internals | `lib/kestrel-test-suite/src/` |
| Test format conventions | `lib/kestrel-test-suite/AGENTS.md` |
| Stdlib source (Kestrel code) | `lang/std/<module>/<type>.ks` |

Testdata categories (`lib/kestrel-test-suite/testdata/`):

```
attributes/   builtins/        codegen/         declarations/
diagnostics/  execution/       execution_graph/ expressions/
inference/    instantiation/   memory_model/    mir/
patterns/     statements/      stdlib/          types/
validation/
```

## hECS API cheatsheet

Inside a query (`fn execute(&self, ctx: &QueryContext)`):

| Call | Purpose |
|------|---------|
| `ctx.get::<C>(entity)` | Fetch component `C` from an entity. Returns `Option<&C>`. |
| `ctx.parent_of(entity)` | Walk up the entity tree. |
| `ctx.iter_component::<C>()` | Iterate every entity that has component `C`. |
| `ctx.query(OtherQuery { … })` | Call another memoized query. Results cached per revision. |
| `registry.0.find_body_check(id)` | Look up an analyzer by id (from `AnalyzerRegistryRef`). |

## Test annotations

```kestrel
// test: diagnostics        // or: compiles, runs
// stdlib: false             // opt out of stdlib for unit-ish diagnostic tests

module Main

struct Foo { let x: Int64 }

func main() -> Unit {
    Foo()        // ERROR: struct 'Foo' has 1 field(s), but 0 argument(s) were provided
}
```

- `// ERROR:` is a substring match; write the full distinctive message.
- Place the annotation on the same line as the offending token.
- See `lib/kestrel-test-suite/AGENTS.md` for the full conventions.

## Useful commands

```bash
# Run tests (never `cargo test -p kestrel-test-suite`)
triage
triage <pattern>
triage --failures

# Verbose debug traces in the compiler
VERBOSE_DEBUG_OUTPUT=1 triage <pattern>

# Format / lint / check
cargo fmt
cargo clippy
cargo check

# Unit tests for a single crate (fine)
cargo test -p kestrel-codegen
cargo test -p kestrel-type-infer
```

Package names in `lib/` have a `2` suffix in `Cargo.toml` (`kestrel-compiler`, `kestrel-codegen`, `kestrel-test-suite`, …) — the directory names don't. Use the package name with `-p`.

## `NodeKind` catalogue

```rust
pub enum NodeKind {
    Module,
    Struct,
    Enum,
    EnumCase,
    Protocol,
    Extension,
    Function,
    Initializer,
    Deinit,
    Field,
    Setter,         // getter lives on the Field itself
    Subscript,
    TypeAlias,
    Import,
    TypeParameter,
    ParamDefault,   // default-value expression for a parameter
}
```

(`lib/kestrel-ast-builder/src/components.rs`)

## Common components on declaration entities

| Component | Meaning |
|-----------|---------|
| `NodeKind` | Discriminant (always present). |
| `Name(String)` | Declared identifier. |
| `DeclSpan(Span)` | Declaration's own span. |
| `CstNode(SyntaxNode)` | Backing CST reference. |
| `FileId(Entity)` | Owning source file entity. |
| `Vis` | Public / Private / Internal / Fileprivate. |
| `Typed` (marker) | Can appear in type position. |
| `TypeAnnotation(AstType)` | Has a declared type (fields, params, alias targets). |
| `Callable` | Parameter list + receiver convention. |
| `Valued(SyntaxNode)` / `Body(AstBody)` | Has a body/initializer (pre- / post-lower). |
| `Gettable` / `Settable` (markers) | Read / write capability. |
| `Static` (marker) | Accessed via type, not instance. |
| `Subscript` (marker) | Call-syntax accessor. |
| `Computed` (marker) | Field is computed (get/set accessors). |
| `TypeParams(Vec<Entity>)` | Generic parameter entities. |
| `WhereClause(Vec<WhereConstraint>)` | Where-clause constraints. |
| `FieldMutability` | `Var` / `Let`. |

Authoritative list: `lib/kestrel-ast-builder/src/components.rs`.

## Reading inference results

Given a body entity, these queries give you the data an analyzer typically needs:

| Query | Output |
|-------|--------|
| `LowerBody { entity, root }` | `Option<HirBody>` — desugared body with scoped names resolved. |
| `InferBody { entity, root }` | `Option<TypedBody>` — types, resolved members, promotions. |
| `WhereClausesOf { entity, root }` | Resolved where-clause constraints in scope. |
| `Analyze { analyzer, entity, root }` | Diagnostics from one analyzer on one entity. |

See each crate's `docs/architecture.md` for the full query list.

## Where the "big bags" live

| Bag | Location |
|-----|----------|
| Analyzer registry | `AnalyzerRegistry` — built by `default_analyzers()` in `lib/kestrel-analyze/src/lib.rs`. |
| Diagnostic descriptors | `static DESCRIPTORS: &[DiagnosticDescriptor]` at the top of each analyzer file. |
| Constraint enum | `lib/kestrel-type-infer/src/constraint.rs`. |
| `InferError` enum | `lib/kestrel-type-infer/src/error.rs`. |
| `MirTy` / `Statement` / `Terminator` | `lib/kestrel-mir/src/`. |
| `HirExpr` / `HirStmt` / `HirPat` | `lib/kestrel-hir/src/body.rs`. |
