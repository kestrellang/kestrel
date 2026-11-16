# Desugarings — HIR-only constructs

HIR variants and shapes that don't have a 1:1 syntax counterpart: constructs synthesized
during HIR lowering to reduce the surface language to a smaller core. "How did we get
this `HirExpr::X`?" is usually answered here.

All citations refer to `lib/kestrel-hir-lower/src/` unless otherwise noted.

---

## HirExpr::ProtocolCall

ProtocolCall is **desugar-only** — no syntax spells it directly. Every
ProtocolCall in the HIR was synthesized from one of the sites below.

Signature (`body.rs`):

```rust
ProtocolCall {
    receiver: HirExprId,
    protocol: Entity,
    method: String,
    type_args: Option<Vec<HirTy>>,
    args: Vec<HirCallArg>,
    span: Span,
}
```

Type-infer: `generate.rs` emits `Constraint::Conforms { ty: recv, protocol }`
AND `Constraint::Member { receiver, name: method, args, ... }`. Solver funnels are
`solve_conforms` and `solve_member`.

MIR dispatch: `body/call/mod.rs` `lower_protocol_call_expr`.
Emits witness-based method dispatch.

### Source: binary operators

- Trigger: `ExprBinary` (precedence already applied by the parser).
- Site: `desugar.rs` (`desugar_binary_hir`) — invoked from `lower_expr_node` in
  `expr.rs` for each binary node.
- Shape:
  ```
  ProtocolCall {
      receiver: lhs,
      protocol: <op protocol>,
      method: "<op method>",
      args: [HirCallArg { label, value: rhs }],
  }
  ```
- Protocol + method table: see `kestrel-hir::body::lookup_binary_op` and
  `desugar_binary_hir` at `desugar.rs`.

### Source: short-circuit operators (`&&`, `||`, `??`)

- Trigger: `ExprBinary` with a short-circuit op.
- Site: `desugar.rs` (inside `desugar_binary_hir`). RHS is wrapped in a
  parameterless `HirExpr::Closure` so the RHS protocol method can lazy-evaluate:
  ```
  ProtocolCall {
      receiver: lhs,
      protocol: <short-circuit protocol>,
      method: ...,
      args: [HirCallArg { value: Closure { params: [], body: { tail_expr: rhs } } }],
  }
  ```
- Gotcha: HIR records no captures. Capture plans (disjoint, place-based) are
  computed after inference in `kestrel-type-infer/src/captures.rs`, the single
  source of truth for MIR closure lowering and the E603 analyzer.

### Source: multi-condition if/while/guard

- Trigger: `if a, b, c { ... }` (comma-separated conditions combining into `a && b && c`).
- Site: `lower_if_conditions` (`expr.rs`) folds the list through
  `desugar_binary_hir(BinaryOp::And, …)`, the same path as a written `&&`.
- Shape: same as short-circuit `&&` above.

### Source: unary operators

- Trigger: `ExprUnary` (except `UnaryOp::Pos` which is identity).
- Site: `desugar.rs` (`desugar_unary_op`). Emits at `desugar.rs`:
  ```
  ProtocolCall {
      receiver: operand,
      protocol: <unary protocol>,
      method: "<method>",
      args: vec![],
  }
  ```

### Source: compound assignment

- Trigger: `ExprCompoundAssignment`.
- Site: `desugar.rs` (`desugar_compound_assign`). Wraps the resulting
  `ProtocolCall` (or `HirExpr::Error` if the syntactic place check rejected the LHS)
  in `HirExpr::Sugar { kind: CompoundAssign, inner, span }`:
  ```
  Sugar {
      kind: CompoundAssign,
      inner: ProtocolCall {
          receiver: lhs,
          protocol: <compound-assign protocol>,
          method: ...,
          args: [HirCallArg { label, value: rhs }],
      },
  }
  ```
- Syntactic place check: `is_place_syntax(lhs)` runs before lowering and
  rejects literals/calls/blocks/etc. with "left-hand side of compound
  assignment is not assignable", returning `Sugar { CompoundAssign, inner: Error }`.
