---
name: kestrel-pipeline
description: Pipeline-routing reference for the lib Kestrel compiler. Use when someone asks "where is X handled?", "where does <AstExpr/HirExpr/HirStmt/HirPat variant> get built?", "how does Type.method() lower?", "what constraint does try emit?", "trace this through the compiler", "what's the MIR shape of X?", or any question that would otherwise require opening 4-5 files across kestrel-ast-builder / kestrel-hir-lower / kestrel-type-infer / kestrel-mir-lower to answer.
---

# kestrel-pipeline

One-lookup map from a source construct to its file:line in every pipeline stage:
CST → AST → HIR → type-infer constraints → solver → MIR. Entries are **enum-complete**
across `AstExpr` / `AstStmt` / `AstPat` and `HirExpr` / `HirStmt` / `HirPat`.

## Which file to open

- `expressions.md` — every `AstExpr` (30) and `HirExpr` (23) variant. Binary ops,
  calls, member access, control flow, literals.
- `statements.md` — every `AstStmt` (4) and `HirStmt` (3) variant. Let bindings,
  expression statements, guard-let, deinit.
- `patterns.md` — every `AstPat` (12) and `HirPat` (12) variant. Plus the
  `HirLiteral` sub-reference used by pattern literals.
- `desugarings.md` — HIR-only constructs (`ProtocolCall`, `OverloadSet`, each
  non-`UserMatch` `MatchSource`, synthetic `If` / `Block` / `Tuple` / `Local` / etc.).
  Open this when the question is "how did we get HIR-expr X?" and the construct has
  no direct AST counterpart.

If you're unsure which file covers a construct, start with its enum name:
`AstExpr::*` / `HirExpr::*` → `expressions.md`, `*::Stmt` → `statements.md`,
`*::Pat` → `patterns.md`, or `MatchSource::*` / `ProtocolCall` / `OverloadSet` →
`desugarings.md`.

## Verify before recommending changes

**Pipeline maps go stale.** Before telling the user to edit at a cited `file:line`,
open the file and confirm the claim — function renames, match-arm reorders, and
inlined helpers move line numbers frequently. The enum variants themselves are
stable (checked via `ast_body.rs:42` / `body.rs:96`), but dispatch sites drift.

If a cited line no longer matches: grep for the variant name (e.g.,
`HirExpr::Match`) or the function name (e.g., `lower_match`, `desugar_try`) to
locate the current site, then update the skill file.

## Extending the skill

If a variant entry is missing or a new variant is added:

1. Read the enum definition (`ast_body.rs:42` / `body.rs:96`) to confirm the scope.
2. Trace the variant through AST-builder (`lower.rs`), HIR-lower
   (`expr.rs` / `stmt.rs` / `pat.rs` / `desugar.rs`), inference
   (`generate.rs` / `solver.rs`), and MIR-lower (`body_lower.rs`).
3. Add the entry with the same shape as existing ones — cite `file:line`, include a
   match-arm excerpt for branching variants, note gotchas. TODO markers are allowed
   when a variant can't be fully traced; never skip silently.

## Related references in MEMORY

These memory files add depth for the referenced topics — copy the load-bearing facts
inline into the variant entry when relevant rather than forcing a lookup:

- `match_pattern_analyzer.md` — `MatchSource` gating for exhaustiveness / unreachable
  arm checks.
- `dispatch_funnel_pattern.md` — method / witness dispatch funnel in MIR.
- `cascading_infer_errors.md` — how pattern / solver poison propagates (TupleRestPat,
  ImplicitPat, solve_equal / solve_coerce Error paths).
- `solver_poison_overreach.md` — SolveResult::Error vs report_and_poison distinction.
- `integer_literal_overflow_silent_zero.md` — `parse_int` u64-max round-trip.
- `static_overload_first_match_truncation.md` — static method overload resolution.
- `witness_instantiation_collapse.md` — `Convertible[T]` dedup bug history.

Plus many more codegen / runtime memory files relevant to the MIR side — see
`MEMORY.md` → "Codegen Bugs".

## Dispatch-switch quick anchors

Top of each pipeline stage — useful when you need the full match statement, not a
single variant:

- AST-builder expr dispatch: `lib/kestrel-ast-builder/src/lower.rs:307`
- AST-builder stmt dispatch: `lib/kestrel-ast-builder/src/lower.rs:160`
- AST-builder pat dispatch: `lib/kestrel-ast-builder/src/lower.rs:1321`
- HIR lowering expr dispatch: `lib/kestrel-hir-lower/src/expr.rs:19`
- HIR lowering stmt dispatch: `lib/kestrel-hir-lower/src/stmt.rs:15`
- HIR lowering pat dispatch: `lib/kestrel-hir-lower/src/pat.rs:34`
- Inference gen_expr: `lib/kestrel-type-infer/src/generate.rs:60`
- Inference gen_stmt: `lib/kestrel-type-infer/src/generate.rs:551`
- Inference gen_pat: `lib/kestrel-type-infer/src/generate.rs:607`
- Constraint dispatch: `lib/kestrel-type-infer/src/solver.rs:583` (`try_solve`)
- MIR lower_expr: `lib/kestrel-mir-lower/src/body_lower.rs:445`
- MIR lower_stmt: `lib/kestrel-mir-lower/src/body_lower.rs:419`

Solver per-constraint fns (all in `solver.rs`): `solve_equal` 817, `solve_coerce` 955,
`solve_conforms` 1076, `solve_associated` 1099, `solve_call` 1251,
`solve_overloaded_call` 1379, `solve_member` 1702, `solve_implicit` 2182,
`solve_implicit_pat` 2293, `solve_tuple_rest_pat` 2401, `solve_reduce` 716.
