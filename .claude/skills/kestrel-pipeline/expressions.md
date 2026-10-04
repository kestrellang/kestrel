# Expressions — CST → HIR → infer → MIR

Covers every `Expr` syntax kind and `HirExpr` (23). Verify before citing — pipeline
maps go stale.

Top-level dispatch anchors:

- HIR lowering switch: `lib/kestrel-hir-lower/src/expr.rs` (`LowerCtx::lower_expr` →
  `lower_expr_node`)
- Inference gen switch: `lib/kestrel-type-infer/src/generate.rs:60` (`gen_expr`)
- Constraint dispatch: `lib/kestrel-type-infer/src/solver.rs:583` (`try_solve`)
- MIR lowering switch: `lib/kestrel-mir-lower/src/body_lower.rs:445` (`lower_expr`)

Solver functions (cite these when tracing a constraint):
`solve_equal` 817, `solve_coerce` 955, `solve_conforms` 1076, `solve_associated` 1099,
`solve_call` 1251, `solve_overloaded_call` 1379, `solve_member` 1702,
`solve_implicit` 2182, `solve_implicit_pat` 2293, `solve_tuple_rest_pat` 2401,
`solve_reduce` 716 (all in `solver.rs`).

---

## Expression syntax → HIR (every `Expr` kind)

Body lowering reads the CST directly: `lib/kestrel-hir-lower/src/expr.rs` `lower_expr` /
`lower_expr_node` match on the `SyntaxKind` of the (unwrapped) `Expression`; the few
multi-child syntax decisions live in `lib/kestrel-hir-lower/src/syntax.rs`. Every lowered
node is recorded in the body's `BodySourceMap`.

### Literals — `ExprInteger` `ExprFloat` `ExprString` `ExprRawString` `ExprChar` `ExprBool` `ExprNull` `ExprUnit`

- Surface: `42`, `3.14`, `"s"`, `'c'`, `true`, `null`, `()`.
- CST: one token in the node (`ExprUnit` is `(` `)`).
- HIR lowering: `lower_expr_node` reads the token text (`literal_text`): `HirLiteral::Integer(parse_int)` / `Float(parse_float)`,
  strings through `literal::decode_string_literal_token` (escape errors kept as data), chars through
  `pat::parse_char_validated` (E709/E710), `Bool`, `Null`. **`ExprUnit` lowers to
  `HirExpr::Tuple { elements: [] }`**, not a literal.
- Type-infer: `generate.rs:63-70` — each `HirLiteral` variant calls
  `ctx.fresh_literal(LiteralKind::X)`. Literal kinds defer to a default via
  `DefaultXLiteralType` builtin until coerced.
- Solver: `solve_equal` / `solve_coerce` pick the default when no other
  constraint pins the literal.
- MIR: `body_lower.rs:448` (`lower_literal_expr`) → `Immediate`.
- Gotchas: `()` never reaches the HIR as a Literal — it's always a zero-element Tuple.
  Integer overflow bug history: `MEMORY.md → integer_literal_overflow_silent_zero.md`.

### `ExprInterpolatedString`

- Surface: `"hello \(name)!"`.
- CST: `ExprInterpolatedString > StringStart (StringFragment | StringInterpolation)* StringEnd`,
  `StringInterpolation > InterpStart (Expression | Error) FormatSpecifier? InterpEnd`.
  Holes are parsed in place by the main parser; a hole that fails to parse is an
  `Error` node and its errors read "invalid expression in string interpolation: …".
- HIR lowering: `desugar.rs` `desugar_interpolated_string` → `interpolated_string_parts`
  decodes the fragments with the shared escape table (strips multi-line indentation
  through placeholders, `string_token::process_multiline_body`) and keeps each hole as
  its node; then builds `var $dsi = DefaultStringInterpolation(literalCapacity:,
  interpolationCount:)`, one `$dsi.appendLiteral(_)` / `$dsi.appendInterpolation(_, opts?)`
  `MethodCall` per part (format spec → `build_format_options_from_spec`, E708), tail
  `$dsi.build()`, all wrapped in `HirExpr::Sugar { kind: StringInterpolation }`.
- Lexer: modal (`kestrel-lexer/src/modal.rs`) —
  `StringStart (StringFragment | InterpStart <tokens> (Colon FormatSpec)? InterpEnd)* StringEnd`;
  a string with no hole stays ONE `String` token (`ExprString`).
- Type-infer: receiver / arg for each chained `add` call — see `HirExpr::ProtocolCall`
  entry below.
- Solver: `solve_member` + `solve_conforms` (via ProtocolCall).
- MIR: falls through the `ProtocolCall` / `MethodCall` paths.
- Gotchas: a `StringPart::Literal` is decoded once, in `interpolated_string_parts`;
  nothing downstream decodes it again.

### `ExprArray`

- Surface: `[1, 2, 3]`.
- CST: `[` `Expression`* `]`.
- HIR lowering: `lower_expr_node` → `HirExpr::Array { elements, span }` (1:1).
- Type-infer: `generate.rs:463-487` — `fresh_literal(Array)` + `Associated(arr, "Element", elem)`
  + `Equal(elem, e_tv)` for each element. Bidirectional hint via `ctx.expected_array_elem`
  (set by the surrounding `let`-binding when the annotation is `Array[E]`).
- Solver: `solve_associated` (1099) resolves Element; `solve_equal` (817) for each element.
- MIR: `body_lower.rs:829-848` — prefers `lower_array_literal_via_init` (for custom
  `ExpressibleByArrayLiteral` types) before falling back to `Rvalue::ArrayLiteral`.
- Gotchas: element-type flow is bidirectional — see `cascading_infer_errors.md`.

### `ExprDictionary`

