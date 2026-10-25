# Statements — CST → HIR → infer → MIR

Covers every statement kind of the CST (`StatementKind`: 4) and `HirStmt` (3). Verify
before citing — pipeline maps go stale.

Top-level dispatch anchors:

- HIR lowering switch: `lib/kestrel-hir-lower/src/stmt.rs` (`LowerCtx::lower_stmt` →
  `lower_stmt_node`, matching on the `SyntaxKind`)
- Inference gen switch: `lib/kestrel-type-infer/src/generate.rs:551` (`gen_stmt`)
- MIR lowering switch: `lib/kestrel-mir-lower/src/body_lower.rs:419` (`lower_stmt`)

Top-of-body lowering: `LowerBodyWithSourceMap` (`kestrel-hir-lower/src/lib.rs`) reads the
entity's `Valued` body node — a `CodeBlock`, a function's `= expr` (`FunctionBody`), a
parameter default's `= expr` (`DefaultValue`) or a field initializer `Expression` —
into a `BlockSyntax` (`syntax::block_syntax`: a `Statement` child is a statement, a bare
final `Expression` or a final statement-like `ExpressionStatement` without `;` is the
value) and calls `lower_block_stmts` (guard-let CPS applies at the top level).

---

## Statement syntax → HIR (every `StatementKind`)

### `VariableDeclaration` (`let` / `var`)

- Surface: `let x = v;`, `var x: Int = 0;`, `let (a, b) = pair;`,
  `let Point { x, y } = p;`, `let r = &place;`.
- CST: `('let' | 'var') Pattern (':' Ty)? ('=' Expression)? ';'` (`syntax::let_syntax`).
- HIR lowering: `stmt.rs` `lower_let_stmt`. Two paths, chosen on the pattern after
  grouping parentheses are removed (`PatSrc::resolve`):
  - **Simple binding** (a `BindingPattern` / `RefBindingPattern` with a name):
    `define_named_local` (records the identifier in the source map), emits
    `HirStmt::Let { local, ty, value, span }` — `Local::span` is the whole statement.
  - **Anything else** (including a binding whose name the parser could not find):
    `{ let $let_tmp = value; match $let_tmp { pattern => () } }` — a `HirStmt::Expr`
    wrapping a `HirExpr::Block`; the `Match` has `source: MatchSource::LetDestructure`.
    `lower_pat_forcing_mut(pattern, is_mut)` propagates an outer `var` into every binding.
  - A `&expr` / `&mutating expr` initializer on a simple `let` → `HirExpr::Borrow`;
    on a `var` or a destructuring pattern → E209 (lowered without the borrow).
- Type-infer: `generate.rs:553-587`. Annotated → `lower_hir_ty(ty)` for local TyVar;
  unannotated → fresh. `ctx.local_types.insert(local, local_tv)`. Bidirectional hints:
  if annotation is `Array[E]` and RHS is `HirExpr::Array`, seed
  `ctx.expected_array_elem`; same for `Dict`. Then `ctx.coerce(val_tv, local_tv, ...)`.
- Solver: `solve_coerce` (955).
- MIR: `body_lower.rs:419-430` — emits `StatementKind::Assign { dest: Place::local(mir_local),
  rvalue: value_to_rvalue(init_value) }`. No init value → no statement emitted (the
  local slot is zero-initialized by default).
- Gotchas:
  - `let _ = expr;` is not a simple binding — `_` routes through the destructuring
    path and becomes a match with a wildcard arm.
  - Desugaring temporaries are `$`-prefixed (`$let_tmp`, `$iter`, `$try_value`,
    `$try_early`, `$dsi`, `$opts`) so they cannot collide with user identifiers.

### `ExpressionStatement`

- Surface: `foo();`, any expression followed by `;`, a statement-like expression
  (`if`, `while`, `match`, …) standing alone mid-block.
- CST: `Expression ';'?`. In a closure, a statement-like expression stands *without*
  the wrapper; `syntax::closure_body_syntax` demotes it to a statement
  (`StmtSyntax::Expr`, synthetic span) when something follows it.
- HIR lowering: `stmt.rs` `lower_stmt_node` → `HirStmt::Expr { expr, span }` (1:1).
- Type-infer: `generate.rs:589-591` — `gen_expr(ctx, hir, expr)`; result discarded.
- MIR: `body_lower.rs:432-435` — `let _ = self.lower_expr(*expr);` (lowered for
  side effects).
- Gotchas:
  - A final statement-like expression without `;` is the block's *value*
    (`block_syntax`), not a statement.
  - Desugarings that produce a statement wrap it in `HirStmt::Expr` internally.

### `GuardStatement`

- Surface: `guard let .Some(x) = opt else { return }`,
  `guard let x = opt, y > 0 else { throw err }`, `guard cond else { … }`.
