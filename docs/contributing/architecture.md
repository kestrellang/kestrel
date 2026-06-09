# Kestrel Architecture (lib)

The Kestrel compiler lives under `lib/`. It is built on a **hierarchical entity-component system** (hECS) with memoized queries — declarations are entities, facts about them are components, and every derived result (name resolution, HIR, types, diagnostics, MIR) is produced by a query that the runtime caches and invalidates automatically.

This document describes the overall shape of the pipeline. For the motivation behind hECS see the `hecs` skill and `lib/AGENTS.md`; per-crate deep-dives live in each crate's own `docs/` folder.

## Compilation pipeline

```
Source text
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  LEX            kestrel-lexer                        │
│  Source → Token stream (logos)                       │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  PARSE          kestrel-parser + kestrel-syntax-tree │
│  Tokens → lossless CST (rowan)                       │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  AST BUILD      kestrel-ast-builder                  │
│  CST → entities + components in the hECS World       │
│  One entity per declaration; components describe     │
│  its syntax (Name, Callable, TypeAnnotation, Body,   │
│  Vis, WhereClause, TypeParams, …).                   │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  NAME RESOLUTION    kestrel-name-res                 │
│  Query-only: ResolveName, ResolveTypePath, etc.      │
│  Entity + name → entity (or diagnostic)              │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  HIR LOWER       kestrel-hir-lower → kestrel-hir     │
│  Per-body: AstBody → HirBody. Names resolvable       │
│  purely from scope are resolved; method/field names  │
│  stay as strings until type inference.               │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  TYPE INFER      kestrel-type-infer                  │
│  Per-body: HirBody → TypedBody. Constraint-based     │
│  solver with fixpoint iteration. Resolves method     │
│  calls, associated types, overload sets, promotions. │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  ANALYZE         kestrel-analyze                     │
│  Roslyn-style analyzers over entities and bodies:    │
│  BodyCheck, DeclCheck, CompilationCheck.             │
│  Produces diagnostics; does not mutate the world.    │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  MIR LOWER       kestrel-mir-lower → kestrel-mir     │
│  Entities + TypedBodies → MirModule (OSSA/SSA):      │
│  ValueIds, Instructions, BasicBlocks, Terminators,   │
│  ownership-checked. Generics stay generic until a    │
│  later monomorphization MIR pass.                    │
└──────────────────────────────────────────────────────┘
    │
    ▼
┌──────────────────────────────────────────────────────┐
│  CODEGEN         kestrel-codegen + …-cranelift       │
│  Monomorphization already ran as a MIR pass;         │
│  mangle, emit Cranelift IR, link.                    │
└──────────────────────────────────────────────────────┘
```

Each arrow is one or more memoized queries. Nothing in this pipeline is batch-imperative: analyzers don't run "a validate pass" over the world — they are queries that fire on demand and whose results are cached until the inputs change.

## The hECS world

Everything after parsing lives in a `World` (`kestrel-hecs`). The contributor-level vocabulary:

| Concept | What it is |
|---------|------------|
| `Entity` | A 32-bit handle. Every declaration gets one. |
| **Component** | Any `Clone + 'static` struct stored against an entity. Components are small and orthogonal — a function entity has `Name`, `Callable`, `Body`, optionally `WhereClause`, etc. |
| `NodeKind` | The discriminant component on every declaration entity (`Module`, `Struct`, `Enum`, `Protocol`, `Function`, `Field`, `TypeAlias`, …). |
| **Query** | A `QueryFn` impl. Inputs: entity + root. Outputs: some derived fact (HIR body, inferred type, diagnostics, MIR). The framework caches results keyed on `(query, revision)` and re-runs them when inputs fingerprint-differ. |
| **Revision** | A counter on the `World`. Bumped when the source changes. Feeds incremental invalidation. |

Because components are orthogonal, capability checks are "does this entity have component X?" instead of "what subclass is this symbol?" A computed property is a `Field` entity that also has a `Computed` marker. A static field is one that has a `Static` marker. Adding a new capability doesn't require editing a giant enum — you define a new component.

### Typical component set per declaration kind

| Kind | Likely components (beyond `NodeKind`, `Name`, `DeclSpan`, `CstNode`, `Vis`) |
|------|-----------------------------------------------------------------------------|
| Function / Initializer / Deinit | `Callable`, `Valued` (pre-lower) or `Body` (post-lower), optional `WhereClause`, `TypeParams` |
| Field | `TypeAnnotation`, `FieldMutability`, optionally `Computed`, `Static`, `Gettable`/`Settable`, `Valued` (default) |
| Struct / Enum / Protocol / TypeAlias | `Typed` marker, optional `TypeParams`, `WhereClause` |
| Subscript | `Callable`, `Subscript` marker, `Gettable`/`Settable` |
| Type parameter | `TypeParameter` component with its constraints |

The authoritative catalogue is `lib/kestrel-ast-builder/src/components.rs`.

## Crate map

