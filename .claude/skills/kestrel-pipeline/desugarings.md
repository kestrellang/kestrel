# Desugarings — HIR-only constructs

HIR variants and shapes that don't have a 1:1 AST counterpart: constructs synthesized
during HIR lowering to reduce the surface language to a smaller core. "How did we get
this `HirExpr::X`?" is usually answered here.

All citations refer to `lib2/kestrel-hir-lower/src/` unless otherwise noted.

---

## HirExpr::ProtocolCall

ProtocolCall is **desugar-only** — there is no `AstExpr::ProtocolCall`. Every
ProtocolCall in the HIR was synthesized from one of the sites below.

Signature (`body.rs:172-179`):

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

Type-infer: `generate.rs:273-290` emits `Constraint::Conforms { ty: recv, protocol }`
AND `Constraint::Member { receiver, name: method, args, ... }`. Solver funnels are
`solve_conforms` (1076) and `solve_member` (1702).

MIR dispatch: `body_lower.rs:744-751` → `lower_protocol_call` (`body_lower.rs:2280`).
Emits witness-based method dispatch.

### Source: binary operators

- Trigger: `AstExpr::Binary` (including chained Pratt-parsed expressions).
- Site: `desugar.rs:21` (`desugar_binary_hir`) — invoked by `lower_binary_with_precedence`
  (`expr.rs:955`) for each reduction.
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
  `desugar_binary_hir` at `desugar.rs:57-70`.

### Source: short-circuit operators (`&&`, `||`, `??`)

- Trigger: `AstExpr::Binary` with a short-circuit op.
- Site: `desugar.rs:29-54` (inside `desugar_binary_hir`). RHS is wrapped in a
  parameterless `HirExpr::Closure` so the RHS protocol method can lazy-evaluate:
  ```
  ProtocolCall {
      receiver: lhs,
      protocol: <short-circuit protocol>,
      method: ...,
      args: [HirCallArg { value: Closure { params: [], body: { tail_expr: rhs } } }],
  }
  ```
- Gotcha: the captures list on the synthesized closure is empty at HIR time;
  `collect_captures` is only called for user-written closures (`expr.rs:1213`). MIR
  closure lowering walks the body for effective captures.

### Source: `desugar_logical_and` (multi-condition if/while/guard)

- Trigger: `if a, b, c { ... }` (comma-separated conditions combining into `a && b && c`).
- Site: `expr.rs:1130-1135` calls `desugar_logical_and` (`desugar.rs:78`) pair-wise
  over the condition list.
- Shape: same as short-circuit `&&` above.

### Source: unary operators

- Trigger: `AstExpr::Unary` (except `UnaryOp::Pos` which is identity).
- Site: `desugar.rs:116` (`desugar_unary_op`). Emits at `desugar.rs:133-140`:
  ```
  ProtocolCall {
      receiver: operand,
      protocol: <unary protocol>,
      method: "<method>",
      args: vec![],
  }
  ```

### Source: compound assignment

- Trigger: `AstExpr::CompoundAssignment`.
- Site: `desugar.rs:158` (`desugar_compound_assign`). Emits at `desugar.rs:170-181`:
  ```
  ProtocolCall {
      receiver: lhs,
      protocol: <compound-assign protocol>,
      method: ...,
      args: [HirCallArg { label, value: rhs }],
  }
  ```
- Gotcha: this is **not** `HirExpr::Assign(lhs, ProtocolCall(lhs, add, rhs))`. The
  compound-assign method mutates the receiver in place. Analyzers that scan for
  `HirExpr::Assign` targets won't see `+=`.

### Source: while-let negation

- Trigger: `AstExpr::WhileLet` (the condition is negated to compute the break
  condition).
- Site: `desugar.rs:280-288`:
  ```
  ProtocolCall {
      receiver: cond,
      protocol: LogicalNotOperatorProtocol,
      method: "logicalNot",
      args: vec![],
  }
  ```

### Source: for-loop `iter()` / `next()`

- Trigger: `AstExpr::For`.
- Sites:
  - `desugar.rs:351-358` — `iterable.iter()` via `IterableProtocol`.
  - `desugar.rs:380-387` — `$iter.next()` via `IteratorProtocol`.
- Fallback: if the protocol isn't resolvable, emits a plain `HirExpr::MethodCall`
  instead (`desugar.rs:360, 389`).

### Source: try-expr `tryExtract()`