- CST: `guard Condition (, Condition)* else CodeBlock`; conditions are `GuardCondition`
  (`let p = v`) or `Expression` (`LowerCtx::guard_parts`).
- HIR lowering: two shapes.
  - Any `let` condition → **CPS** in `expr.rs` `lower_block_stmts` / `lower_guard_cps`:
    the remaining statements + tail become the success continuation of a
    `lower_condition_chain` with `MatchSource::GuardLet` (bindings dominate the rest of
    the block under OSSA); the else block is lowered once as the shared fail arm.
  - No `let` → `stmt.rs` `lower_guard`: `HirExpr::If { condition, then: {}, else: Some(else) }`
    in a `HirStmt::Expr`, conditions through `lower_if_conditions(MatchSource::Guard)`;
    the statement id is pushed to `guard_stmts` for the divergence analyzer.
- Type-infer: `generate.rs:589-591` (routes through `HirStmt::Expr`). The `HirExpr::If`
  arm skips the else-equate for guard Ifs, because the else block must diverge.
- MIR: `body_lower.rs:432-435` (through `HirStmt::Expr` → `lower_expr`).
- Gotchas:
  - The else block is required to diverge (return/break/continue/throw); the guard
    divergence analyzer enforces it — HIR lowering does not.
  - Always a statement, never a block value.

### `DeinitStatement`

- Surface: `deinit handle;`.
- CST: `deinit ident ;`.
- HIR lowering: `stmt.rs` `lower_deinit_stmt` — `lookup_local`, E137 ("undeclared
  variable") when missing, then `HirStmt::Deinit { name, local: Option<LocalId>, span }`.
  A missing name (reported by the parser) lowers to `HirName::Missing` with no E137.
- Type-infer: `generate.rs:593-595` — no constraints. Purely a cleanup registration.
- MIR: `body_lower.rs:436-438` — **skipped**. Deinit resolution is handled by a later
  pass (not yet fully wired in lib).
- Gotchas:
  - Not a method call — `deinit x` is a statement keyword, not `x.deinit()`.

---

## HirStmt variants (3)

Enum: `lib/kestrel-hir/src/body.rs:234`.

### HirStmt::Let

- Produced by: `VariableDeclaration` with a simple `BindingPattern` (`stmt.rs`). Also
  synthesized for complex-pattern let desugaring's `$let_tmp` binding
  (`stmt.rs`) and for the for-loop `$iter` temp (`desugar.rs`).
- Type-infer: `generate.rs:553-587` (see `VariableDeclaration`). Both the annotated and
  unannotated paths live here; bidirectional hints are handled before generating the
  value expression.
- MIR: `body_lower.rs:419-430` — `Assign` into `Place::local(map_local(local))`.
- Gotchas: do not assume `HirStmt::Let` has the same scope semantics as the original
  AST — when the source had a complex pattern, there's a synthetic `$let_tmp` local
  followed by a Match that binds the real names.

### HirStmt::Expr

- Produced by: `ExpressionStatement` (`stmt.rs`), `GuardStatement` (`stmt.rs`,
  wrapping a synthesized `HirExpr::If`), complex-pattern `let` (`stmt.rs`,
  wrapping the `HirExpr::Block(Let + Match)` and also inner wrapping of the Match
  itself at 120-123), `desugar_while` intermediate if-break (`desugar.rs`),
  `desugar_while_let` intermediate if-break (`desugar.rs`), and `desugar_for_loop`
  iterator let (`desugar.rs` emits `HirStmt::Let`, not `::Expr`, but the match
  body ends up inside a `HirStmt::Expr` at 452-455).
- Type-infer: `generate.rs:589-591` — `gen_expr`, discard result.
- MIR: `body_lower.rs:432-435` — `lower_expr(expr)`, result discarded.

### HirStmt::Deinit

- Produced by: `DeinitStatement` only (`stmt.rs`). The `local: Option<LocalId>`
  is resolved at HIR-lowering time; `None` means lookup failed and a diagnostic was
  already emitted.
- Type-infer: `generate.rs:593-595` — no constraints.
- MIR: `body_lower.rs:436-438` — skipped (handled by a later pass when wired).
- Gotchas: the HIR stores the unresolved `name` string alongside the resolved
  `local: Option<LocalId>` — both fields exist so a later pass can either act on the
  local or re-emit a better diagnostic on the original name.

---

## Cross-references

- `HirExpr` details referenced here (If, Match, Block) — see `expressions.md`.
- `HirPat` produced by let-destructuring patterns — see `patterns.md`.
- `MatchSource` tagging for synthetic matches — see `desugarings.md`.
- `guard_let_stmts` / `while_conditions` fields on `HirBody` — see
  `lib/kestrel-hir/src/body.rs:42-45`. Analyzers use them to find specific
  source-original constructs after desugaring.
