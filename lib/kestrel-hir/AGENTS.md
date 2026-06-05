# kestrel-hir — Agent Guide

Patterns and invariants for the HIR representation. Read this before adding
or modifying `HirExpr`, `HirStmt`, `HirPat`, or fields on `HirBody`.

## Structure on the node, not in a side-table

When a downstream pass needs to distinguish "this node came from X vs Y"
(e.g. desugared vs user-written, different desugaring source), put the tag
**on the node itself** as a field, not in a parallel `Vec<HirExprId>` on
`HirBody`.

Exemplar: `MatchSource` on `HirExpr::Match`. Any match on a `HirExpr::Match`
is forced by the compiler to handle the source field — analyzers can't
forget to check a side-table because there is no side-table.

When a side-table is tempting, ask first:
1. Can the information live on the node it describes? If yes, prefer that.
2. Can an `is_*()` predicate on an existing field answer the question?
3. Does every consumer need the same tag, or is it analyzer-local? If
   analyzer-local, keep it in the analyzer, not on `HirBody`.

Side-tables are acceptable only when the information spans multiple nodes
or lives on a different node shape than what it describes (e.g.
`guard_let_stmts` marks `HirStmtId`s for divergence checks; the desugared
`if/else` shape doesn't fit a source tag as cleanly).

## Desugared construct invariants

Desugared matches share one invariant: **the analyzer must know it is
desugared** so it can skip warnings that only make sense for user code.
The mechanism is `MatchSource::is_desugared()`. Any new desugared `Match`
site must pick the right variant:

| Desugars | MatchSource | Emits |
|----------|-------------|-------|
| `if let p = v` | `IfLet` | E302 if irrefutable |
| `while let p = v` | `WhileLet` | E308 if irrefutable |
| `guard let p = v` | `GuardLet` | E309 if irrefutable |
| `for p in iter` | `ForLoop` | nothing (for_loop_pattern handles refutability) |
| `let <complex> = v` | `LetDestructure` | nothing |
| `try e` | `TryOp` | nothing |
| `match v { … }` | `UserMatch` | full exhaustiveness suite |

(`e!` force-unwrap does **not** produce a `Match` — it lowers to a
`ForceUnwrap.forceUnwrap()` `ProtocolCall`; see `kestrel-hir-lower/AGENTS.md`.)

Adding a new desugaring that produces a `Match`? Add a variant to
`MatchSource` rather than reusing an existing one — the source is also
consumed by future tooling that may need to distinguish them.

## Incremental-compilation awareness

`HirBody` is one of the main cache keys in the incremental layer. Before
adding a field:

1. Is the field derivable from existing state? If yes, make it a method, not a field.
2. Does it change more frequently than the node it describes? If yes, put it in a separate cache.
3. Does every edit to the function body invalidate the field? If no, the field shouldn't live here.

Enum fields on existing variants are cheap. New side-tables on `HirBody`
are not — they expand the hash surface.