- Trigger: `AstExpr::Try`.
- Site: `desugar.rs:509-516` — `operand.tryExtract()` via `TryableProtocol`. If the
  protocol isn't resolvable, the match scrutinee becomes the raw operand
  (`desugar.rs:517-520`) and the arms switch to `.Ok` / `.Err`.

### Source: interpolated-string concatenation

- Trigger: `AstExpr::InterpolatedString`.
- Site: `desugar.rs:766-777`:
  ```
  ProtocolCall {
      receiver: result_so_far,
      protocol: AddOperatorProtocol,
      method: "add",
      args: [HirCallArg { value: next_part }],
  }
  ```
- Each `StringPart::Interpolation` becomes a `HirExpr::MethodCall { method: "description" }`
  on the expression (`desugar.rs:740-746`), then the parts are chained with `add`.

---

## HirExpr::OverloadSet

HIR-only (no `AstExpr::OverloadSet`).

Signature (`body.rs:129-133`):

```rust
OverloadSet {
    candidates: Vec<Entity>,
    type_args: Vec<HirTy>,
    span: Span,
}
```

Sources:

- `AstExpr::Path` resolving to `ValueResolution::Overloaded` — `expr.rs:344-350`.
- Multi-candidate static-method resolution in `lower_call`:
  - base `MemberAccess` path — `expr.rs:508-513`.
  - multi-segment `Path` — `expr.rs:578-584`.

Type-infer: `generate.rs:108-115` errors if standalone (AmbiguousMember). In a
`HirExpr::Call` callee position, `generate.rs:120-132` dispatches via
`Constraint::OverloadedCall`.

Solver: `solve_overloaded_call` (1379) — picks by labels + arity, then type.

MIR: `body_lower.rs:721-733` — resolves via `typed.resolutions[expr_id]`; falls back
to first candidate if inference didn't resolve. See MEMORY
`static_overload_first_match_truncation.md`.

---

## HirExpr::Match with MatchSource

