# Statements — CST → AST → HIR → infer → MIR

Covers every variant of `AstStmt` (4) and `HirStmt` (3). Verify before citing —
pipeline maps go stale.

Top-level dispatch anchors:

- AST constructor switch: `lib2/kestrel-ast-builder/src/lower.rs:160` (`lower_stmt`)
- HIR lowering switch: `lib2/kestrel-hir-lower/src/stmt.rs:15` (`LowerCtx::lower_stmt`)
- Inference gen switch: `lib2/kestrel-type-infer/src/generate.rs:551` (`gen_stmt`)
- MIR lowering switch: `lib2/kestrel-mir-lower/src/body_lower.rs:419` (`lower_stmt`)

Top-of-body lowering: `AstBody::statements` → `HirBody::statements` through
`kestrel-hir-lower/src/lib.rs` which calls `lower_stmt` for each. Tail expressions are
lowered via `lower_expr` only.

---

## AstStmt variants (4)

Enum: `lib2/kestrel-ast/src/ast_body.rs:200`.

### AstStmt::Let

- Surface: `let x = v;`, `var x: Int = 0;`, `let (a, b) = pair;`,
  `let Point { x, y } = p;`.
- CST: `VariableDeclaration` (`lower.rs:162`).
- AST-builder: `lib2/kestrel-ast-builder/src/lower.rs:186` (`lower_variable_decl`,
  alloc at 232). Detects `var` for `is_mut`, pulls pattern / type / optional value.
- HIR lowering: `lib2/kestrel-hir-lower/src/stmt.rs:18-24` dispatches to
  `lower_let_stmt` (`stmt.rs:64`). Two paths:
  - **Simple binding pattern** (`stmt.rs:78-86`): allocates a `LocalId` via
    `define_local`, emits `HirStmt::Let { local, ty, value, span }` directly.
  - **Complex pattern** (`stmt.rs:87-138`): desugars to
    `{ let $let_tmp = value; match $let_tmp { pattern => () } }`. The outer return is
    a `HirStmt::Expr` wrapping a `HirExpr::Block`, the inner `HirStmt::Let` names the
    `$let_tmp` local, and the `HirExpr::Match` has `source: MatchSource::LetDestructure`.
    `lower_pat_forcing_mut(body, pattern, is_mut)` propagates outer `var` into all sub-bindings.
- Type-infer: `generate.rs:553-587`. Annotated → `lower_hir_ty(ty)` for local TyVar;
  unannotated → fresh. `ctx.local_types.insert(local, local_tv)`. Bidirectional hints:
  if annotation is `Array[E]` and RHS is `HirExpr::Array`, seed
  `ctx.expected_array_elem`; same for `Dict`. Then `ctx.coerce(val_tv, local_tv, ...)`.
- Solver: `solve_coerce` (955).
- MIR: `body_lower.rs:419-430` — emits `StatementKind::Assign { dest: Place::local(mir_local),
  rvalue: value_to_rvalue(init_value) }`. No init value → no statement emitted (the
  local slot is zero-initialized by default).
- Gotchas:
  - `let _ = expr;` is NOT a separate variant — `_` is `AstPat::Wildcard`, which is
    NOT a simple binding, so it routes through the complex-pattern path and becomes a
    match with a wildcard arm.
  - Complex-pattern desugaring uses `$let_tmp` as the local name — prefixed with `$`
    to avoid collisions with user identifiers. Same convention as `$iter`, `$try_value`,
    `$try_early`, `$unwrap` (see `desugar.rs`).
  - `var (a, b) = pair` propagates `is_mut` into both `a` and `b` via
    `lower_pat_forcing_mut`. Plain `let` does not.

### AstStmt::Expr

- Surface: `foo();`, any expression followed by `;` (or a block-form expr as an
  expression statement).
- CST: `ExpressionStatement` (`lower.rs:163`). Also the fallback for unknown expr-like
  statements (`lower.rs:168-180`).
- AST-builder: `lower.rs:252` (`lower_expr_stmt`, alloc at 261). Also synthesized for
  malformed statements (`lower.rs:179`) and promoted tail expressions (see
  `lower_expr_stmt_as_expr` at 244).
- HIR lowering: `stmt.rs:26-32` — 1:1.
  ```
  AstStmt::Expr { expr, span } => {
      let lowered = self.lower_expr(body, *expr);
      self.alloc_stmt(HirStmt::Expr { expr: lowered, span: span.clone() })
  }
  ```
- Type-infer: `generate.rs:589-591` — `gen_expr(ctx, hir, expr)`; result discarded.
- MIR: `body_lower.rs:432-435` — `let _ = self.lower_expr(*expr);` (lowered for
  side effects).
- Gotchas:
  - Block expressions at the end of a code block can be promoted to the block's tail
    expression by `lower_block` (`lower.rs:115-141`) — the expr appears in
    `tail_expr`, not as a `Stmt::Expr`. See MEMORY `match_pattern_analyzer.md` for
    the closure-vs-block ambiguity background.
  - If a guard-let / while / let-destructure desugaring produces a statement rather
    than an expression, it's wrapped in `HirStmt::Expr` internally — see
    `stmt.rs:120-123, 174-177`, `desugar.rs:234-237, 303-306, 452-455`.

