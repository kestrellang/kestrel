---
name: hecs
description: 'The compiler''s hECS architecture (entities, components, memoized queries) and why it is built that way. Use for "should this be a component or a query?", "where should this fact live?", or when designing a new query or analyzer.'
---

# hECS

The compiler's semantic database is **hECS**: a hierarchical entity component
system with Salsa-style memoized queries, in `lib/kestrel-hecs`. There are no
build, bind or validate phases. The query graph is demand-driven, and when a
file changes only the queries that depended on it re-run.

## The model

| Piece | What it is | Where |
|---|---|---|
| **Entity** | A declaration (module, struct, enum, case, protocol, extension, function, init, field, subscript, type alias, import, type parameter) or a file. A cheap `u32` handle | Spawned by `kestrel-ast-builder` |
| **Component** | A fact about one entity that comes from that declaration's own syntax: `NodeKind`, `Name`, `DeclSpan`, `CstNode`, `Callable`, `TypeAnnotation`, generics, attributes, … No cross-entity lookups | Set only by `kestrel-ast-builder`; catalog in `lib/kestrel-ast-builder/docs/components.md` |
| **Hierarchy** | Parent/child links: module → struct → field | `World::set_parent` |
| **Query** | A memoized function of the world (`impl QueryFn`). Everything that needs more than one entity: resolution, conformance, member lookup, inference, analysis | Spread across the crates (name-res, semantics, type-infer, …) |
| **Accumulator** | Side-effect output of a query, mainly diagnostics | `ctx.accumulate(value)` |

The crate's own doc, `lib/kestrel-hecs/docs/architecture.md`, covers storage,
the lifecycle, memoization, backdating and cycle recovery in detail.

## Component or query?

- **A component** if it can be computed from the declaration's syntax alone,
  for one file, without looking at another entity. It's set once, when the
  builder walks the file.
- **A query** for anything else: a name lookup, a resolved type, a
  conformance, "every method on this type", a lang item. Queries may call other
  queries; the dependency graph is recorded for you.
- **Never mutate the world downstream of the builder.** Everything after
  `kestrel-ast-builder` reads components and runs queries.
- **Per-nominal MIR facts** are a different layer: they live on MIR's
  `TypeInfo`, set by the pass that decides them (see the `mir` skill).

## Writing a query

```rust
/// What it answers, and why it's a query.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ResolveBuiltin {
    pub builtin: Builtin,
    pub root: Entity,
}

impl QueryFn for ResolveBuiltin {
    type Output = Option<Entity>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Self::Output {
        // Read components with ctx.get::<T>(entity) and call other queries
        // with ctx.query(Other { .. }); both record dependencies.
        …
    }
}
```

- The key (the struct's fields) is the memo key, so keep it small and
  `Hash + Eq`. Key per-type facts by `(Entity, type_args)`, never by the
  nominal alone.
- The output must be `Clone + Hash` so it can be fingerprinted. If a
  re-execution gives the same fingerprint, dependents don't re-run
  (backdating).
- A query that can re-enter itself (recursive types) picks a `CycleMode`:
  `Panic` (the default; a cycle is a bug), fixpoint iteration, or a fallback
  answer.
- Report diagnostics with `ctx.accumulate`, not by returning them, so they are
  attributed to the query and replaced when it re-runs.
- `rg 'impl QueryFn for' lib` lists every query (about 90 today).

## Rules that keep it incremental

- **Single source of truth.** One query answers each question. Before adding
  one, grep for an existing query that does it (`rg 'impl QueryFn for' lib`).
- **No name lookups for compiler-known things.** Find lang items with
  `ResolveBuiltin` (`@builtin(.X)`), never by spelling. See the root
  `CLAUDE.md`.
- **Don't split on struct vs enum when a component says it.** `HirTy` and the
  solver's `TyKind` still have separate `Struct` and `Enum` variants, but the
  shared rules (copy semantics, `Static`, conformance checking) work on one
  nominal shape (`CopyShape::Nominal`, `StaticShape::Nominal`). Before adding a
  struct-only or enum-only arm, look for the component or shared helper that
  already expresses the difference.
- **Systems work on components, not entity kinds.** An analyzer asks "does it
  have `Callable`?" rather than "is it a function?", so new declaration kinds
  get the behavior for free.
- **Components hold no syntax nodes.** Use `CstNode` (a `Send`, hashable
  pointer) and resolve it against `FileSyntax`; a `SyntaxNode` would pin the
  tree and isn't `Send`.
- **Tests clone the std world.** `World::snapshot()` copies a built world so
  each test doesn't rebuild the std.

## History

`history/` holds the migration plans that produced this design: the move from
the old symbol-tree compiler, the struct/enum consolidation
(`deduplication.md`), moving all resolution into inference, the parser and
mangling rewrites (`big-steps.md`), and the original query catalog
(`queries.md`, `GOALS.md`). They explain *why* things are shaped the way they
are. Their file paths, line numbers and query counts describe the code at the
time and are not current.

For where a construct is handled stage by stage, use `kestrel-pipeline`. For
MIR, use `mir`.