- Surface: `[k1: v1, k2: v2]` (or `[:]` for empty).
- CST: `DictionaryEntry > key:Expression : value:Expression` children.
- HIR lowering: `lower_expr_node` → `HirExpr::Dict { entries }` (an entry missing its key or value is dropped).
- Type-infer: `generate.rs:489-531` — `fresh_literal(Dictionary)` + `Associated("Key")`
  + `Associated("Value")` + `Equal(key_tv, k_tv)` / `Equal(val_tv, v_tv)` per entry.
  Bidirectional hint via `ctx.expected_dict_entry`. Uses
  `emit_dict_literal_acceptance_error` for per-key/value mismatch phrasing.
- Solver: `solve_associated` + `solve_equal`.
- MIR: `body_lower.rs:851-889` — lowered as an `ArrayLiteral` of `(K, V)` tuples.
- Gotchas: empty `[:]` still gets a default type via the literal default mechanism.

### `ExprTuple`

- Surface: `(a, b, c)`.
- CST: `(` `Expression` (`,` `Expression`)+ `)`.
- HIR lowering: `lower_expr_node` → `HirExpr::Tuple { elements }`.
- Type-infer: `generate.rs:533-536` — `ctx.tuple(elem_tvs)`.
- Solver: `solve_equal` (tuples unify structurally).
- MIR: `body_lower.rs:450-458` — `Rvalue::Tuple(values)` into a fresh temp.
- Gotchas: `ExprUnit` becomes `HirExpr::Tuple { elements: [] }`,
  not a HIR literal — see `expr.rs`.

### `ExprPath` — a path (`a.b.c`)

- Surface: `a`, `a.b.c`, `Pointer[UInt8]`, `Type.case`.
- CST: `ExprPath` whose children are identifier tokens (`(Expression '.')? ident TypeArgumentList? ('.' ident TypeArgumentList?)*`).
  `syntax::PathSyntax::of` reads it: `PathBase::Segments(Vec<PathSeg>)` with no members
  (a trailing `.` with no name is a member whose name is missing).
- HIR lowering: `expr.rs` `lower_path` (via `lower_path_chain`). Scope decides what the
  segments are — the path-vs-member decision is made here, not in the syntax:
  - first segment a local (`lookup_local`, no type args) → `HirExpr::Local` + one
    `HirExpr::Field` per trailing segment (`lower_trailing_member_segments`).
  - local with type args → E130 + `HirExpr::Error`; `self` with no receiver → E135.
  - first segment a type parameter (multi-segment) → `HirExpr::Def(T)` + `Field` chain.
  - else `ResolveValuePath`: `Def | TypeParameter | SelfValue` → `HirExpr::Def(entity, type_args)`;
    `Overloaded` → `HirExpr::OverloadSet`; `EnumCaseValue` / `FieldValue` → `Def` + trailing
    `Field`s; `AssociatedType { container: Some }` → `lower_type_receiver_path` (`TypeRef`);
    `AssociatedTypeStaticMember` → `Field { base: TypeRef }`; `Ambiguous` → E133,
    `SelfNotInScope` → E134, `NotFound` → E132, each `HirExpr::Error`.
  - `Type.instanceMethod` used as a value (not in callee position) → E100.
  Every segment that lowers to its own expression is recorded in the body's
  `BodySourceMap` (`alloc_seg` / `record_segments`).
- Type-infer: per-variant — see `HirExpr::Local` / `::Def` / `::OverloadSet` / `::Field`.
- Gotchas: `Type.method()` parses as a **2-segment path**, not a member access on a
  type — dispatched in `lower_path_call`, not `lower_path`. See `ExprCall`.

### `ExprPath` — member access on a computed base (`f().x`)

- Surface: `expr.field`, `expr.method[T]` (with explicit type args).
- CST: `ExprPath` whose first child is an `Expression` (the base), then `. ident TypeArgumentList?`
  members (`PathSyntax { base: PathBase::Expr, members }`). `a.b().c.d` is
  `ExprPath[Expression(ExprCall(…)) . c . d]`.
- HIR lowering: **context-sensitive.**
  - Standalone: `lower_path_chain` → lower the base, then `HirExpr::Field { base, name }` per
    member (spans = the whole `ExprPath`; member type args dropped). A missing member
    name is `HirName::Missing`.
  - As a callee: `lower_member_call` — `Type[Args].staticMethod()` (base a plain path
    naming a struct/enum, `static_call_base` + `try_resolve_static_call`) →
    `HirExpr::Call { callee: Def | OverloadSet }`; otherwise
    `HirExpr::MethodCall { receiver, method, type_args, args }`.
- Type-infer: see `HirExpr::Field` or `HirExpr::MethodCall`.
- Gotchas: explicit type args (`x.method[Int]`) are only meaningful for method calls;
  the `type_args` on a standalone `MemberAccess` are dropped at HIR lowering (`expr.rs`).

### `ExprTupleIndex`

- Surface: `pair.0`, `triple.2`.
- CST: `Expression . int`.
- HIR lowering: `lower_expr_node` → `HirExpr::TupleIndex { base, index, span }`.
- Type-infer: `generate.rs:316-335` — emits a `Member` constraint with the index as
  `name` (string form). Solver's `solve_member` recognizes numeric names for tuples.
- Solver: `solve_member` (1702).
- MIR: `body_lower.rs:575-588` — `Place::index` if base is a `Place`, else materialize
  to temp then index.

### `ExprImplicitMemberAccess`

- Surface: `.Some(x)`, `.None`, `.fromResidual(...)`.
- CST: `. Name ArgumentList?`.
- HIR lowering: `lower_expr_node` → `HirExpr::ImplicitMember { name, args, span }` (a missing name is
  `HirName::Missing`).