### AstStmt::GuardLet

- Surface: `guard let .Some(x) = opt else { return }`,
  `guard let x = opt, y > 0 else { throw err }`.
- CST: `GuardLetStatement` (`lower.rs:164`).
- AST-builder: `lower.rs:265` (`lower_guard_let`, alloc at 280). Stores
  `Vec<IfCondition>` (mixed let/expr) plus the else block.
- HIR lowering: `stmt.rs:34-38` → `lower_guard_let` (`stmt.rs:143`). Emits
  `HirExpr::If { condition: lowered_conditions, then_body: {}, else_body: Some(else_block) }`
  wrapped in `HirStmt::Expr`. The statement id is pushed into
  `ctx.guard_let_stmts` (`stmt.rs:180`) so the **guard-let-divergence analyzer** can
  enforce that the else block diverges. `lower_if_conditions` is called with
  `source: MatchSource::GuardLet` so any desugared let-condition match gets the right
  `MatchSource` tag.
- Type-infer: `generate.rs:589-591` (routes through `HirStmt::Expr`). Crucially
  `generate.rs:376-380` in the `HirExpr::If` arm skips the else-equate for guard-let
  Ifs (via `is_guard_let_if`), because the else block must diverge.
- MIR: `body_lower.rs:432-435` (through `HirStmt::Expr` → `lower_expr` → `lower_if`).
- Gotchas:
  - The **bindings** from let-conditions live in the **outer** scope (not a nested
    scope) — see `stmt.rs:157-162`. That's how `guard let x = opt else { return }`
    lets you use `x` after the guard.
  - The else block is required to diverge (return/break/continue/throw). The
    `guard_let_divergence` analyzer is responsible — HIR lowering does not enforce this.
  - Differs from `AstExpr::If` in that GuardLet is always a **statement**, not an
    expression, so it never has a tail value.

### AstStmt::Deinit

- Surface: `deinit handle;`.
- CST: `DeinitStatement` (`lower.rs:165`).
- AST-builder: `lower.rs:288` (`lower_deinit_stmt`, alloc at 298).
- HIR lowering: `stmt.rs:40-57` — looks up `name` via `lookup_local`, emits a
  diagnostic ("undeclared variable") if missing, then allocates
  `HirStmt::Deinit { name, local: Option<LocalId>, span }`. The `local` may be `None`
  if the lookup failed (error already reported).
- Type-infer: `generate.rs:593-595` — no constraints. Purely a cleanup registration.
- MIR: `body_lower.rs:436-438` — **skipped**. Deinit resolution is handled by a later
  pass (not yet fully wired in lib2).
- Gotchas:
  - No runtime code is currently emitted for deinit — if you're debugging a dropped
    value and expect a destructor call, check that the pass that consumes
    `HirStmt::Deinit` is actually running.
  - Not a method call — `deinit x` is a statement keyword, not `x.deinit()`.

---

## HirStmt variants (3)

Enum: `lib2/kestrel-hir/src/body.rs:234`.

### HirStmt::Let

- Produced by: `AstStmt::Let` with a simple `AstPat::Binding` (`stmt.rs:78-86`). Also
  synthesized for complex-pattern let desugaring's `$let_tmp` binding
  (`stmt.rs:91-95`) and for the for-loop `$iter` temp (`desugar.rs:370-374`).
- Type-infer: `generate.rs:553-587` (see `AstStmt::Let`). Both the annotated and
  unannotated paths live here; bidirectional hints are handled before generating the
  value expression.
- MIR: `body_lower.rs:419-430` — `Assign` into `Place::local(map_local(local))`.
- Gotchas: do not assume `HirStmt::Let` has the same scope semantics as the original
  AST — when the source had a complex pattern, there's a synthetic `$let_tmp` local
  followed by a Match that binds the real names.

### HirStmt::Expr

- Produced by: `AstStmt::Expr` (`stmt.rs:26-32`), `AstStmt::GuardLet` (`stmt.rs:174-177`,
  wrapping a synthesized `HirExpr::If`), complex-pattern `let` (`stmt.rs:120-136`,
  wrapping the `HirExpr::Block(Let + Match)` and also inner wrapping of the Match
  itself at 120-123), `desugar_while` intermediate if-break (`desugar.rs:234-237`),
  `desugar_while_let` intermediate if-break (`desugar.rs:303-306`), and `desugar_for_loop`
  iterator let (`desugar.rs:370-374` emits `HirStmt::Let`, not `::Expr`, but the match
  body ends up inside a `HirStmt::Expr` at 452-455).
- Type-infer: `generate.rs:589-591` — `gen_expr`, discard result.
- MIR: `body_lower.rs:432-435` — `lower_expr(expr)`, result discarded.

### HirStmt::Deinit

- Produced by: `AstStmt::Deinit` only (`stmt.rs:40-57`). The `local: Option<LocalId>`
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
  `lib2/kestrel-hir/src/body.rs:42-45`. Analyzers use them to find specific
  source-original constructs after desugaring.
