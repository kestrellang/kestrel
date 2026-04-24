# Kestrel Code Patterns

Conventions and idioms used throughout the lib codebase. Follow these by default; when you deviate, the deviation itself should be the point.

## Naming

| Thing | Convention | Examples |
|-------|------------|----------|
| Crate | `kestrel-<role>` (dir name), `kestrel-<role>2` (package name) | `kestrel-hir`, `kestrel-type-infer2` |
| Token | PascalCase matching the keyword | `Module`, `Struct`, `Func`, `Public` |
| `SyntaxKind` node | `{Feature}Declaration`, `{Feature}Body`, `{Type}Expr`, `{Type}Stmt`, `{Type}Pat` | `StructDeclaration`, `BinaryExpr`, `LetStmt` |
| Component | Noun describing a capability or fact | `Name`, `Callable`, `TypeAnnotation`, `WhereClause`, `Computed` |
| Query type | Verb-or-noun struct, UpperCamelCase | `LowerBody`, `InferBody`, `ResolveName`, `WhereClausesOf` |
| Analyzer | `{Purpose}Analyzer` | `ExhaustiveReturnAnalyzer`, `ConformanceRulesAnalyzer` |
| Diagnostic id | `E` + digits, grouped by category | `E001` missing return; `E310`–`E315` match patterns |
| `MirTy` variant | Short primitive tag or PascalCase composite | `I64`, `Bool`, `Named`, `FuncThin`, `AssociatedProjection` |
| Constraint variant | UpperCamelCase | `Equal`, `Coerce`, `Conforms`, `Member` |

Markers (component structs with no fields) are nouns or adjectives — `Typed`, `Static`, `Gettable`, `Computed`.

## Query authoring

Every derived fact in lib is a query. The shape of one:

```rust
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct LowerBody {
    pub entity: Entity,
    pub root: Entity,
}

impl QueryFn for LowerBody {
    type Output = HirBody;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        // Read components and call sub-queries — never mutate the world.
        let ast = ctx.get::<Body>(self.entity)?.0.clone();
        // ...
    }

    fn describe(&self) -> String {
        format!("LowerBody({:?})", self.entity)
    }
}
```

Rules of thumb:

1. **Queries are pure.** No side effects other than accumulating diagnostics via the framework-provided channels.
2. **Inputs on the struct, everything else via `ctx`.** The `(query_struct, revision)` pair is the cache key; putting runtime state on the struct breaks memoization.
3. **Prefer calling sub-queries to re-reading components.** `ctx.query(OtherQuery { … })` is memoized and participates in change tracking; open-coding the work defeats the cache.
4. **`root` is always passed through.** It's the compilation root entity; most queries need it to find registries.
5. **Fail soft.** If a required component is missing, return `None` / `vec![]` / `Ty::Error` — don't panic. Upstream passes emit diagnostics; downstream queries handle Error types gracefully.

## Adding a component

1. Define the struct in `lib/kestrel-ast-builder/src/components.rs` (or a topical sibling module if the group is large). `#[derive(Clone, Debug, PartialEq, Eq, Hash)]`; `Hash` + `Eq` are required for fingerprinting.
2. Set it on the entity during AST build, next to the other capability components for that `NodeKind`.
3. Read it elsewhere via `ctx.get::<YourComponent>(entity)`.

Prefer **more, smaller** components over fewer bigger ones. If a capability is ever optional, it's its own component.

## Analyzer shape

All analyzers follow the same skeleton. Example from `lib/kestrel-analyze/src/body/exhaustive_return.rs`:

```rust
static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E001",
    name: "missing_return",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ExhaustiveReturnAnalyzer;

impl Describe for ExhaustiveReturnAnalyzer {
    fn id(&self) -> &'static str { "exhaustive_return" }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] { DESCRIPTORS }
}

impl BodyCheck for ExhaustiveReturnAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // 1. Filter: is this the right NodeKind? return vec![] if not.
        // 2. Filter: are upstream errors suppressing us? return vec![] if so.
        // 3. Do the analysis against cx.hir / cx.typed.
        // 4. Return diagnostics.
    }
}
```

Conventions:

- Analyzers are **ZSTs** (no fields). All state is in the query context.
- One analyzer produces one or a small handful of related diagnostic ids.
- Document each diagnostic at the top of the file — message template, labels, notes, cascading behavior.
- **Suppress on upstream errors.** If `cx.typed.errors.is_empty()` is false, most downstream analyzers should bail — a type-mismatched body will fire spurious secondary diagnostics otherwise.
- **Filter early.** Bail in the first few lines if the analyzer doesn't apply to this entity.

`DeclCheck` has the same shape, plus `target_kinds(&self) -> &'static [NodeKind]` so the framework can skip irrelevant entities cheaply.

`CompilationCheck` runs once per compilation with a `CompilationContext` — use it for cross-entity work (cycles, global conflicts).

Register every new analyzer in `default_analyzers()` in `lib/kestrel-analyze/src/lib.rs`, in the correct section (body / decl / compilation).

## Diagnostic style

A Kestrel diagnostic is `message` + labels + optional notes.

- **Message:** one line, complete sentence, no trailing period. State the rule violated, with concrete identifiers quoted.
  - Good: `function 'f' does not return a value on all code paths`
  - Bad: `missing return value here`
- **Primary label:** narrow span at the offending token. Short text (`"missing return"`, `"unexpected type here"`).
- **Secondary labels:** additional spans that help locate the cause — the struct declaration, the conflicting conformance, the where-clause, etc.
- **Notes:** only when they add real information (the rule's rationale, a suggested fix). No filler.

Tests match `// ERROR:` substrings against the rendered message, so distinctive wording doubles as a regression anchor.

## Error recovery

The type system and MIR both have an `Error` variant designed as a **poison absorber**:

- `TyKind::Error` / `MirTy::Error` unifies with anything and silently propagates.
- When inference reports an error, it returns a fresh `Error` TyVar — downstream constraints see the absorber and stop firing cascades.
- Analyzers should skip when `cx.typed.errors.is_empty()` is false, for the same reason.

Don't panic when you see a broken input. Return `Error` / `None` / `vec![]` and let the user fix the root cause first.

## Reporting from the inference solver

Inside the solver (`lib/kestrel-type-infer/src/`), never accumulate diagnostics directly. Call `ctx.report_error(InferError::...)` — it returns an Error TyVar. Use the returned var as the result of the constraint-generation branch so downstream work sees the absorber. The accumulate path is reserved for hir-lower and decl-level analyzers.

## Where-clause queries

Where clauses belong to an entity's own scope. Never re-resolve them from a call site's context.

```rust
// Right:
let clauses = ctx.query(WhereClausesOf { entity: fn_entity, root });

// Wrong: trying to resolve where-clause names using the call-site's
// scope. You'll pick up the wrong bindings.
```

## Stdlib source (`lang/std/`)

Module path matches directory structure (`std.collections`, `std.core`, …). Public API uses `public`; internal state is `private`. Mutating methods are marked `mutating`. COW types (`String`, `Array`) call `makeUnique()` before `grow()` in mutating methods.

## Test patterns

See `lib/kestrel-test-suite/AGENTS.md` and [Workflows](workflows.md) for the test file format. Two rules worth repeating here:

1. **Write the full distinctive diagnostic message** in `// ERROR:` annotations. Substring matching allows minimal annotations, but minimal annotations drift silently when diagnostics are reworded.
2. **One idea per file.** A test file exercises one feature or one diagnostic — not a cluster.

## Common pitfalls

1. **Mutating the world outside `kestrel-ast-builder`.** Only the builder writes components. Everything downstream is queries.
2. **Forgetting to register an analyzer.** Without a line in `default_analyzers`, your analyzer exists but never runs.
3. **Adding an `InferError` in only one place.** New variants touch **five** files — see `lib/kestrel-type-infer/AGENTS.md`. The build won't catch you until a dependent crate is recompiled.
4. **Emitting a diagnostic from the solver via accumulate.** Use `report_error` so cascades get absorbed by `TyKind::Error`.
5. **Dispatching method calls without going through the funnel.** Method and protocol dispatch is routed through `emit_method_dispatch`; bypassing it is how you grow a second source of truth.
6. **Building on the dollar-suffixed name in MIR.** The mangler strips `$` disambiguators; if you're reading entity names elsewhere, decide explicitly whether you want the raw or the user-visible form.
7. **Panicking on missing components.** Return an empty result and let upstream diagnose the real problem.

For each of these, the nearest `AGENTS.md` in the affected crate has the fuller story.