- Gotcha: this is **not** `HirExpr::Assign(lhs, ProtocolCall(lhs, add, rhs))`. The
  compound-assign method mutates the receiver in place. Analyzers that scan for
  `HirExpr::Assign` targets won't see `+=` — they need to check Sugar of kind
  `CompoundAssign` (or recurse through Sugar transparently).

### Source: while-let negation

- Trigger: `ExprWhile (let)` (the condition is negated to compute the break
  condition).
- Site: `desugar.rs`:
  ```
  ProtocolCall {
      receiver: cond,
      protocol: LogicalNotOperatorProtocol,
      method: "logicalNot",
      args: vec![],
  }
  ```

### Source: for-loop `iter()` / `next()`

- Trigger: `ExprFor`.
- Sites:
  - `desugar.rs` (`desugar_for_loop`) — `iterable.iter()` via `IterableProtocol`.
  - Same function — `$iter.next()` via `IteratorProtocol`.
- The whole desugaring (outer `Block { let $iter; loop { match $iter.next() } }`)
  is wrapped in `HirExpr::Sugar { kind: ForLoop, inner: Block, span }`.
- If `IterableProtocol` isn't resolvable (e.g. `stdlib: false` tests), the
  desugar emits "`for` loop requires the `Iterable` protocol" directly and
  returns `Sugar { ForLoop, inner: HirExpr::Error }`. The `next()` fallback
  to a plain `MethodCall` only matters when Iterable resolved but Iterator
  didn't (inconsistent stdlib state).

### Source: try-expr `tryExtract()`

- Trigger: `ExprTry`.
- Site: `desugar.rs` (`desugar_try`) — `operand.tryExtract()` via `TryableProtocol`.
  The whole desugaring (the `Match { source: TryOp, ... }`) is wrapped in
  `HirExpr::Sugar { kind: Try, inner: Match, span }`.
- If `TryableProtocol` isn't resolvable, the desugar emits "`try` expression
  requires the `Tryable` protocol" directly and returns
  `Sugar { Try, inner: HirExpr::Error }` — no `.Ok`/`.Err` fallback (it would
  cascade into "implicit member not found" garbage).

### Source: interpolated-string concatenation

- Trigger: `ExprInterpolatedString`.
- Site: `desugar.rs`:
  ```
  ProtocolCall {
      receiver: result_so_far,
      protocol: AddOperatorProtocol,
      method: "add",
      args: [HirCallArg { value: next_part }],
  }
  ```
- Each `StringPart::Interpolation` becomes a `HirExpr::MethodCall { method: "description" }`
  on the expression (`desugar.rs`), then the parts are chained with `add`.

---

## HirExpr::OverloadSet

HIR-only (no syntax of its own).

Signature (`body.rs`):

```rust
OverloadSet {
    candidates: Vec<Entity>,
    type_args: Vec<HirTy>,
    span: Span,
}
```

Sources:

- `ExprPath` resolving to `ValueResolution::Overloaded` — `expr.rs`.
- Multi-candidate static-method resolution in `lower_call`:
  - base `MemberAccess` path — `expr.rs`.
  - multi-segment `Path` — `expr.rs`.

Type-infer: `generate.rs` errors if standalone (AmbiguousMember). In a
`HirExpr::Call` callee position, `generate.rs` dispatches via
`Constraint::OverloadedCall`.

Solver: `solve_overloaded_call` — picks by labels + arity, then type.

MIR: `body/expr.rs`, the `HirExpr::OverloadSet` arm: uses the winner type-infer recorded in `typed.resolutions[expr_id]`.

---

## HirExpr::Match with MatchSource

`HirExpr::Match` is produced by **nine** distinct sources. The `source: MatchSource`
tag lets analyzers skip exhaustiveness / unreachable-arm checks on synthetic matches
(`body.rs` — `is_desugared()`).

