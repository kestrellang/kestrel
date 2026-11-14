---
name: kestrel-pipeline
description: Where each source construct is handled in the compiler, stage by stage (HIR lowering, type inference, MIR). Use for "where is X handled?", "how does X lower?" or "trace this through the compiler".
---

# kestrel-pipeline

One-lookup map from a source construct to its file:line in every pipeline stage:
CST → HIR → type-infer constraints → solver → MIR. Bodies have no AST: HIR lowering
reads the CST's typed views directly. Entries are **enum-complete** across the CST's
`Expr` / `StatementKind` / `Pat` kinds and `HirExpr` / `HirStmt` / `HirPat`.

## Which file to open

- `expressions.md` — every `Expr` syntax kind and `HirExpr` variant. Binary ops,
  calls, member access, control flow, literals.
- `statements.md` — every `StatementKind` (4) and `HirStmt` (3) variant. Let bindings,
  expression statements, guard-let, deinit.
- `patterns.md` — every `Pat` syntax kind and `HirPat` variant. Plus the
  `HirLiteral` sub-reference used by pattern literals.
- `desugarings.md` — HIR-only constructs (`ProtocolCall`, `OverloadSet`, each
  non-`UserMatch` `MatchSource`, synthetic `If` / `Block` / `Tuple` / `Local` / etc.).
  Open this when the question is "how did we get HIR-expr X?" and the construct has
  no direct syntax counterpart.

If you're unsure which file covers a construct, start with its enum name:
`Expr*` / `HirExpr::*` → `expressions.md`, statements → `statements.md`,
`*::Pat` → `patterns.md`, or `MatchSource::*` / `ProtocolCall` / `OverloadSet` →
`desugarings.md`.

## Verify before recommending changes

**Pipeline maps go stale.** Entries cite a file and a function, never a line
number, because line numbers drift with every edit. Before telling the user to
edit somewhere, open the file and confirm the claim; functions get renamed and
inlined too. The enum variants themselves are
stable (`kestrel.ungram` in `kestrel-syntax-tree` / `kestrel-hir/src/body.rs`), but
dispatch sites drift.

If a cited function no longer exists: grep for the variant name (e.g.,
`HirExpr::Match`) or the function name (e.g., `lower_match`, `desugar_try`) to
locate the current site, then update the skill file.

## Extending the skill

If a variant entry is missing or a new variant is added:

1. Read the grammar rule (`lib/kestrel-syntax-tree/kestrel.ungram`) / the HIR enum
   (`lib/kestrel-hir/src/body.rs`) to confirm the scope.
2. Trace the variant through HIR-lower (`syntax.rs` for multi-child syntax decisions,
   `expr.rs` / `stmt.rs` / `pat.rs` / `desugar.rs`), inference
   (`generate.rs` / `solver/`), and MIR-lower (`lib/kestrel-mir-lower/src/body/`).
3. Add the entry with the same shape as existing ones — cite the file and function (no line numbers), include a
   match-arm excerpt for branching variants, note gotchas. TODO markers are allowed
   when a variant can't be fully traced; never skip silently.

## Dispatch-switch quick anchors

Top of each pipeline stage — useful when you need the full match statement, not a
single variant:

- Body entry (body syntax → `BlockSyntax`, params): `lib/kestrel-hir-lower/src/lib.rs`
  (`LowerBodyWithSourceMap`, `body_syntax`, `define_params`)
- Syntax helpers (blocks, conditions, paths, operators, implicit `it`):
  `lib/kestrel-hir-lower/src/syntax.rs`
- HIR lowering expr dispatch: `lib/kestrel-hir-lower/src/expr.rs` (`lower_expr_node`)
- HIR lowering stmt dispatch: `lib/kestrel-hir-lower/src/stmt.rs` (`lower_stmt_node`)
- HIR lowering pat dispatch: `lib/kestrel-hir-lower/src/pat.rs` (`lower_pat_node`)
- Inference: `lib/kestrel-type-infer/src/generate.rs` (`gen_expr`, `gen_stmt`, `gen_pat`)
- Constraint dispatch: `lib/kestrel-type-infer/src/solver/` (`try_solve`)
- MIR lower_expr: `lib/kestrel-mir-lower/src/body/expr.rs` (`lower_expr_no_promote` holds the per-variant match)
- MIR lower_stmt: `lib/kestrel-mir-lower/src/body/stmt.rs`
- MIR patterns / match: `lib/kestrel-mir-lower/src/body/pattern.rs` (`lower_match`)

MIR is OSSA: SSA values (`ValueId`) and blocks with params, no places or
rvalues. `kestrel dump mir -s all f.ks` prints every pass stage.

Solver per-constraint fns (all in `solver/`): `solve_equal`, `solve_coerce`,
`solve_conforms`, `solve_associated`, `solve_call`,
`solve_overloaded_call`, `solve_member`, `solve_implicit`,
`solve_implicit_pat`, `solve_tuple_rest_pat`, `solve_reduce`.