| Crate | Responsibility |
|-------|----------------|
| `kestrel-span` | Source locations, `Span`, `Spanned<T>`, file IDs. |
| `kestrel-lexer` | Tokenization with logos. |
| `kestrel-parser` | Event-driven parser; emits events consumed by `kestrel-syntax-tree`. |
| `kestrel-syntax-tree` | Lossless CST (rowan). |
| `kestrel-ast` | Arena-allocated AST types (`AstType`, `AstBody`, `AstExpr`, `AstStmt`, `AstPat`). |
| `kestrel-ast-builder` | Lowers CST → hECS entities + components. Defines `NodeKind` and the component catalogue. |
| `kestrel-hecs` | The ECS itself: `Entity`, `World`, `QueryFn`, `QueryContext`, `Fingerprint`, `Revision`, snapshots. |
| `kestrel-name-res` | Scope and name resolution queries (`ResolveName`, `ResolveTypePath`, `ResolveValuePath`). |
| `kestrel-hir` | Body HIR (`HirExpr`, `HirPat`, `HirStmt`, `HirBody`). |
| `kestrel-hir-lower` | `LowerBody`, `LowerCallableTypes` queries — AST bodies → HIR bodies. |
| `kestrel-type-infer` | Constraint-based inference; `InferBody` query, `Constraint` / `InferError` enums, `TypeResolver`. |
| `kestrel-semantics` | Higher-level semantic queries: conformance resolution and polarity, protocol refinement, builtin-protocol identification, copy semantics. Used by infer/analyze. (Witness resolution lives in `kestrel-mir-lower`/codegen.) |
| `kestrel-analyze` | Analyzer framework + every concrete analyzer. |
| `kestrel-pattern-matching` | Exhaustiveness checking. |
| `kestrel-mir` | OSSA MIR types: `MirModule`, `FunctionDef`, `OssaBody`, `ValueId`, `Instruction`, `BasicBlock`, `Terminator`. SSA, not place-based (no `Place`/`Rvalue`/`Statement`). Also owns the MIR pass pipeline, monomorphization, type layout, and symbol mangling. |
| `kestrel-mir-lower` | Entities + typed bodies → MIR via plain functions (entry: `lower_module`), not a query; its only query is `IsProtocolMethod`. |
| `kestrel-codegen` | Tiny backend-agnostic crate: target configuration (`TargetConfig`). Layout and mangling now live in `kestrel-mir`. |
| `kestrel-codegen-cranelift` | Cranelift backend: lowers the already-monomorphized OSSA MIR → machine code and links. |
| `kestrel-compiler` | Low-level compiler / query engine. Owns the `World`. |
| `kestrel-compiler-driver` | High-level orchestration used by the CLI and tests. |
| `kestrel-debug` | Introspection utilities. |
| `kestrel-reporting` | Diagnostic formatting (codespan-reporting wrapper). |
| `kestrel-test-suite` | `.ks`-file test runner. Package name in `Cargo.toml` is `kestrel-test-suite`. |

## Data flow example — `5.toString()`

```
1. LEX          "5.toString()" → [Integer, Dot, Identifier, LParen, RParen]

2. PARSE/CST    MethodCallExpr
                 ├── Integer  "5"
                 ├── Dot      "."
                 ├── Identifier "toString"
                 └── Arguments ()

3. AST BUILD    The enclosing function entity gets a Valued component
                pointing at its CST body. No new entities per call.

4. HIR LOWER    HirExpr::MethodCall {
                    receiver: HirExpr::Literal(Int(5)),
                    name: "toString",
                    args: [],
                }
                The method NAME is a string — its target isn't known
                until we know the receiver's type.

5. TYPE INFER   Emits a Member constraint on the method call.
                Once the receiver's type resolves to Int64, the solver
                asks the semantics layer for Int64.toString, records
                the resolved callee entity on the expression, and
                unifies the result type with String.

6. ANALYZE      TypeCheckAnalyzer etc. run against the typed body.
                Any mismatches turn into diagnostics.

7. MIR LOWER    The method call becomes a concrete Call instruction
                whose callee is the resolved entity (generic if
                needed; monomorphized by a later MIR pass).
```

## Where the phase boundaries are (and aren't)

lib1 had explicit **BUILD → BIND → VALIDATE** phases that mutated a shared model in a specific order. lib does not.

- There is no "BIND pass." Name resolution is a query (`ResolveName`) fired as needed.
- There is no "VALIDATE pass." Each analyzer is a query that runs on demand and caches its result.
- The closest thing to a phase boundary is **component production**: `kestrel-ast-builder` is the only crate that writes components to the world. Everything downstream is read-only queries.

Practical consequence: to add a fact about a declaration, you either (a) add a component during AST build, or (b) write a query that computes the fact. You rarely need to thread it through a "pass."

## Further reading

- `lib/AGENTS.md` — documentation conventions for each lib crate; also lists which crates have `docs/` folders.
- `lib/kestrel-hecs/docs/architecture.md` — the ECS mechanics in detail.
- `lib/kestrel-ast-builder/docs/components.md` and `entity-mapping.md` — the component catalogue.
- `lib/kestrel-hir/docs/` — HIR shape and desugaring.
- `lib/kestrel-type-infer/docs/` and `lib/kestrel-type-infer/AGENTS.md` — inference internals.
- `lib/kestrel-analyze/AGENTS.md` — analyzer patterns.
- `docs/contributing/type-inference.md` — contributor overview of inference.
- `docs/contributing/quick-reference.md` — file paths by task.
- `docs/contributing/workflows.md` — step-by-step guides for common tasks.