```rust
pub enum MatchSource {
    UserMatch,        // source code match
    IfLet,            // if let pattern = value { ... }
    WhileLet,         // while let pattern = value { ... }
    GuardLet,         // guard let pattern = value else { ... }
    ForLoop,          // for pattern in iter { ... }
    LetDestructure,   // let <complex_pattern> = value;
    ParamDestructure, // fn f((a, b): (I, I)) { ... } or { ((a, b)) in ... }
    TryOp,            // try expr
}
```

### MatchSource::UserMatch

- Trigger: `ExprMatch`.
- Site: `expr.rs` (`lower_match`) → allocated at `expr.rs`.
- Shape: direct 1:1 mapping of the source match.
- Analyzers: full exhaustiveness + redundancy checks apply.

### MatchSource::IfLet

- Trigger: `ExprIf` with `IfCondition::Let` — or any `if let pattern = value { ... }`.
- Site: `expr.rs` (inside `lower_if_conditions`, called from `lower_if` at
  `expr.rs`).
- Shape:
  ```
  Match {
      scrutinee: value,
      arms: [
          { pattern, guard: None, body: true_lit },
          { pattern: _, guard: None, body: false_lit },
      ],
      source: IfLet,
  }
  ```
  This reduces the `if let` to a boolean condition; the if-expr itself then wraps
  this bool match with its own then/else branches.
- Diagnostics: E302 fires on IfLet-specific analyzer issues.

### MatchSource::WhileLet

- Trigger: `ExprWhile (let)`.
- Site: `expr.rs` with `source: WhileLet` (from
  `desugar_while_let` at `desugar.rs`). The bool match produced here feeds
  into the negation + break check in `desugar_while_let` (`desugar.rs`).
- Full shape: `loop { if !<match_bool> { break } <body stmts> }`. See
  `desugar.rs` for the full flow.
- Diagnostics: E308.

### MatchSource::GuardLet

- Trigger: a `GuardStatement` with at least one `let` condition.
- Site: `lower_guard_cps` (`expr.rs`) CPS-desugars it: success = the remaining
  statements + tail (so bindings stay in scope), fail = the else body.
- A `guard` with no `let` lowers through `lower_guard` (`stmt.rs`) with
  `MatchSource::Guard`: `if <cond> { } else { <else_body> }` in a `HirStmt::Expr`,
  recorded in `hir.guard_stmts`.
- `kestrel-analyze/src/body/guard.rs` checks that every else body diverges.
- Diagnostics: E309.

### MatchSource::ForLoop

- Trigger: `ExprFor`.
- Site: `desugar.rs` (inside `desugar_for_loop`).
- Shape:
  ```
  Match {
      scrutinee: $iter.next(),       // ProtocolCall on Iterator
      arms: [
          { pattern: .Some(loop_pat), guard: None, body: <for body> },
          { pattern: .None, guard: None, body: break },
      ],
      source: ForLoop,
  }
  ```
- Gotcha: `$iter` is a temp local defined by `desugar_for_loop` at
  `desugar.rs` — the surrounding Block wraps the `let $iter = ...` stmt and
  the enclosing `HirExpr::Loop`.

### MatchSource::LetDestructure

- Trigger: `VariableDeclaration` with any pattern other than `BindingPattern`.
- Site: `stmt.rs` (inside `lower_let_stmt` at `stmt.rs`).
- Full shape:
  ```
  Block {
      stmts: [
          HirStmt::Let { local: $let_tmp, value: rhs, ... },
          HirStmt::Expr { Match { scrutinee: Local($let_tmp), arms: [{ pattern, body: () }], source: LetDestructure } },
      ],
      tail_expr: None,
  }
  ```
  The wrapping `HirStmt::Expr` at `stmt.rs` returns one statement to the caller.
- Gotcha: `var (a, b) = ...` propagates mutability into the sub-bindings via
  `lower_pat_forcing_mut` at `stmt.rs`.

### MatchSource::ParamDestructure

- Trigger: a fn, method, or closure parameter whose pattern isn't
  `BindingPattern` or `WildcardPattern`.