- Type-infer: `generate.rs:338-347` — emits `Constraint::Implicit { expected, name, args }`
  (expected = fresh result TyVar — must be pinned by surrounding context).
- Solver: `solve_implicit` (2182).
- MIR: `body_lower.rs:761-826` — dispatches on resolved entity:
  - EnumCase → `Rvalue::EnumVariant`.
  - Protocol static method → `Callee::witness`.
  - Regular static → `Callee::direct_generic`.
- Gotchas: `.fromResidual(residual: early)` is emitted by try-expr desugaring
  (`desugar.rs`); it's a protocol static method, not an enum case.

### `ExprUnary`

- Surface: `-x`, `not b`, `!b` (bitwise), `+x`.
- CST: operator token (`-` `+` `!` `not` `..<` `..=` `&` `&mutating`) + `Expression`;
  `syntax::unary_op` maps it (an unknown token → `HirExpr::Error`, never a guess).
- HIR lowering: `desugar_unary_op` (`desugar.rs`) emits `HirExpr::ProtocolCall { protocol: <unary
  proto>, method }`. **`+x` is identity.** `&`/`&mutating` outside a `let` initializer →
  E488 (the operand is still lowered); inside one, `stmt.rs` `lower_let_stmt` makes a
  `HirExpr::Borrow`.
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: desugar-only — there is no `HirExpr::Unary`.

### `ExprPostfix`

- Surface: `x!` (unwrap).
- CST: `Expression ('!' | '..')`.
- HIR lowering: `desugar_postfix_op` → `HirExpr::ProtocolCall` through `POSTFIX_OP_PROTOCOLS`
  (`!` = `ForceUnwrap.forceUnwrap()`, `..` = `rangeFrom`); E124 when the protocol is missing.
- Type-infer: see `HirExpr::Match`.
- Gotchas: the `None` arm body is currently `HirExpr::Error` as a trap placeholder
  (`desugar.rs`) — MIR treats it as `Immediate::error()`. See `funcref_to_functhick_coercion.md`
  for unwrap-related unkillable zombie tests.

### `ExprBinary`