`HirExpr::Match` is produced by **nine** distinct sources. The `source: MatchSource`
tag lets analyzers skip exhaustiveness / unreachable-arm checks on synthetic matches
(`body.rs:76-82` — `is_desugared()`). See `match_pattern_analyzer.md` in MEMORY.

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
    UnwrapOp,         // expr!
}
```

### MatchSource::UserMatch

- Trigger: `AstExpr::Match`.
- Site: `expr.rs:1226` (`lower_match`) → allocated at `expr.rs:1252`.
- Shape: direct 1:1 mapping of the source match.
- Analyzers: full exhaustiveness + redundancy checks apply.

### MatchSource::IfLet

- Trigger: `AstExpr::If` with `IfCondition::Let` — or any `if let pattern = value { ... }`.
- Site: `expr.rs:1093-1125` (inside `lower_if_conditions`, called from `lower_if` at
  `expr.rs:1048`).
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

- Trigger: `AstExpr::WhileLet`.
- Site: `expr.rs:1093-1125` with `source: WhileLet` (from
  `desugar_while_let` at `desugar.rs:271`). The bool match produced here feeds
  into the negation + break check in `desugar_while_let` (`desugar.rs:279-301`).
- Full shape: `loop { if !<match_bool> { break } <body stmts> }`. See
  `desugar.rs:259` for the full flow.
- Diagnostics: E308.

### MatchSource::GuardLet

- Trigger: `AstStmt::GuardLet`.
- Site: `stmt.rs:155` calls `lower_if_conditions(..., MatchSource::GuardLet, ...)`.
- Full shape: `if <cond> { } else { <else_body> }` wrapped in `HirStmt::Expr`.
- Pushed into `ctx.guard_let_stmts` (`stmt.rs:180`) so the
  `guard_let_divergence` analyzer can verify the else block diverges.
- Diagnostics: E309.

### MatchSource::ForLoop

- Trigger: `AstExpr::For`.
- Site: `desugar.rs:434-450` (inside `desugar_for_loop`).
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
  `desugar.rs:369-374` — the surrounding Block wraps the `let $iter = ...` stmt and
  the enclosing `HirExpr::Loop`.

### MatchSource::LetDestructure

- Trigger: `AstStmt::Let` with any pattern other than `AstPat::Binding`.
- Site: `stmt.rs:110-119` (inside `lower_let_stmt` at `stmt.rs:87-138`).
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
  The wrapping `HirStmt::Expr` at `stmt.rs:133-136` returns one statement to the caller.
- Gotcha: `var (a, b) = ...` propagates mutability into the sub-bindings via
  `lower_pat_forcing_mut` at `stmt.rs:101`.

### MatchSource::ParamDestructure

- Trigger: a fn, method, or closure parameter whose pattern isn't
  `AstPat::Binding` or `AstPat::Wildcard`.
- Sites:
  - Closures: `expr.rs:1179-1188` (inside `lower_closure`). The synthetic param
    name is `_cparam_N`; the match is prepended to the closure body as a `HirStmt::Expr`.
  - For function/method params: see `lib2/kestrel-hir-lower/src/lib.rs` (not included
    here, but the pattern is the same — lowered via `lower_param_pattern` at
    `pat.rs:410`). Also see `param_pattern` analyzer (E111) which emits a tuple-arity
    error and is specifically gated to skip `ParamDestructure`.
- Gotcha: `generate.rs:605-612` explicitly skips the scrutinee/pattern equate for
  `ParamDestructure` to avoid cascading the generic type-mismatch on top of E111.

### MatchSource::TryOp

- Trigger: `AstExpr::Try`.
- Site: `desugar.rs:590-606` (inside `desugar_try`).
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
  Fallback (no Tryable protocol): arms are `.Ok($v) => $v` and `.Err($e) => return .Err($e)`.

### MatchSource::UnwrapOp

- Trigger: `AstExpr::Postfix(Unwrap)` (the `x!` syntax).
- Site: `desugar.rs:672-688` (inside `desugar_unwrap`).
- Shape:
  ```
  Match {
      scrutinee: operand,
      arms: [
          { pattern: .Some($unwrap), body: $unwrap },
          { pattern: .None, body: Error { span } },   // trap placeholder
      ],
      source: UnwrapOp,
  }
  ```
- Gotcha: the `.None` body is `HirExpr::Error` as a trap placeholder; MIR currently
  emits `Immediate::error()`. A proper panic intrinsic isn't wired yet. See MEMORY
  `funcref_to_functhick_coercion.md` for related test-process fallout.

---

## HirExpr::If (synthetic)

User-written `AstExpr::If` produces `HirExpr::If` directly, but there are three
synthesis sites worth knowing about.

### Synthetic for `desugar_while`

- Site: `desugar.rs:221-232`.
- Shape: `if <cond> { } else { break }`. The condition is the unmodified
  `lower_expr(condition)`; the break exits the enclosing loop.
- Rationale comment at `desugar.rs:199-203`: avoids requiring the condition type to
  conform to `Not`.

### Synthetic for `desugar_while_let`

- Site: `desugar.rs:293-301`.
- Shape: `if <!cond> { break } else { }` — uses an explicit `ProtocolCall` on
  `LogicalNotOperatorProtocol` for the negation.

### Synthetic for `lower_guard_let`

- Site: `stmt.rs:164-172`.
- Shape: `if <cond> { } else { <else_body> }`. The condition is a match-bool produced
  by `lower_if_conditions(..., GuardLet, ...)`. The else body is the user-written
  `else` block.
- Gotcha: `generate.rs:376-380` detects this via `is_guard_let_if` and skips the
  else-body type-equate, because the else block is required to diverge.

---

## HirExpr::Block (synthetic)

User-written `AstExpr::Block` maps 1:1. Synthesis sites:

- Complex let-destructure wrapper: `stmt.rs:126-131`. Wraps
  `HirStmt::Let($let_tmp) + HirStmt::Expr(Match)` into a single `HirExpr::Block` so
  the caller receives one statement expression.
- `desugar_for_loop` body wrapper: `desugar.rs:417-420`. Wraps `lower_for_body` in a
  `HirExpr::Block` so all statements are reachable (match arms are exprs, and the body
  of `.Some(pat) => { body }` needs to be an expr).
- `desugar_for_loop` outer wrapper: `desugar.rs:468-474`. Wraps `let $iter = ...` +
  the enclosing `HirExpr::Loop` into one block expression.

---

## HirExpr::Tuple (synthetic)

Synthesized for:

- `AstLiteral::Unit` — `expr.rs:209-214` returns `HirExpr::Tuple { elements: vec![] }`
  directly from `lower_literal`. This is why unit values are tuples, not literals, in
  HIR.
- Match-arm unit body for let-destructure and param-destructure — `stmt.rs:106-109`
  and `expr.rs:1175-1178`.

---

## HirExpr::Local (synthetic) — temp conventions

Temp locals are $-prefixed so they can't collide with user identifiers. Full list:

| Local name    | Where                                                              |
| ------------- | ------------------------------------------------------------------ |
| `$let_tmp`    | complex let destructure (`stmt.rs:90`)                             |
| `$iter`       | for-loop iterator (`desugar.rs:369`)                               |
| `$try_value`  | try-expr `.Continue` payload (`desugar.rs:531`)                    |
| `$try_early`  | try-expr `.Break` payload (`desugar.rs:548`)                       |
| `$unwrap`     | unwrap `.Some` payload (`desugar.rs:649`)                          |
| `_cparam_N`   | complex closure param destructure (`expr.rs:1163`)                 |

All of these are allocated via `define_local(name, is_mut, span)` which assigns a
fresh `LocalId` and records the local in `HirBody::locals`.

---

## HirExpr::ImplicitMember (synthetic) — `.Err` / `.fromResidual`

User-written `.Case` / `.Case(args)` maps 1:1 from `AstExpr::ImplicitMember`. Synthesis
sites:

- `desugar_throw`: `.Err(value)` at `desugar.rs:618-625`. The outer
  `HirExpr::Return` wraps it.
- `desugar_try`: `.fromResidual(residual: $try_early)` at `desugar.rs:566-573` when
  Tryable is available; `.Err($try_early)` fallback at `desugar.rs:576-583`.

These are resolved by `solve_implicit` against the function's return type.

---

## HirPat::ImplicitVariant (synthetic)

User-written `.Case` / `.Case(binding)` in pattern position maps from
`AstPat::Enum` that did NOT resolve to a concrete EnumCase (`pat.rs:279-293`).
Synthesized:

- for-loop match: `.Some(pattern)` (`desugar.rs:401-408`), `.None` (`desugar.rs:423-427`).
- try-expr match: `.Continue($v)` (`desugar.rs:536-542`), `.Break($e)`
  (`desugar.rs:553-559`). Fallback: `.Ok($v)` / `.Err($e)`.
- unwrap match: `.Some($v)` (`desugar.rs:654-660`), `.None` (`desugar.rs:665-668`).

---

## HirPat::Binding (synthetic shorthand expansion)

`AstPat::Struct { fields: [{ field_name: "x", pattern: None }], ... }` (shorthand
`{ x }`) expands to `HirStructPatField { field_name: "x", pattern: Some(HirPat::Binding(x_local)) }`
at `pat.rs:313-321`. The `HirPat::Binding` here was never written by the user.

---

## Paren unwrapping

`AstExpr::Paren { inner, .. }` does not become a `HirExpr::Paren` — `expr.rs:185`
unwraps it:

```rust
AstExpr::Paren { inner, .. } => self.lower_expr(body, inner),
```

AstExpr::Paren exists only so the Pratt parser in `lower_binary_with_precedence`
doesn't flatten across user-written grouping.

---

## The "eight ways to get HirExpr::Match" cheat-sheet

| Source user writes              | MatchSource       | HIR-lowering function      |
| ------------------------------- | ----------------- | -------------------------- |
| `match x { ... }`               | `UserMatch`       | `lower_match` (expr.rs:1226) |
| `if let p = v { ... }`          | `IfLet`           | `lower_if_conditions` (expr.rs:1076) |
| `while let p = v { ... }`       | `WhileLet`        | `desugar_while_let` (desugar.rs:259) |
| `guard let p = v else { ... }`  | `GuardLet`        | `lower_guard_let` (stmt.rs:143) + `lower_if_conditions` |
| `for p in iter { ... }`         | `ForLoop`         | `desugar_for_loop` (desugar.rs:338) |
| `let (a,b) = pair;` (complex)   | `LetDestructure`  | `lower_let_stmt` (stmt.rs:64) |
| `fn f((a,b): (I,I)) { ... }` / `{ ((a,b)) in ... }` | `ParamDestructure` | `lower_closure` (expr.rs:1139) or `lower_param_pattern` + lib.rs |
| `try expr`                      | `TryOp`           | `desugar_try` (desugar.rs:499) |
| `expr!`                         | `UnwrapOp`        | `desugar_unwrap` (desugar.rs:640) |

---

## Cross-references

- For each surface construct, see `expressions.md` / `statements.md` for the AST side.
- Pattern desugarings that produce `HirPat::*` variants (shorthand, `@`) — see
  `patterns.md`.
- Historical cascading-error fixes from pattern desugaring —
  `cascading_infer_errors.md`.
- Method / witness dispatch funnel (MIR side) — `dispatch_funnel_pattern.md`.