- Sites:
  - Closures: `expr.rs` (inside `lower_closure`). The synthetic param
    name is `_cparam_N`; the match is prepended to the closure body as a `HirStmt::Expr`.
  - For function/method params: see `lib/kestrel-hir-lower/src/lib.rs` (not included
    here, but the pattern is the same — lowered via `lower_param_pattern` at
    `pat.rs`). Also see `param_pattern` analyzer (E111) which emits a tuple-arity
    error and is specifically gated to skip `ParamDestructure`.
- Gotcha: `generate.rs` explicitly skips the scrutinee/pattern equate for
  `ParamDestructure` to avoid cascading the generic type-mismatch on top of E111.

### MatchSource::TryOp

- Trigger: `ExprTry`.
- Site: `desugar.rs` (inside `desugar_try`).
- Shape:
  ```
  Match {
      scrutinee: operand.tryExtract(),  // ProtocolCall on Tryable
      arms: [
          { pattern: .Continue($try_value), body: $try_value },
          { pattern: .Break($try_early), body: return .fromResidual(residual: $try_early) },
      ],
      source: TryOp,
  }
  ```
  Without the Tryable protocol: E128 and `Sugar { kind: Try, inner: Error }`.

### `x!` (force unwrap) — not a match

`x!` is not desugared to a `Match` (there is no `MatchSource::UnwrapOp`): it lowers
to `ForceUnwrap.forceUnwrap()` as a `ProtocolCall` through `POSTFIX_OP_PROTOCOLS`
(`desugar_postfix_op`); the `.None` trap is the stdlib's `fatalError`. See
`expressions.md` → `ExprPostfix` and `lib/kestrel-hir-lower/AGENTS.md`.

---

## HirExpr::If (synthetic)

User-written `ExprIf` produces `HirExpr::If` directly, but there are three
synthesis sites worth knowing about.

### Synthetic for `desugar_while`

- Site: `desugar.rs`.
- Shape: `if <cond> { } else { break }`. The condition is the unmodified
  `lower_expr(condition)`; the break exits the enclosing loop.
- Rationale comment at `desugar.rs`: avoids requiring the condition type to
  conform to `Not`.

### Synthetic for `desugar_while_let`

- Site: `desugar.rs`.
- Shape: `if <!cond> { break } else { }` — uses an explicit `ProtocolCall` on
  `LogicalNotOperatorProtocol` for the negation.

### Synthetic for `lower_guard` (no-`let` guards)

- Site: `stmt.rs` (`lower_guard`).
- Shape: `if <cond> { } else { <else_body> }`. The condition is a match-bool produced
  by `lower_if_conditions(..., MatchSource::Guard, ...)`. The else body is the user-written
  `else` block.
- Gotcha: `generate.rs` detects this via `is_guard_if` and skips the
  else-body type-equate, because the else block is required to diverge.

---

## HirExpr::Block (synthetic)

User-written `match-arm block` maps 1:1. Synthesis sites:

- Complex let-destructure wrapper: `stmt.rs`. Wraps
  `HirStmt::Let($let_tmp) + HirStmt::Expr(Match)` into a single `HirExpr::Block` so
  the caller receives one statement expression.
- `desugar_for_loop` body wrapper: `desugar.rs`. Wraps the loop body in a
  `HirExpr::Block` so all statements are reachable (match arms are exprs, and the body
  of `.Some(pat) => { body }` needs to be an expr).
- `desugar_for_loop` outer wrapper: `desugar.rs`. Wraps `let $iter = ...` +
  the enclosing `HirExpr::Loop` into one block expression.

---

## HirExpr::Tuple (synthetic)

Synthesized for:

- `ExprUnit` — `expr.rs` returns `HirExpr::Tuple { elements: vec![] }`
  directly from `lower_literal`. This is why unit values are tuples, not literals, in
  HIR.
- Match-arm unit body for let-destructure and param-destructure — `stmt.rs`
  and `expr.rs`.

---