- Surface: `a + b`, `a == b`, `a && b`, `a .. b`, `a ?? b`.
- CST: `lhs:Expression <op> rhs:Expression`.
- HIR lowering: `lower_expr_node` lowers lhs, rhs, then `desugar_binary_hir` (`desugar.rs`):
  - Short-circuit ops (`and`, `or`, `??`) wrap RHS in a parameterless `HirExpr::Closure`
    then emit `HirExpr::ProtocolCall`.
  - Regular ops emit `HirExpr::ProtocolCall` directly.
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: all operators route through protocols — if a builtin protocol isn't
  resolvable, `desugar_binary_hir` emits `HirExpr::Error` and a diagnostic ("is the
  standard library imported?") `desugar.rs`.

### `ExprAssignment`

- Surface: `x = y`, `obj.prop = v`, `Foo.staticVar = v`.
- CST: `lhs:Expression = rhs:Expression`.
- HIR lowering: `lower_expr_node` → `HirExpr::Assign { target, value, span }` (direct).
- Type-infer: `generate.rs:448-457` — `coerce(value_tv, target_tv)`, result is unit.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:607-624`. **Setter detour**: `try_lower_setter_assign` at 612
  dispatches computed-property assigns (`obj.computed = v`, `Foo.staticComputed = v`,
  global computed vars) through a Setter child entity instead of emitting a stored-Place
  write. Falls back to `StatementKind::Assign` with a `Place` destination.
- Gotchas: `self.field = v` inside an initializer is a stored-field write, not a
  settable check. Mutability on the LHS comes from the binding, not the expression.

### `ExprCompoundAssignment`

- Surface: `x += y`, `x *= y`, `x <<=`, etc.
- CST: `lhs:Expression <op>= rhs:Expression`.
- HIR lowering: `desugar_compound_assign` (`desugar.rs`): the LHS must be place-shaped
  (`is_place_syntax`: a path, member access, tuple index, or a grouping of one) or
  call-shaped, else E213; emits `HirExpr::Sugar { kind: CompoundAssign, inner:
  ProtocolCall { protocol: <compound-assign proto>, receiver: lhs, args: [rhs] } }`.
  **Not** `HirExpr::Assign`.
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: `x += y` does NOT lower as `Assign(x, ProtocolCall(x, add, y))` — it's a
  single `ProtocolCall` on a compound-assign protocol whose method mutates the receiver
  in-place. Catch this when reviewing analyzers that look for `Assign` targets.

### `ExprCall`

- Surface: `foo(x)`, `obj.method(x)`, `Type.staticMethod(x)`, `dict(key)` (subscript).
- CST: `Expression ArgumentList` (parenthesised args, then trailing closures, in order).
- HIR lowering: `expr.rs` `lower_call`. Arguments first (`lower_call_args`), then the callee:
  1. member access on a computed base → `lower_member_call` (static method on a type →
     `Call { callee: Def | OverloadSet }`, else `MethodCall`).
  2. a path with ≥ 2 segments → `lower_path_call`:
     - `self.init(…)` outside an initializer → E136.
     - first segment a local → `MethodCall { receiver: Local + Field chain (lower_path_prefix) }`.
     - static method via `try_resolve_static_call_from_segments` → `Call { callee: Def | OverloadSet }`.
     - instance method named through a type → E100 + `HirExpr::Error`.
     - type-level (type parameter / associated-type prefix) → `MethodCall { receiver: Def(T) | TypeRef }`.
     - value prefix (a static field / enum case value) → `MethodCall { receiver: lower_path(prefix) }`.
     - type prefix + protocol-extension static → `MethodCall { receiver: Def(Type) }`.
     - otherwise → `Call { callee: lower_callee(path) }`.
  3. any other callee → `Call { callee: lower_callee(callee) }` (`in_callee_position` exempts
     a path from the "instance method used as a value" check).
- Type-infer: `generate.rs:118-216`. Arm structure:
  - If callee is `HirExpr::OverloadSet` → `ctx.overloaded_call(candidates, type_args, ...)`
    (120-132).
  - If callee is `HirExpr::Def(struct)` → `gen_struct_init` (138-143).
  - If callee is `HirExpr::Def(enum_case)` with `Callable` → overloaded-call route for
    label checking (147-161).
  - If callee is `HirExpr::Def(function)` with mismatching labels → pre-emit
    `NoMatchingOverload` (165-188) so tests get the richer diagnostic instead of
    "wrong number of arguments" from `solve_call`.
  - Otherwise: `ctx.call(callee_tv, arg_tvs, result_tv, ...)` (`Constraint::Call`).
    Bonus: if callee resolves to a fn with `HirTy::Never` return, use
    `ctx.never()` for result (197-212) so divergence propagates.
- Solver: `solve_call` (1251) or `solve_overloaded_call` (1379).
- MIR: `body_lower.rs:736` → `lower_call` (`body_lower.rs:1867`).
- Gotchas: `Type.method()` (multi-segment path, no explicit local prefix) is a pure
  `ExprPath` in the CST. Dispatch happens in `lower_path_call`, not `lower_path`. See also
  `static_overload_first_match_truncation.md` in MEMORY.

### `ExprIf`

- Surface: `if c { ... }`, `if let p = v { ... } else if ... { ... } else { ... }`.
- CST: `if Condition (, Condition)* CodeBlock ElseClause?`; conditions are
  `IfLetCondition` or `Expression` (`syntax::if_conditions` → `Vec<Cond>`).
- HIR lowering: `expr.rs` `lower_if`. Any `let` condition → `lower_condition_chain` with
  `MatchSource::IfLet` (nested `Match`/`If` so bindings dominate the then-block; the
  else is lowered once and shared). Otherwise `lower_if_conditions` (single expression;
  multiple conditions AND-ed through `desugar_binary_hir(And)`) and
  `HirExpr::If { condition, then_body, else_body }`.
- Type-infer: `generate.rs:351-386`. Generates block types for both branches and
  `ctx.equal(then_tv, result)` / `ctx.equal(else_tv, result)` — but **skips the
  else-equate when the If came from a guard-let desugaring** (`is_guard_let_if` helper)
  since the else block is required to diverge. No else → result type is unit.
- Solver: `solve_equal`.
- MIR: `body_lower.rs:590-595` → `lower_if` at `body_lower.rs:2817`.
- Gotchas: an `if`-expression with no `else` has unit result even if the `then` block
  has a typed tail — this comes from `generate.rs:382-385`.

### `ExprWhile` (no `let` condition)

- Surface: `while cond { body }`, `'label: while cond { body }`.
- CST: `LoopLabel? while Condition (, Condition)* CodeBlock`.
- HIR lowering: `desugar_while` (`desugar.rs`), on the **first** condition expression. Emits:
  ```
  HirExpr::Loop {
      stmts: [ HirStmt::Expr { HirExpr::If { cond, then: {}, else: Some({ break }) } } ] ++ body.stmts,
      tail_expr: body.tail_expr,
  }
  ```
  — the condition also gets pushed into `while_conditions` for the condition-type analyzer.
- Type-infer: see `HirExpr::Loop`.
- Gotchas: `while` desugars to `if cond {} else { break }` (positive test) instead of
  `if !cond { break }` — avoids requiring the condition type to conform to `Not`
  (`desugar.rs`).

### `ExprWhile` with a `WhileLetCondition`

- Surface: `while let .Some(x) = iter.next() { ... }`.
- CST: same `ExprWhile`; a `WhileLetCondition` child selects this lowering
  (`syntax::let_conditions(node, WhileLetCondition)`).
- HIR lowering: `desugar_while_let`. A single `let` → `desugar_while_let_single`:
  `loop { match v { pat => body, _ => break } }` (`MatchSource::WhileLet`), body inside the
  arm so move-check sees the binding as initialized. Several conditions →
  `desugar_while_let_chain` through `lower_condition_chain` (fail = `break`).
- Type-infer: see `HirExpr::Loop` + `HirExpr::Match` (the desugared let-condition
  becomes a match with `source: WhileLet`).
- Gotchas: bindings from the let-condition live until the loop ends — the scope push is
  around the condition AND body (`desugar.rs`).

### `ExprLoop`

- Surface: `loop { ... }`, `'label: loop { ... }`.
- CST: `LoopLabel? loop CodeBlock`.
- HIR lowering: `lower_expr_node` pushes the label (`push_loop`), lowers the block, emits
  `HirExpr::Loop { label, body }`.
- Type-infer: `generate.rs:426-430` — result type is `Never` unless a `break` with a
  value pins it (the break-value analysis happens through the break's flow into the
  block type).
- Solver: N/A (Never/concrete types resolve directly).
- MIR: `body_lower.rs:596` → `lower_loop` at `body_lower.rs:2875`.
- Gotchas: `loop { break 5 }` as an expression relies on the break-value path; plain
  `loop {}` is `!` (Never).

### `ExprFor`

- Surface: `for x in iter { ... }`, `for (a, b) in pairs { ... }`.
- CST: `LoopLabel? for ForPattern in ForIterable CodeBlock`.
- HIR lowering: `desugar_for_loop` (`desugar.rs`). Emits `HirExpr::Sugar { kind: ForLoop }` around:
  ```
  let $iter = iterable.iter()      // ProtocolCall on Iterable (E127 if missing)
  loop {
      match $iter.next() {         // ProtocolCall on Iterator
          .Some(pattern) => { body }
          .None => break
      }
  }
  ```
  — `match` uses `MatchSource::ForLoop`.
- Type-infer: see `HirExpr::Loop`, `HirExpr::Match`, `HirExpr::ProtocolCall`.
- Gotchas: the match is `source: ForLoop` — analyzers should skip exhaustiveness checks
  on it (see `match_pattern_analyzer.md`).

### `ExprBreak`

- Surface: `break`, `'label: loop { break 'label }`.
- CST: `break label:ident?`.
- HIR lowering: `lower_expr_node` → `validate_break_continue` (E010 outside a loop, E011 unknown label)
  then `HirExpr::Break { label, span }`.
- Type-infer: `generate.rs:432` — type is `Never`.
- MIR: `body_lower.rs:597` → `lower_break` at `body_lower.rs:2909`.
- Gotchas: `break` with a value is not yet represented — the grammar and HIR carry a
  label only.

### `ExprContinue`

- Surface: `continue`, `'label: loop { continue 'label }`.
- CST: `continue label:ident?`.
- HIR lowering: `lower_expr_node` → `validate_break_continue` + `HirExpr::Continue`.
- Type-infer / MIR: mirror `ExprBreak`. MIR fn: `lower_continue`
  (`body_lower.rs:2918`).

### `ExprReturn`

- Surface: `return`, `return value`.
- CST: `return Expression?`.
- HIR lowering: `lower_expr_node` → `HirExpr::Return { value }`; a bare `return` in a failable/throwing
  initializer returns `.Some(())` / `.Ok(())` (`wrap_init_success_value`).
- Type-infer: `generate.rs:434-445` — coerces value (or unit) to `ctx.return_ty`; type
  is `Never`.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:599-605` — `Terminator::ret`, returns a sentinel unit immediate
  so downstream sees a value.
- Gotchas: bare `return` in a non-void function is a type mismatch (unit coerced to the
  return type — `generate.rs:440-442`).

### `ExprThrow`

- Surface: `throw err`.
- CST: `throw Expression?` (a missing value lowers to an error expression).
- HIR lowering: `desugar_throw` → `HirExpr::Return { value: Some(HirExpr::ImplicitMember { name: "Err", args: [value] }) }`;
  an erroneous value short-circuits to `HirExpr::Error`.
- Type-infer / MIR: follow the `Return` + `ImplicitMember` paths. `.Err(x)` resolves
  against the function's return type (which must provide an `Err` case).
- Gotchas: not yet routed through `Tryable` — always synthesizes `.Err(x)` regardless
  of whether the return type is a tuple-less result enum.

### `ExprTry`

- Surface: `try expr`.
- CST: `try Expression`.
- HIR lowering: `desugar_try` → `HirExpr::Sugar { kind: Try, inner: Match { source: TryOp } }` with arms:
  ```
  .Continue($try_value) => $try_value
  .Break($try_early)    => return .fromResidual(residual: $try_early)
  ```
  Scrutinee = `operand.tryExtract()` (ProtocolCall on `TryableProtocol`); without it,
  E128 + `Sugar(Error)`.
- Type-infer: via `HirExpr::Match` + the ProtocolCall scrutinee.
- Gotchas: `.fromResidual(...)` resolves as a protocol static on the function's return
  type (via `FromResidual`). If the return type doesn't conform, expect an
  ImplicitMemberNotFound at the return site.

### `ExprClosure`

- Surface: `{ x in x + 1 }`, `{ (a: Int, b: Int) in a + b }`, `{ x }`.
- CST: `{ (ClosureParams in)? BlockItem* }` — items sit directly in the node
  (`syntax::closure_body_syntax`: a statement-like expression that is not last is
  demoted to a statement, in source order).
- HIR lowering: `expr.rs` `lower_closure`. No parameter header + a reference to the
  *name* `it` in the body (`syntax::implicit_it_reference`, stopping at nested
  header-less closures) → implicit `it` parameter (E142/E143 via
  `warn_implicit_it_shadowing`). For each param: a plain binding or `_` → a normal
  `HirClosureParam`; any other pattern → synthetic local `_cparam_N` + a prepended
  `HirExpr::Match` with `source: ParamDestructure`. The loop-label stack is cleared
  for the body. Captures are computed post-inference (`ClosureCaptures`).
- Type-infer: `generate.rs:460` → `gen_closure` (later in `generate.rs`). Emits
  closure type as `FuncThick`; param types either from explicit annotations or fresh.
- Solver: closure params / body unify via `solve_equal` and `solve_coerce`.
- MIR: `body_lower.rs:758` → `lower_closure` at `body_lower.rs:2948`. Emits
  `Rvalue::ApplyPartial { func, captures }`.
- Gotchas: closures that capture can't currently be **returned** from fns — see the
  stdlib conventions list in `CLAUDE.md`. Complex param patterns are materialized as
  match-destructures with `MatchSource::ParamDestructure` so analyzers skip E111
  cascades (see `cascading_infer_errors.md`).

### `ExprMatch`

- Surface: `match x { .Some(y) => y, .None => 0 }`.
- CST: `match Expression { MatchArm (, MatchArm)* }`, `MatchArm > Pattern MatchArmGuard? => Expression`.
- HIR lowering: `expr.rs` `lower_match` → `HirExpr::Match { scrutinee, arms, source: MatchSource::UserMatch }`;
  arm patterns are the one place `&`/`&mutating` binders are legal (`ref_patterns_allowed`).
- Type-infer: `generate.rs:388-424` — `gen_pat(arm.pattern, scrut_tv, source)` per arm,
  `ctx.equal(body_tv, result_tv, arm.body.span)` per arm. Empty match poisons the
  result (prevents cascading "could not infer type"). Never-arms don't pin the
  result.
- Solver: `solve_equal`; patterns dispatch via `gen_pat`.
- MIR: `body_lower.rs:753-756` → `lower_match` at `body_lower.rs:3319`.
- Gotchas: check `MatchSource` before reporting exhaustiveness / unreachable arm
  diagnostics. `MatchSource::is_desugared()` (`body.rs:79`) returns true for anything
  except `UserMatch`. See `match_pattern_analyzer.md`.

### Match-arm block (`pat => { … }`)

- Surface: `{ stmt; stmt; tail }` as an expression (match-arm body, etc.).
- CST: an arm body that is a header-less `ExprClosure`.
- HIR lowering: `lower_match` treats it as a block, not a closure (no implicit `it`):
  `HirExpr::Block { body: lower_block(closure_body_syntax(..)), span }`.
- Type-infer: `generate.rs:539` → `gen_block` (elsewhere in `generate.rs`). If last
  stmt diverges (return/break/continue), block type = `Never`.
- MIR: `body_lower.rs:625` → `lower_hir_block`.
- Gotchas: `match-arm block` isn't used for normal fn bodies — those lower through
  `body_syntax` → `lower_block` directly. It's specifically for block-expression-in-expression
  positions.

### `ExprGrouping`

- Surface: `(expr)` (grouping, not a 1-tuple).
- CST: `( Expression )`.
- HIR lowering: `lower_expr_node` lowers the inner expression — no `HirExpr::Paren` exists; the
  grouping node maps to the inner expression's id in the source map. Precedence is
  already in the tree from the parser.
- Type-infer / MIR: N/A (unwrapped before reaching them).
- Gotchas: only HIR removes it.

### Malformed / missing expressions

- Surface: none — parse error recovery.
- CST: a missing child, an empty `Expression`, an `Error` node, an operator node
  without a recognised operator.
- HIR lowering: `ExprSrc::Error(span)` / the `_` arm of `lower_expr_node` → `HirExpr::Error { span }`
  (`malformed_expr_span` gives the span a demoted closure statement carries).
- Type-infer: `generate.rs:541` → `ctx.report_error(InferError::FromHir { span })`
  (poisons cleanly).
- MIR: `body_lower.rs:626` → `Immediate::error()`.
- Gotchas: HIR lowering **also** emits `HirExpr::Error` at many of its own sites (ambiguous
  resolution, undefined paths, etc.) — they don't all come from `malformed expression`. See
  the HirExpr entry.

---

## HirExpr variants (23)

Enum: `lib/kestrel-hir/src/body.rs:96` (header comment says "19 variants" — stale,
actually 23).

### HirExpr::Literal

- Produced by: `literal` (all kinds except `Unit`) via
  `lower_literal` (`expr.rs`). Also synthesized for if-let desugar bool arms
  (`expr.rs`), while-let / guard-let bool seeds (`expr.rs`), empty
  interpolated strings (`desugar.rs`).
- Type-infer: `generate.rs:63-70` (see literal entry above).
- MIR: `body_lower.rs:448` → `lower_literal_expr` → `Immediate`.

### HirExpr::Tuple

- Produced by: `ExprTuple` (`expr.rs`), **also** from
  `ExprUnit` at `expr.rs`, and as synthetic unit bodies in let-destructure
  and param-destructure `Match` arms (`stmt.rs`, `expr.rs`).
- Type-infer: `generate.rs:533-536` → `ctx.tuple`.
- MIR: `body_lower.rs:450-458` → `Rvalue::Tuple` into fresh temp.

### HirExpr::Array

- Produced by: `ExprArray` (`expr.rs`). 1:1.
- Type-infer: `generate.rs:463-487` (see ExprArray).
- MIR: `body_lower.rs:829-848` — tries custom array-literal init first via
  `lower_array_literal_via_init`, else `Rvalue::ArrayLiteral`.

### HirExpr::Dict

- Produced by: `ExprDictionary` (`expr.rs`). 1:1.
- Type-infer: `generate.rs:489-531`.
- MIR: `body_lower.rs:851-889` — lowered as an ArrayLiteral of (K, V) tuples.

### HirExpr::Closure

- Produced by: `ExprClosure` (`expr.rs`). Also synthesized to wrap the RHS of
  short-circuit binary ops (`&&`, `||`, `??`) as a parameterless closure
  (`desugar.rs`).
- Type-infer: `generate.rs:460` → `gen_closure`.
- MIR: `body_lower.rs:758` → `lower_closure` (2948). Emits `Rvalue::ApplyPartial`.

### HirExpr::Local(LocalId, Span)

- Produced by: `ExprPath` whose first segment resolves to a local
  (`expr.rs`). Also emitted as receivers for many desugared expressions
  (e.g., unwrap bind at `desugar.rs`; for-loop `$iter` ref at `desugar.rs`;
  try-expr `$try_value` / `$try_early` at `desugar.rs`; let-destructure
  `$let_tmp` ref at `stmt.rs`; closure-param destructure receiver at `expr.rs`).
- Type-infer: `generate.rs:73-87` — looks up `ctx.local_types[local_id]`. Reports
  `FromHir` error if unexpectedly missing.
- MIR: `body_lower.rs:449` → `Place::local(map_local(hir_local))`.

### HirExpr::Def(Entity, Vec\<HirTy\>, Span)

- Produced by: many sites. Primary:
  - `ExprPath` resolving to a single entity (`expr.rs`).
  - Multi-segment path with type-parameter first segment (`expr.rs`).
  - Static method call in `lower_call` (`expr.rs`, `expr.rs`).
  - `AssociatedTypeStaticMember` — embedded as base of a `Field` (`expr.rs`).
- Type-infer: `generate.rs:89-105` — `instantiate_entity_with_args`. Records
  `ctx.type_args` so MIR can retrieve resolved type args. Tracks `type_param_defs` so
  stray `Def(TypeParameter)` references (not consumed by a Call/MethodCall/Field)
  become a `TypeParamAsValue` error later.
- MIR: `body_lower.rs:629-719` — dispatches on `NodeKind`:
  - Function / Initializer: `Immediate::function_ref_generic` (or `Rvalue::ApplyPartial`
    if inference pinned to `FuncThick`).
  - EnumCase: `Rvalue::EnumVariant` with empty payload.
  - Struct: `Immediate::function_ref(init)` (default init if found).
  - Field (callable) → getter call; Field (stored) → `Place::Global`.
  - TypeParameter / TypeAlias → `Immediate::unit` (no runtime rep).
- Gotchas: `Def(TypeParameter)` is only valid when consumed by `Call`, `MethodCall`, or
  `Field` (static property access) — see the consumption sites that call
  `ctx.type_param_defs.remove(callee)` in `generate.rs:182, 194, 249, 308`.

### HirExpr::OverloadSet

- Produced by: `ExprPath` resolving to `ValueResolution::Overloaded`
  (`expr.rs`). Also static-method overload candidates
  (`expr.rs`).
- Type-infer: `generate.rs:108-115` — **error** if standalone (not consumed by a Call).
  In a Call: `generate.rs:120-132` dispatches to `ctx.overloaded_call(candidates, ...)`
  which emits `Constraint::OverloadedCall`.
- Solver: `solve_overloaded_call` (1379) picks by labels + arity, then by type.
- MIR: `body_lower.rs:721-733` — uses `typed.resolutions[expr_id]` to pick the winner,
  falls back to the first candidate if inference didn't resolve.

### HirExpr::Field

- Produced by:
  - Standalone `ExprPath (member access)` outside a Call (`expr.rs`).
  - Multi-segment `ExprPath` after a local (chained) (`expr.rs`).
  - Multi-segment `ExprPath` after a type-parameter (chained) (`expr.rs`).
  - `AssociatedTypeStaticMember` resolution (`expr.rs`).
- Type-infer: `generate.rs:293-314` — `Member` constraint with empty args, `is_call: false`.
  Consumes the `Def(TypeParameter)` base (`generate.rs:303-309`).
- Solver: `solve_member` (1702).
- MIR: `body_lower.rs:460-573` — dispatches by `is_callable` / `is_static` /
  `is_protocol_property`:
  - Static protocol property → witness call with no receiver.
  - Instance protocol property → witness call with receiver.
  - Static computed property → direct getter call.
  - Static stored field → `Place::Global`.
  - Computed property (instance) → getter call via `Callee::method`.
  - Stored field → `Place::field`.
- Gotchas: see `dispatch_funnel_pattern.md` — method/witness dispatch funnels through
  `emit_method_dispatch` in body_lower.

### HirExpr::TupleIndex

- Produced by: `ExprTupleIndex` (`expr.rs`). 1:1.
- Type-infer: `generate.rs:316-335` — `Member` with index as string name.
- MIR: `body_lower.rs:575-588` — `Place::index` or materialize-then-index.

### HirExpr::ImplicitMember

- Produced by:
  - `ExprImplicitMemberAccess` (`expr.rs`).
  - Synthesized in `desugar_try` for `.fromResidual` / `.Err` early-return
    (`desugar.rs`).
  - Synthesized in `desugar_throw` for `.Err(value)` (`desugar.rs`).
- Type-infer: `generate.rs:338-347` — `Constraint::Implicit { expected, name, args }`.
- Solver: `solve_implicit` (2182).
- MIR: `body_lower.rs:761-826` — if resolved entity is an EnumCase, emit
  `Rvalue::EnumVariant`; else if it's a protocol method, emit witness call; else direct
  static call.

### HirExpr::Call

- Produced by: `ExprCall` via `lower_call` (`expr.rs`). Also static method
  calls (see ExprCall entry 1, 2a, 2b, 2c subcases).
- Type-infer: `generate.rs:118-216` — see ExprCall. Branches based on callee
  (OverloadSet / Def(struct) / Def(enum_case) / Def(function) with mismatched labels /
  fallback Call constraint).
- Solver: `solve_call` (1251) for generic; `solve_overloaded_call` (1379) for
  overload-set callee.
- MIR: `body_lower.rs:736` → `lower_call` (`body_lower.rs:1867`).

### HirExpr::MethodCall

- Produced by: `ExprCall` with MemberAccess or multi-segment Path callee. Also
  string-interpolation `description()` calls (`desugar.rs`) and for-loop
  `iter()` / `next()` fallbacks when the protocol isn't resolvable
  (`desugar.rs`).
- Type-infer: `generate.rs:218-271` — `Member` constraint with `is_call: true`. Static
  context detected by receiver being `Def(struct/enum/protocol/type-alias/type-param)`;
  sets `is_static_context: true` so `solve_member` rejects instance-only methods.
  Explicit type args (`x.method[Int](...)`) routed through `ctx.member_with_type_args`.
- Solver: `solve_member` (1702).
- MIR: `body_lower.rs:737-743` → `lower_method_call` (`body_lower.rs:2121`).

### HirExpr::ProtocolCall

- Produced by (desugar-only):
  - `desugar_binary_hir` (`desugar.rs`) — all binary ops.
  - `desugar_logical_and` (`desugar.rs`) — if-condition ANDs.
  - `desugar_unary_op` (`desugar.rs`).
  - `desugar_compound_assign` (`desugar.rs`).
  - `desugar_while_let` negation (`desugar.rs`).
  - `desugar_for_loop` (`desugar.rs` iter, `desugar.rs` next).
  - `desugar_try` (`desugar.rs`).
  - `desugar_interpolated_string` add chain (`desugar.rs`).
- Type-infer: `generate.rs:273-290` — emits `Constraint::Conforms { ty: recv, protocol }`
  (recv must conform) AND `Constraint::Member { receiver, name, args, ... }`.
- Solver: `solve_conforms` (1076) + `solve_member` (1702).
- MIR: `body_lower.rs:744-751` → `lower_protocol_call` (`body_lower.rs:2280`). Dispatch
  through witness tables.
- Gotchas: the protocol Entity is pre-resolved via `ResolveBuiltin`; if it's missing,
  the desugar emits a diagnostic and falls through to `HirExpr::Error`. See
  `witness_instantiation_collapse.md` for monomorphization-side gotchas.

### HirExpr::If

- Produced by: `ExprIf` via `lower_if` (`expr.rs`). Also synthesized in
  `desugar_while` as `if cond {} else { break }` (`desugar.rs`), in
  `desugar_while_let` as `if !cond { break }` (`desugar.rs`), and in
  `lower_guard_let` as `if cond {} else { else_body }` (`stmt.rs`).
- Type-infer: `generate.rs:351-386` — no constraint on condition type (validated in a
  later analyzer pass, matches lib1). Equates branches via `ctx.equal`. Skips
  else-equate for guard-let If (must diverge).
- MIR: `body_lower.rs:590-595` → `lower_if` (2817).

### HirExpr::Loop

- Produced by: `ExprLoop` (`expr.rs`), desugared `ExprWhile`
  (`desugar.rs`), `ExprWhile (let)` (`desugar.rs`), and `ExprFor`
  (`desugar.rs`).
- Type-infer: `generate.rs:426-430` — result is `Never`.
- MIR: `body_lower.rs:596` → `lower_loop` (2875).

### HirExpr::Match

- Produced by: **nine** distinct sources via `MatchSource`:
  - `UserMatch` — `ExprMatch` (`expr.rs`).
  - `IfLet` — `ExprIf` with let-conditions (`expr.rs`).
  - `WhileLet` — `ExprWhile (let)` conditions (through `lower_if_conditions`
    at `expr.rs` with `source` override).
  - `GuardLet` — `GuardStatement` (`stmt.rs`).
  - `ForLoop` — `ExprFor` iterator match (`desugar.rs`).
  - `LetDestructure` — `VariableDeclaration` with complex pattern (`stmt.rs`).
  - `ParamDestructure` — closure param with complex pattern (`expr.rs`).
  - `TryOp` — `ExprTry` (`desugar.rs`).
- Type-infer: `generate.rs:388-424` — `gen_pat` on each arm's pattern with
  `scrutinee_tv` and `source`, equate arm bodies to result. Empty match poisons result.
- Solver: `solve_equal`; pattern constraints through `gen_pat`.
- MIR: `body_lower.rs:753-756` → `lower_match` (3319).
- Gotchas: analyzers check `MatchSource::is_desugared()` before running exhaustiveness
  or unreachable-arm checks. See `match_pattern_analyzer.md`.

### HirExpr::Break

- Produced by: `ExprBreak` (`expr.rs`). Also synthesized:
  - in `desugar_while` as the break exit (`desugar.rs`).
  - in `desugar_while_let` negation break (`desugar.rs`).
  - in `desugar_for_loop` for `.None => break` (`desugar.rs`).
- Type-infer: `generate.rs:432` — `Never`.
- MIR: `body_lower.rs:597` → `lower_break` (2909).

### HirExpr::Continue

- Produced by: `ExprContinue` only (`expr.rs`).
- Type-infer: `generate.rs:432` — `Never`.
- MIR: `body_lower.rs:598` → `lower_continue` (2918).

### HirExpr::Return

- Produced by: `ExprReturn` (`expr.rs`). Also synthesized in `desugar_try`
  for the `.Break($early) => return .fromResidual(...)` arm (`desugar.rs`) and
  `desugar_throw` (`desugar.rs`).
- Type-infer: `generate.rs:434-445` — coerce value (or unit) to return type; result
  type is `Never`.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:599-605` → `Terminator::ret`.

### HirExpr::Assign

- Produced by: `ExprAssignment` (`expr.rs`). 1:1.
- Type-infer: `generate.rs:448-457` — coerce value to target, result is unit.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:607-624` — setter dispatch first (`try_lower_setter_assign`), else
  direct `Place` write.

### HirExpr::Block

- Produced by: `match-arm block` (`expr.rs`). Also synthesized:
  - Complex let-destructure wrapper (`stmt.rs`) — wraps the temp `Let` and the
    destructuring `Match` in one expression.
  - `desugar_for_loop` wraps body + loop in a block (`desugar.rs`).
- Type-infer: `generate.rs:539` → `gen_block`.
- MIR: `body_lower.rs:625` → `lower_hir_block`.

### HirExpr::Error

- Produced by: `malformed expression` (`expr.rs`). Also emitted at many HIR-lowering
  error sites:
  - Empty path (`expr.rs`).
  - Type args on a local variable (`expr.rs`).
  - Empty type argument brackets (`expr.rs`).
  - Ambiguous name (`expr.rs`).
  - Undefined path (`expr.rs`).
  - Instance method called on a type (`expr.rs`).
  - Missing binary/unary/compound-assign operator protocol
    (`desugar.rs`).
  - Unwrap trap arm (`desugar.rs`).
  - Standalone rest pattern lowering (`pat.rs`) — but that's a pattern, not an
    expression.
- Type-infer: `generate.rs:541` — reports `InferError::FromHir`.
- MIR: `body_lower.rs:626` → `Immediate::error()`.
- Gotchas: `HirExpr::Error` is a concrete variant, not `null`/absence — analyzers and
  MIR must handle it, not assume it away.