## HirExpr::Local (synthetic) — temp conventions

Temp locals are $-prefixed so they can't collide with user identifiers. Full list:

| Local name    | Where                                                              |
| ------------- | ------------------------------------------------------------------ |
| `$let_tmp`    | complex let destructure (`stmt.rs`)                             |
| `$iter`       | for-loop iterator (`desugar.rs`)                               |
| `$try_value`  | try-expr `.Continue` payload (`desugar.rs`)                    |
| `$try_early`  | try-expr `.Break` payload (`desugar.rs`)                       |
| `$unwrap`     | unwrap `.Some` payload (`desugar.rs`)                          |
| `_cparam_N`   | complex closure param destructure (`expr.rs`)                 |

All of these are allocated via `define_local(name, is_mut, span)` which assigns a
fresh `LocalId` and records the local in `HirBody::locals`.

---

## HirExpr::ImplicitMember (synthetic) — `.Err` / `.fromResidual`

User-written `.Case` / `.Case(args)` maps 1:1 from `ExprImplicitMemberAccess`. Synthesis
sites:

- `desugar_throw`: `.Err(value)` at `desugar.rs`. The outer
  `HirExpr::Return` wraps it.
- `desugar_try`: `.fromResidual(residual: $try_early)` at `desugar.rs` when
  Tryable is available; `.Err($try_early)` fallback at `desugar.rs`.

These are resolved by `solve_implicit` against the function's return type.

---

## HirPat::ImplicitVariant (synthetic)

User-written `.Case` / `.Case(binding)` in pattern position maps from
`EnumPattern` that did NOT resolve to a concrete EnumCase (`pat.rs`).
Synthesized:

- for-loop match: `.Some(pattern)` (`desugar.rs`), `.None` (`desugar.rs`).
- try-expr match: `.Continue($v)` (`desugar.rs`), `.Break($e)`
  (`desugar.rs`). Fallback: `.Ok($v)` / `.Err($e)`.
- unwrap match: `.Some($v)` (`desugar.rs`), `.None` (`desugar.rs`).

---

## HirPat::Binding (synthetic shorthand expansion)

`StructPattern { fields: [{ field_name: "x", pattern: None }], ... }` (shorthand
`{ x }`) expands to `HirStructPatField { field_name: "x", pattern: Some(HirPat::Binding(x_local)) }`
at `pat.rs`. The `HirPat::Binding` here was never written by the user.

---

## Paren unwrapping

`ExprGrouping { inner, .. }` does not become a `HirExpr::Paren` — `expr.rs`
unwraps it:

```rust
ExprGrouping { inner, .. } => self.lower_expr(body, inner),
```

ExprGrouping records user-written grouping; precedence is already in the
tree (the parser applies it), so HIR just unwraps it.

---

## The "eight ways to get HirExpr::Match" cheat-sheet

| Source user writes              | MatchSource       | HIR-lowering function      |
| ------------------------------- | ----------------- | -------------------------- |
| `match x { ... }`               | `UserMatch`       | `lower_match`  |
| `if let p = v { ... }`          | `IfLet`           | `lower_if_conditions`  |
| `while let p = v { ... }`       | `WhileLet`        | `desugar_while_let`  |
| `guard let p = v else { ... }`  | `GuardLet`        | `lower_guard_cps` (`expr.rs`) |
| `for p in iter { ... }`         | `ForLoop`         | `desugar_for_loop`  |
| `let (a,b) = pair;` (complex)   | `LetDestructure`  | `lower_let_stmt`  |
| `fn f((a,b): (I,I)) { ... }` / `{ ((a,b)) in ... }` | `ParamDestructure` | `lower_closure`  or `lower_param_pattern` + lib.rs |
| `try expr`                      | `TryOp`           | `desugar_try`  |

---

## Cross-references

- For each surface construct, see `expressions.md` / `statements.md` for the syntax side.
- Pattern desugarings that produce `HirPat::*` variants (shorthand, `@`) — see
  `patterns.md`.
