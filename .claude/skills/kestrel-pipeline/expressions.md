# Expressions — CST → AST → HIR → infer → MIR

Covers every variant of `AstExpr` (30) and `HirExpr` (23). Verify before citing — pipeline
maps go stale.

Top-level dispatch anchors:

- AST constructor switch: `lib/kestrel-ast-builder/src/lower.rs:307` (`lower_expr`)
- HIR lowering switch: `lib/kestrel-hir-lower/src/expr.rs:19` (`LowerCtx::lower_expr`)
- Inference gen switch: `lib/kestrel-type-infer/src/generate.rs:60` (`gen_expr`)
- Constraint dispatch: `lib/kestrel-type-infer/src/solver.rs:583` (`try_solve`)
- MIR lowering switch: `lib/kestrel-mir-lower/src/body_lower.rs:445` (`lower_expr`)

Solver functions (cite these when tracing a constraint):
`solve_equal` 817, `solve_coerce` 955, `solve_conforms` 1076, `solve_associated` 1099,
`solve_call` 1251, `solve_overloaded_call` 1379, `solve_member` 1702,
`solve_implicit` 2182, `solve_implicit_pat` 2293, `solve_tuple_rest_pat` 2401,
`solve_reduce` 716 (all in `solver.rs`).

---

## AstExpr variants (30)

Enum: `lib/kestrel-ast/src/ast_body.rs:42`.

### AstExpr::Literal

- Surface: `42`, `3.14`, `"s"`, `'c'`, `true`, `null`, `()`.
- CST: `ExprInteger`, `ExprFloat`, `ExprString`, `ExprRawString`, `ExprChar`, `ExprBool`,
  `ExprNull`, `ExprUnit` (dispatch: `lower.rs:309-338`).
- AST-builder: `lib/kestrel-ast-builder/src/lower.rs:410` (shared `lower_literal`);
  `Bool` at 320, `Null` at 327, `Unit` at 334.
- HIR lowering: `lib/kestrel-hir-lower/src/expr.rs:20` →
  `AstExpr::Literal { kind, span } => self.lower_literal(&kind, &span)`
  (impl `expr.rs:191`). **Unit literal is special** — `AstLiteral::Unit` becomes
  `HirExpr::Tuple { elements: vec![] }` (`expr.rs:208-213`), NOT `HirExpr::Literal`.
- Type-infer: `generate.rs:63-70` — each `HirLiteral` variant calls
  `ctx.fresh_literal(LiteralKind::X)`. Literal kinds defer to a default via
  `DefaultXLiteralType` builtin until coerced.
- Solver: `solve_equal` / `solve_coerce` pick the default when no other
  constraint pins the literal.
- MIR: `body_lower.rs:448` (`lower_literal_expr`) → `Immediate`.
- Gotchas: `()` never reaches the HIR as a Literal — it's always a zero-element Tuple.
  Integer overflow bug history: `MEMORY.md → integer_literal_overflow_silent_zero.md`.

### AstExpr::InterpolatedString

- Surface: `"hello \(name)!"`.
- CST: `ExprInterpolatedString` (`lower.rs:339`).
- AST-builder: `lower.rs:418` (`lower_interpolated_string`) — stores `Vec<StringPart>`.
- HIR lowering: `expr.rs:21` →
  `AstExpr::InterpolatedString { parts, span } => self.desugar_interpolated_string(body, &parts, &span)`.
  Implementation: `desugar.rs:697` — each `StringPart::Interpolation` becomes a
  `HirExpr::MethodCall { method: "description" }` on the operand; all parts chained
  left-associatively with `HirExpr::ProtocolCall { protocol: AddOperatorProtocol, method: "add" }`.
- Type-infer: receiver / arg for each chained `add` call — see `HirExpr::ProtocolCall`
  entry below.
- Solver: `solve_member` + `solve_conforms` (via ProtocolCall).
- MIR: falls through the `ProtocolCall` / `MethodCall` paths.
- Gotchas: parser currently emits the interpolated string as a single token;
  `desugar_interpolated_string` skips escape decoding on `StringPart::Literal` text
  (`desugar.rs:722-734`). When the parser splits interpolations, decode the structured parts.

### AstExpr::Array

- Surface: `[1, 2, 3]`.
- CST: `ExprArray` (`lower.rs:342`).
- AST-builder: `lower.rs:472` (`lower_array`).
- HIR lowering: `expr.rs:24-31` → `HirExpr::Array { elements, span }` (1:1).
- Type-infer: `generate.rs:463-487` — `fresh_literal(Array)` + `Associated(arr, "Element", elem)`
  + `Equal(elem, e_tv)` for each element. Bidirectional hint via `ctx.expected_array_elem`
  (set by the surrounding `let`-binding when the annotation is `Array[E]`).
- Solver: `solve_associated` (1099) resolves Element; `solve_equal` (817) for each element.
- MIR: `body_lower.rs:829-848` — prefers `lower_array_literal_via_init` (for custom
  `ExpressibleByArrayLiteral` types) before falling back to `Rvalue::ArrayLiteral`.
- Gotchas: element-type flow is bidirectional — see `cascading_infer_errors.md`.

### AstExpr::Dictionary

- Surface: `[k1: v1, k2: v2]` (or `[:]` for empty).
- CST: `ExprDictionary` (`lower.rs:343`).
- AST-builder: `lower.rs:478` (`lower_dictionary`).
- HIR lowering: `expr.rs:32-43` → `HirExpr::Dict { entries }`.
- Type-infer: `generate.rs:489-531` — `fresh_literal(Dictionary)` + `Associated("Key")`
  + `Associated("Value")` + `Equal(key_tv, k_tv)` / `Equal(val_tv, v_tv)` per entry.
  Bidirectional hint via `ctx.expected_dict_entry`. Uses
  `emit_dict_literal_acceptance_error` for per-key/value mismatch phrasing.
- Solver: `solve_associated` + `solve_equal`.
- MIR: `body_lower.rs:851-889` — lowered as an `ArrayLiteral` of `(K, V)` tuples.
- Gotchas: empty `[:]` still gets a default type via the literal default mechanism.

### AstExpr::Tuple

- Surface: `(a, b, c)`.
- CST: `ExprTuple` (`lower.rs:344`).
- AST-builder: `lower.rs:500` (`lower_tuple`).
- HIR lowering: `expr.rs:45-51` → `HirExpr::Tuple { elements }`.
- Type-infer: `generate.rs:533-536` — `ctx.tuple(elem_tvs)`.
- Solver: `solve_equal` (tuples unify structurally).
- MIR: `body_lower.rs:450-458` — `Rvalue::Tuple(values)` into a fresh temp.
- Gotchas: `AstExpr::Literal { kind: Unit }` becomes `HirExpr::Tuple { elements: [] }`,
  not a HIR literal — see `expr.rs:208-213`.

### AstExpr::Path

- Surface: `a`, `a.b.c`, `Pointer[UInt8]`, `Type.case`.
- CST: `ExprPath` (`lower.rs:365`).
- AST-builder: `lower.rs:518` (`lower_path`) → stores `Vec<ExprPathSegment>`; and
  `lower_member_access_chain` at 581 for `expr.field` chains whose base is not a path.
- HIR lowering: `expr.rs:222` (`lower_path`). Dispatches by first-segment kind and name
  resolution (`ResolveValuePath`). Decision tree:
  - Local variable (`lookup_local` hit, no type args) → `HirExpr::Local` + chain of
    `HirExpr::Field` for trailing segments (`expr.rs:236-247`).
  - Local with type args → error (`expr.rs:248-262`).
  - Type parameter with multi-segment → `HirExpr::Def(T)` + `HirExpr::Field` chain
    (`expr.rs:267-294`).
  - `ValueResolution::Def | TypeParameter` → `HirExpr::Def(entity, type_args, span)` (338).
  - `ValueResolution::Overloaded` → `HirExpr::OverloadSet { candidates, type_args }` (344).
  - `ValueResolution::EnumCaseValue` → `HirExpr::Def(entity)` (352).
  - `ValueResolution::FieldValue` → `HirExpr::Def(entity, vec![])` (357).
  - `ValueResolution::AssociatedType` → `HirExpr::Def(entity, vec![])` (360).
  - `ValueResolution::AssociatedTypeStaticMember` → `HirExpr::Field { base: Def(assoc), name }`
    for Self-substitution (`expr.rs:363-375`).
  - `Ambiguous` / `NotFound` → diagnostic + `HirExpr::Error`.
- Type-infer: per-variant — see `HirExpr::Local` / `::Def` / `::OverloadSet` / `::Field`.
- Gotchas: `Type.method()` parses as a **2-segment Path**, not a `MemberAccess` on a
  TypeRef — dispatched in `lower_call`, not `lower_path`. See `AstExpr::Call`.

### AstExpr::MemberAccess

- Surface: `expr.field`, `expr.method[T]` (with explicit type args).
- CST: Emitted by chains of `.member` through `lower_member_access_chain`
  (`lower.rs:581`).
- AST-builder: `lower.rs:627` (inside `lower_member_access_chain`, allocates the
  `AstExpr::MemberAccess` variant).
- HIR lowering: **Context-sensitive.**
  - Standalone (not in Call): `expr.rs:54-67` → `HirExpr::Field { base, name, span }`
    (type_args dropped — field/property access doesn't take method type args).
  - In a call: `AstExpr::Call { callee: MemberAccess, ... }` at `expr.rs:479-533` →
    `HirExpr::MethodCall { receiver, method, type_args, args }` OR, if base is a type
    path with a matching static method, `HirExpr::Call { callee: Def(static_fn) }` via
    `try_resolve_static_call` (`expr.rs:494-519`).
    ```
    if let Some((static_candidates, base_type_args)) =
        self.try_resolve_static_call(body, base, &member) { ... HirExpr::Def | OverloadSet + Call }
    else { HirExpr::MethodCall { ... } }
    ```
- Type-infer: see `HirExpr::Field` or `HirExpr::MethodCall`.
- Gotchas: explicit type args (`x.method[Int]`) are only meaningful for method calls;
  the `type_args` on a standalone `MemberAccess` are dropped at HIR lowering (`expr.rs:57`).

### AstExpr::TupleIndex

- Surface: `pair.0`, `triple.2`.
- CST: `ExprTupleIndex` (`lower.rs:366`).
- AST-builder: `lower.rs:645` (`lower_tuple_index`).
- HIR lowering: `expr.rs:68-75` → `HirExpr::TupleIndex { base, index, span }`.
- Type-infer: `generate.rs:316-335` — emits a `Member` constraint with the index as
  `name` (string form). Solver's `solve_member` recognizes numeric names for tuples.
- Solver: `solve_member` (1702).
- MIR: `body_lower.rs:575-588` — `Place::index` if base is a `Place`, else materialize
  to temp then index.

### AstExpr::ImplicitMember

- Surface: `.Some(x)`, `.None`, `.fromResidual(...)`.
- CST: `ExprImplicitMemberAccess` (`lower.rs:367`).
- AST-builder: `lower.rs:690` (`lower_implicit_member`). Parses both `.name` and
  `.name(args)`.
- HIR lowering: `expr.rs:76-87` → `HirExpr::ImplicitMember { name, args, span }`.
- Type-infer: `generate.rs:338-347` — emits `Constraint::Implicit { expected, name, args }`
  (expected = fresh result TyVar — must be pinned by surrounding context).
- Solver: `solve_implicit` (2182).
- MIR: `body_lower.rs:761-826` — dispatches on resolved entity:
  - EnumCase → `Rvalue::EnumVariant`.
  - Protocol static method → `Callee::witness`.
  - Regular static → `Callee::direct_generic`.
- Gotchas: `.fromResidual(residual: early)` is emitted by try-expr desugaring
  (`desugar.rs:566`); it's a protocol static method, not an enum case.

### AstExpr::Unary

- Surface: `-x`, `not b`, `!b` (bitwise), `+x`.
- CST: `ExprUnary` (`lower.rs:370`).
- AST-builder: `lower.rs:717` (`lower_unary`).
- HIR lowering: `expr.rs:88-90` → `desugar_unary_op` (`desugar.rs:116`). Emits
  `HirExpr::ProtocolCall { protocol: <unary proto>, method: "..." }`. **`UnaryOp::Pos`
  is identity** — `desugar_unary_op` returns `self.lower_expr(operand)` (`desugar.rs:124`).
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: desugar-only — there is no `HirExpr::Unary`.

### AstExpr::Postfix

- Surface: `x!` (unwrap).
- CST: `ExprPostfix` (`lower.rs:371`).
- AST-builder: `lower.rs:730` (`lower_postfix`). `PostfixOp` currently has only
  `Unwrap`.
- HIR lowering: `expr.rs:91-93` →
  `AstExpr::Postfix { operand, op: Unwrap, span } => self.desugar_unwrap(body, operand, &span)`.
  Implementation: `desugar.rs:640` — produces `HirExpr::Match` with
  `source: UnwrapOp`, arms `.Some($v) => $v` and `.None => Error` (trap placeholder).
- Type-infer: see `HirExpr::Match`.
- Gotchas: the `None` arm body is currently `HirExpr::Error` as a trap placeholder
  (`desugar.rs:670`) — MIR treats it as `Immediate::error()`. See `funcref_to_functhick_coercion.md`
  for unwrap-related unkillable zombie tests.

### AstExpr::Binary

- Surface: `a + b`, `a == b`, `a && b`, `a .. b`, `a ?? b`.
- CST: `ExprBinary` (`lower.rs:372`).
- AST-builder: `lower.rs:762` (`lower_binary`).
- HIR lowering: `expr.rs:94` → `lower_binary_with_precedence` (`expr.rs:955`) flattens
  nested `AstExpr::Binary` into operand / operator lists then Pratt-parses with
  `pratt_parse`. Each reduction calls `desugar_binary_hir` (`desugar.rs:21`):
  - Short-circuit ops (`&&`, `||`, `??`) wrap RHS in a parameterless `HirExpr::Closure`
    then emit `HirExpr::ProtocolCall` (`desugar.rs:29-54`).
  - Regular ops emit `HirExpr::ProtocolCall` directly (`desugar.rs:57-71`).
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: all operators route through protocols — if a builtin protocol isn't
  resolvable, `desugar_binary_hir` emits `HirExpr::Error` and a diagnostic ("is the
  standard library imported?") `desugar.rs:73`.

### AstExpr::Assignment

- Surface: `x = y`, `obj.prop = v`, `Foo.staticVar = v`.
- CST: `ExprAssignment` (`lower.rs:373`).
- AST-builder: `lower.rs:780` (`lower_assignment`).
- HIR lowering: `expr.rs:95-103` → `HirExpr::Assign { target, value, span }` (direct).
- Type-infer: `generate.rs:448-457` — `coerce(value_tv, target_tv)`, result is unit.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:607-624`. **Setter detour**: `try_lower_setter_assign` at 612
  dispatches computed-property assigns (`obj.computed = v`, `Foo.staticComputed = v`,
  global computed vars) through a Setter child entity instead of emitting a stored-Place
  write. Falls back to `StatementKind::Assign` with a `Place` destination.
- Gotchas: `self.field = v` inside an initializer is a stored-field write, not a
  settable check. Mutability on the LHS comes from the binding, not the expression.

### AstExpr::CompoundAssignment

- Surface: `x += y`, `x *= y`, `x <<=`, etc.
- CST: `ExprCompoundAssignment` (`lower.rs:374`).
- AST-builder: `lower.rs:805` (`lower_compound_assignment`).
- HIR lowering: `expr.rs:104-106` → `desugar_compound_assign` (`desugar.rs:158`) emits
  `HirExpr::ProtocolCall { protocol: <compound-assign proto>, method, receiver: lhs,
  args: [rhs] }`. **Not** `HirExpr::Assign`.
- Type-infer: see `HirExpr::ProtocolCall`.
- Gotchas: `x += y` does NOT lower as `Assign(x, ProtocolCall(x, add, y))` — it's a
  single `ProtocolCall` on a compound-assign protocol whose method mutates the receiver
  in-place. Catch this when reviewing analyzers that look for `Assign` targets.

### AstExpr::Call

- Surface: `foo(x)`, `obj.method(x)`, `Type.staticMethod(x)`, `dict(key)` (subscript).
- CST: `ExprCall` (`lower.rs:377`).
- AST-builder: `lower.rs:825` (`lower_call`).
- HIR lowering: `expr.rs:107-111` → `lower_call` at `expr.rs:469`. Decision tree on the
  callee:
  1. `AstExpr::MemberAccess` (`expr.rs:480-533`) — try static resolution first
     (`try_resolve_static_call` 872). If hit, emit `HirExpr::Call { callee: Def|OverloadSet }`.
     Otherwise emit `HirExpr::MethodCall { receiver, method, type_args, args }`.
  2. `AstExpr::Path` with `>= 2` segments (`expr.rs:537-725`):
     - first segment is a local (`expr.rs:538-560`) → `HirExpr::MethodCall { receiver:
       build Field chain, method: last seg, ... }`.
     - static method via `try_resolve_static_call_from_segments` (800) →
       `HirExpr::Call { callee: Def|OverloadSet }`.
     - instance method on a type (no receiver) → diagnostic + `HirExpr::Error`
       (`expr.rs:596-609`).
     - type parameter static → `HirExpr::MethodCall { receiver: Def(TypeParam) }` (640).
     - value-prefix chain (computed getter → instance method) → `HirExpr::MethodCall`
       (`expr.rs:668-715`).
     - otherwise fall through to regular `HirExpr::Call { callee: lowered_path, args }`.
  3. Any other callee → `HirExpr::Call { callee: lowered, args }` (`expr.rs:727-735`).
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
- Gotchas: `Type.method()` (multi-segment path, no explicit local prefix) is `Path` at
  the AST level. Dispatch happens in `lower_call`, not `lower_path`. See also
  `static_overload_first_match_truncation.md` in MEMORY.

### AstExpr::If

- Surface: `if c { ... }`, `if let p = v { ... } else if ... { ... } else { ... }`.
- CST: `ExprIf` (`lower.rs:380`).
- AST-builder: `lower.rs:918` (`lower_if`). Stores `Vec<IfCondition>` (mixed let/expr
  conditions) + optional `ElseBody::{Block, ElseIf}`.
- HIR lowering: `expr.rs:112-117` → `lower_if` at `expr.rs:1037`. Conditions routed
  through `lower_if_conditions` (1076) with `MatchSource::IfLet`:
  - Single `IfCondition::Expr` → just lower the expression.
  - Single `IfCondition::Let` → `HirExpr::Match` with `source: IfLet`, arms
    `pattern => true` and `_ => false` (`expr.rs:1094-1125`).
  - Multiple conditions → pair-wise `desugar_logical_and` (ProtocolCall on
    `LogicalAndOperatorProtocol`).
  Then `HirExpr::If { condition, then_body, else_body }` at `expr.rs:1063`.
- Type-infer: `generate.rs:351-386`. Generates block types for both branches and
  `ctx.equal(then_tv, result)` / `ctx.equal(else_tv, result)` — but **skips the
  else-equate when the If came from a guard-let desugaring** (`is_guard_let_if` helper)
  since the else block is required to diverge. No else → result type is unit.
- Solver: `solve_equal`.
- MIR: `body_lower.rs:590-595` → `lower_if` at `body_lower.rs:2817`.
- Gotchas: an `if`-expression with no `else` has unit result even if the `then` block
  has a typed tail — this comes from `generate.rs:382-385`.

### AstExpr::While

- Surface: `while cond { body }`, `'label: while cond { body }`.
- CST: `ExprWhile` (`lower.rs:381`).
- AST-builder: `lower.rs:1019` (`lower_while`). Emits `AstExpr::While` for plain
  while, or `AstExpr::WhileLet` when conditions contain a `let` (detected at 993).
- HIR lowering: `expr.rs:118-123` → `desugar_while` at `desugar.rs:204`. Emits:
  ```
  HirExpr::Loop {
      stmts: [ HirStmt::Expr { HirExpr::If { cond, then: {}, else: Some({ break }) } } ] ++ body.stmts,
      tail_expr: body.tail_expr,
  }
  ```
  — the condition also gets pushed into `ctx.while_conditions` for the condition-type analyzer.
- Type-infer: see `HirExpr::Loop`.
- Gotchas: `while` desugars to `if cond {} else { break }` (positive test) instead of
  `if !cond { break }` — avoids requiring the condition type to conform to `Not`
  (`desugar.rs:199-202`).

### AstExpr::WhileLet

- Surface: `while let .Some(x) = iter.next() { ... }`.
- CST: Same `ExprWhile` node with a let-condition (AST-builder distinguishes at
  `lower.rs:993`).
- AST-builder: emits `AstExpr::WhileLet` at `lower.rs:993`.
- HIR lowering: `expr.rs:124-129` → `desugar_while_let` at `desugar.rs:259`. Scopes
  the let-bindings to the condition + body via `push_scope`. Calls
  `lower_if_conditions` with `MatchSource::WhileLet` to build a bool scrutinee. Then
  wraps: `if !cond { break }` + body inside `HirExpr::Loop`.
- Type-infer: see `HirExpr::Loop` + `HirExpr::Match` (the desugared let-condition
  becomes a match with `source: WhileLet`).
- Gotchas: bindings from the let-condition live until the loop ends — the scope push is
  around the condition AND body (`desugar.rs:268-311`).

### AstExpr::Loop

- Surface: `loop { ... }`, `'label: loop { ... }`.
- CST: `ExprLoop` (`lower.rs:383`).
- AST-builder: `lower.rs:1081` (`lower_loop`).
- HIR lowering: `expr.rs:130-143` — pushes loop label via `push_loop`, lowers body,
  emits `HirExpr::Loop { label, body }` directly.
- Type-infer: `generate.rs:426-430` — result type is `Never` unless a `break` with a
  value pins it (the break-value analysis happens through the break's flow into the
  block type).
- Solver: N/A (Never/concrete types resolve directly).
- MIR: `body_lower.rs:596` → `lower_loop` at `body_lower.rs:2875`.
- Gotchas: `loop { break 5 }` as an expression relies on the break-value path; plain
  `loop {}` is `!` (Never).

### AstExpr::For

- Surface: `for x in iter { ... }`, `for (a, b) in pairs { ... }`.
- CST: `ExprFor` (`lower.rs:382`).
- AST-builder: `lower.rs:1059` (`lower_for`).
- HIR lowering: `expr.rs:144-150` → `desugar_for_loop` at `desugar.rs:338`. Emits a
  `HirExpr::Block` wrapping:
  ```
  let $iter = iterable.iter()      // ProtocolCall on Iterable
  loop {
      match $iter.next() {         // ProtocolCall on Iterator
          .Some(pattern) => { body }
          .None => break
      }
  }
  ```
  — `match` uses `MatchSource::ForLoop` (`desugar.rs:448`).
- Type-infer: see `HirExpr::Loop`, `HirExpr::Match`, `HirExpr::ProtocolCall`.
- Gotchas: the match is `source: ForLoop` — analyzers should skip exhaustiveness checks
  on it (see `match_pattern_analyzer.md`).

### AstExpr::Break

- Surface: `break`, `'label: loop { break 'label }`.
- CST: `ExprBreak` (`lower.rs:384`).
- AST-builder: `lower.rs:1087` (`lower_break`).
- HIR lowering: `expr.rs:151-154` → `validate_break_continue` (`expr.rs:1278` — checks
  `in_loop` and optional label scope) then `HirExpr::Break { label, span }`.
- Type-infer: `generate.rs:432` — type is `Never`.
- MIR: `body_lower.rs:597` → `lower_break` at `body_lower.rs:2909`.
- Gotchas: `break` with a value is not yet represented — current AST is `label`-only
  (no `value: Option<ExprId>`).

### AstExpr::Continue

- Surface: `continue`, `'label: loop { continue 'label }`.
- CST: `ExprContinue` (`lower.rs:385`).
- AST-builder: `lower.rs:1093` (`lower_continue`).
- HIR lowering: `expr.rs:155-158` → `validate_break_continue` + `HirExpr::Continue`.
- Type-infer / MIR: mirror `AstExpr::Break`. MIR fn: `lower_continue`
  (`body_lower.rs:2918`).

### AstExpr::Return

- Surface: `return`, `return value`.
- CST: `ExprReturn` (`lower.rs:386`).
- AST-builder: `lower.rs:1102` (`lower_return`).
- HIR lowering: `expr.rs:159-165` → `HirExpr::Return { value: Option<HirExprId> }`.
- Type-infer: `generate.rs:434-445` — coerces value (or unit) to `ctx.return_ty`; type
  is `Never`.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:599-605` — `Terminator::ret`, returns a sentinel unit immediate
  so downstream sees a value.
- Gotchas: bare `return` in a non-void function is a type mismatch (unit coerced to the
  return type — `generate.rs:440-442`).

### AstExpr::Throw

- Surface: `throw err`.
- CST: `ExprThrow` (`lower.rs:387`).
- AST-builder: `lower.rs:1112` (`lower_throw`).
- HIR lowering: `expr.rs:166` → `desugar_throw` at `desugar.rs:610`. Emits
  `HirExpr::Return { value: Some(HirExpr::ImplicitMember { name: "Err", args: [value] }) }`.
- Type-infer / MIR: follow the `Return` + `ImplicitMember` paths. `.Err(x)` resolves
  against the function's return type (which must provide an `Err` case).
- Gotchas: not yet routed through `Tryable` — always synthesizes `.Err(x)` regardless
  of whether the return type is a tuple-less result enum.

### AstExpr::Try

- Surface: `try expr`.
- CST: `ExprTry` (`lower.rs:388`).
- AST-builder: `lower.rs:1122` (`lower_try`).
- HIR lowering: `expr.rs:167` → `desugar_try` at `desugar.rs:499`. Emits
  `HirExpr::Match { source: TryOp }` with arms:
  ```
  .Continue($value) => $value
  .Break($early)    => return .fromResidual(residual: $early)
  ```
  Scrutinee = `operand.tryExtract()` (ProtocolCall on `TryableProtocol`). If
  `TryableProtocol` isn't resolvable, falls back to `.Ok` / `.Err` on the raw operand
  (`desugar.rs:517-528`).
- Type-infer: via `HirExpr::Match` + the ProtocolCall scrutinee.
- Gotchas: `.fromResidual(...)` resolves as a protocol static on the function's return
  type (via `FromResidual`). If the return type doesn't conform, expect an
  ImplicitMemberNotFound at the return site.

### AstExpr::Closure

- Surface: `{ x in x + 1 }`, `{ (a: Int, b: Int) in a + b }`, `{ x }`.
- CST: `ExprClosure` (`lower.rs:391`).
- AST-builder: `lower.rs:1196` (`lower_closure`).
- HIR lowering: `expr.rs:168-172` → `lower_closure` at `expr.rs:1139`. For each param:
  - `AstPat::Binding` or `AstPat::Wildcard` → a normal `HirClosureParam` with no
    pattern.
  - Any other pattern → synthetic local `_cparam_N` + a prepended `HirExpr::Match`
    with `source: ParamDestructure` in the closure body (`expr.rs:1171-1194`).
  Captures are collected by walking the body (`collect_captures`).
- Type-infer: `generate.rs:460` → `gen_closure` (later in `generate.rs`). Emits
  closure type as `FuncThick`; param types either from explicit annotations or fresh.
- Solver: closure params / body unify via `solve_equal` and `solve_coerce`.
- MIR: `body_lower.rs:758` → `lower_closure` at `body_lower.rs:2948`. Emits
  `Rvalue::ApplyPartial { func, captures }`.
- Gotchas: closures that capture can't currently be **returned** from fns — see the
  stdlib conventions list in `CLAUDE.md`. Complex param patterns are materialized as
  match-destructures with `MatchSource::ParamDestructure` so analyzers skip E111
  cascades (see `cascading_infer_errors.md`).

### AstExpr::Match

- Surface: `match x { .Some(y) => y, .None => 0 }`.
- CST: `ExprMatch` (`lower.rs:392`).
- AST-builder: `lower.rs:1254` (`lower_match`).
- HIR lowering: `expr.rs:173-177` → `lower_match` at `expr.rs:1226` →
  `HirExpr::Match { scrutinee, arms, source: MatchSource::UserMatch }`.
- Type-infer: `generate.rs:388-424` — `gen_pat(arm.pattern, scrut_tv, source)` per arm,
  `ctx.equal(body_tv, result_tv, arm.body.span)` per arm. Empty match poisons the
  result (prevents cascading "could not infer type"). Never-arms don't pin the
  result.
- Solver: `solve_equal`; patterns dispatch via `gen_pat`.
- MIR: `body_lower.rs:753-756` → `lower_match` at `body_lower.rs:3319`.
- Gotchas: check `MatchSource` before reporting exhaustiveness / unreachable arm
  diagnostics. `MatchSource::is_desugared()` (`body.rs:79`) returns true for anything
  except `UserMatch`. See `match_pattern_analyzer.md`.

### AstExpr::Block

- Surface: `{ stmt; stmt; tail }` as an expression (match-arm body, etc.).
- CST: `CodeBlock` (when parser treats an arm body as a closure, see
  `lib/kestrel-ast-builder/AGENTS.md` and MEMORY entry for the 2026-03-07 fix).
- AST-builder: `lower.rs:1136` (inside `lower_match_arm` / the closure→block promotion).
  Allocated when an arm body is a parameterless closure with statements.
- HIR lowering: `expr.rs:178-184` → `HirExpr::Block { body, span }` (1:1).
- Type-infer: `generate.rs:539` → `gen_block` (elsewhere in `generate.rs`). If last
  stmt diverges (return/break/continue), block type = `Never`.
- MIR: `body_lower.rs:625` → `lower_hir_block`.
- Gotchas: `AstExpr::Block` isn't used for normal fn bodies — those are `AstBody` with
  `statements + tail_expr` directly. It's specifically for block-expression-in-expression
  positions.

### AstExpr::Paren

- Surface: `(expr)` (grouping, not a 1-tuple).
- CST: `ExprGrouping` (`lower.rs:347`).
- AST-builder: `lower.rs:355`.
- HIR lowering: `expr.rs:185` → `AstExpr::Paren { inner, .. } => self.lower_expr(body, inner)`.
  Unwrapped — no `HirExpr::Paren` exists.
- Rationale: preserved at AST level so the Pratt parser (`lower_binary_with_precedence`
  at `expr.rs:955`) doesn't merge across user-written grouping.
- Type-infer / MIR: N/A (unwrapped before reaching them).
- Gotchas: don't flatten at AST level — `AstExpr::Paren` is load-bearing for
  precedence. Only HIR removes it.

### AstExpr::Error

- Surface: none — parse error recovery.
- CST: any unrecognized expression kind (`lower.rs:394` fallback) or in-flight parse
  errors.
- AST-builder: dozens of sites emit `AstExpr::Error { span }` — see the constructor
  hits at `lower.rs:178, 249, 259, 360, 396, 440, 653, 715, 727, 748, 760, 774, 778,
  792, 803, 818, 846, 1008, 1048, 1111, 1121, 1245, 1279, 1307, 1773`.
- HIR lowering: `expr.rs:186` → `HirExpr::Error { span }`.
- Type-infer: `generate.rs:541` → `ctx.report_error(InferError::FromHir { span })`
  (poisons cleanly).
- MIR: `body_lower.rs:626` → `Immediate::error()`.
- Gotchas: HIR lowering **also** emits `HirExpr::Error` at many of its own sites (ambiguous
  resolution, undefined paths, etc.) — they don't all come from `AstExpr::Error`. See
  the HirExpr entry.

---

## HirExpr variants (23)

Enum: `lib/kestrel-hir/src/body.rs:96` (header comment says "19 variants" — stale,
actually 23).

### HirExpr::Literal

- Produced by: `AstExpr::Literal` (all kinds except `Unit`) via
  `lower_literal` (`expr.rs:191`). Also synthesized for if-let desugar bool arms
  (`expr.rs:1099,1103`), while-let / guard-let bool seeds (`expr.rs:1085`), empty
  interpolated strings (`desugar.rs:704`).
- Type-infer: `generate.rs:63-70` (see AstExpr::Literal entry above).
- MIR: `body_lower.rs:448` → `lower_literal_expr` → `Immediate`.

### HirExpr::Tuple

- Produced by: `AstExpr::Tuple` (`expr.rs:45-51`), **also** from
  `AstLiteral::Unit` at `expr.rs:208-213`, and as synthetic unit bodies in let-destructure
  and param-destructure `Match` arms (`stmt.rs:106-109`, `expr.rs:1175-1178`).
- Type-infer: `generate.rs:533-536` → `ctx.tuple`.
- MIR: `body_lower.rs:450-458` → `Rvalue::Tuple` into fresh temp.

### HirExpr::Array

- Produced by: `AstExpr::Array` (`expr.rs:24-31`). 1:1.
- Type-infer: `generate.rs:463-487` (see AstExpr::Array).
- MIR: `body_lower.rs:829-848` — tries custom array-literal init first via
  `lower_array_literal_via_init`, else `Rvalue::ArrayLiteral`.

### HirExpr::Dict

- Produced by: `AstExpr::Dictionary` (`expr.rs:32-43`). 1:1.
- Type-infer: `generate.rs:489-531`.
- MIR: `body_lower.rs:851-889` — lowered as an ArrayLiteral of (K, V) tuples.

### HirExpr::Closure

- Produced by: `AstExpr::Closure` (`expr.rs:1139`). Also synthesized to wrap the RHS of
  short-circuit binary ops (`&&`, `||`, `??`) as a parameterless closure
  (`desugar.rs:30-38, 85-93`).
- Type-infer: `generate.rs:460` → `gen_closure`.
- MIR: `body_lower.rs:758` → `lower_closure` (2948). Emits `Rvalue::ApplyPartial`.

### HirExpr::Local(LocalId, Span)

- Produced by: `AstExpr::Path` whose first segment resolves to a local
  (`expr.rs:236-247`). Also emitted as receivers for many desugared expressions
  (e.g., unwrap bind at `desugar.rs:544, 662`; for-loop `$iter` ref at `desugar.rs:378`;
  try-expr `$try_value` / `$try_early` at `desugar.rs:544, 561`; let-destructure
  `$let_tmp` ref at `stmt.rs:105`; closure-param destructure receiver at `expr.rs:1174`).
- Type-infer: `generate.rs:73-87` — looks up `ctx.local_types[local_id]`. Reports
  `FromHir` error if unexpectedly missing.
- MIR: `body_lower.rs:449` → `Place::local(map_local(hir_local))`.

### HirExpr::Def(Entity, Vec\<HirTy\>, Span)

- Produced by: many sites. Primary:
  - `AstExpr::Path` resolving to a single entity (`expr.rs:332,346,351,354`).
  - Multi-segment path with type-parameter first segment (`expr.rs:280-284`).
  - Static method call in `lower_call` (`expr.rs:501-506, 572-577`, `expr.rs:647-651`).
  - `AssociatedTypeStaticMember` — embedded as base of a `Field` (`expr.rs:370`).
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

- Produced by: `AstExpr::Path` resolving to `ValueResolution::Overloaded`
  (`expr.rs:338-344`). Also static-method overload candidates
  (`expr.rs:508-513, 578-584`).
- Type-infer: `generate.rs:108-115` — **error** if standalone (not consumed by a Call).
  In a Call: `generate.rs:120-132` dispatches to `ctx.overloaded_call(candidates, ...)`
  which emits `Constraint::OverloadedCall`.
- Solver: `solve_overloaded_call` (1379) picks by labels + arity, then by type.
- MIR: `body_lower.rs:721-733` — uses `typed.resolutions[expr_id]` to pick the winner,
  falls back to the first candidate if inference didn't resolve.

### HirExpr::Field

- Produced by:
  - Standalone `AstExpr::MemberAccess` outside a Call (`expr.rs:54-66`).
  - Multi-segment `AstExpr::Path` after a local (chained) (`expr.rs:239-244`).
  - Multi-segment `AstExpr::Path` after a type-parameter (chained) (`expr.rs:286-291`).
  - `AssociatedTypeStaticMember` resolution (`expr.rs:370-374`).
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

- Produced by: `AstExpr::TupleIndex` (`expr.rs:68-75`). 1:1.
- Type-infer: `generate.rs:316-335` — `Member` with index as string name.
- MIR: `body_lower.rs:575-588` — `Place::index` or materialize-then-index.

### HirExpr::ImplicitMember

- Produced by:
  - `AstExpr::ImplicitMember` (`expr.rs:76-87`).
  - Synthesized in `desugar_try` for `.fromResidual` / `.Err` early-return
    (`desugar.rs:566-584`).
  - Synthesized in `desugar_throw` for `.Err(value)` (`desugar.rs:618-625`).
- Type-infer: `generate.rs:338-347` — `Constraint::Implicit { expected, name, args }`.
- Solver: `solve_implicit` (2182).
- MIR: `body_lower.rs:761-826` — if resolved entity is an EnumCase, emit
  `Rvalue::EnumVariant`; else if it's a protocol method, emit witness call; else direct
  static call.

### HirExpr::Call

- Produced by: `AstExpr::Call` via `lower_call` (`expr.rs:469`). Also static method
  calls (see AstExpr::Call entry 1, 2a, 2b, 2c subcases).
- Type-infer: `generate.rs:118-216` — see AstExpr::Call. Branches based on callee
  (OverloadSet / Def(struct) / Def(enum_case) / Def(function) with mismatched labels /
  fallback Call constraint).
- Solver: `solve_call` (1251) for generic; `solve_overloaded_call` (1379) for
  overload-set callee.
- MIR: `body_lower.rs:736` → `lower_call` (`body_lower.rs:1867`).

### HirExpr::MethodCall

- Produced by: `AstExpr::Call` with MemberAccess or multi-segment Path callee. Also
  string-interpolation `description()` calls (`desugar.rs:740-746`) and for-loop
  `iter()` / `next()` fallbacks when the protocol isn't resolvable
  (`desugar.rs:360, 389`).
- Type-infer: `generate.rs:218-271` — `Member` constraint with `is_call: true`. Static
  context detected by receiver being `Def(struct/enum/protocol/type-alias/type-param)`;
  sets `is_static_context: true` so `solve_member` rejects instance-only methods.
  Explicit type args (`x.method[Int](...)`) routed through `ctx.member_with_type_args`.
- Solver: `solve_member` (1702).
- MIR: `body_lower.rs:737-743` → `lower_method_call` (`body_lower.rs:2121`).

### HirExpr::ProtocolCall

- Produced by (desugar-only):
  - `desugar_binary_hir` (`desugar.rs:40-50, 59-70`) — all binary ops.
  - `desugar_logical_and` (`desugar.rs:96-106`) — if-condition ANDs.
  - `desugar_unary_op` (`desugar.rs:133-141`).
  - `desugar_compound_assign` (`desugar.rs:170-181`).
  - `desugar_while_let` negation (`desugar.rs:280-288`).
  - `desugar_for_loop` (`desugar.rs:351-358` iter, `desugar.rs:380-387` next).
  - `desugar_try` (`desugar.rs:509-516`).
  - `desugar_interpolated_string` add chain (`desugar.rs:766-777`).
- Type-infer: `generate.rs:273-290` — emits `Constraint::Conforms { ty: recv, protocol }`
  (recv must conform) AND `Constraint::Member { receiver, name, args, ... }`.
- Solver: `solve_conforms` (1076) + `solve_member` (1702).
- MIR: `body_lower.rs:744-751` → `lower_protocol_call` (`body_lower.rs:2280`). Dispatch
  through witness tables.
- Gotchas: the protocol Entity is pre-resolved via `ResolveBuiltin`; if it's missing,
  the desugar emits a diagnostic and falls through to `HirExpr::Error`. See
  `witness_instantiation_collapse.md` for monomorphization-side gotchas.

### HirExpr::If

- Produced by: `AstExpr::If` via `lower_if` (`expr.rs:1037`). Also synthesized in
  `desugar_while` as `if cond {} else { break }` (`desugar.rs:221-232`), in
  `desugar_while_let` as `if !cond { break }` (`desugar.rs:293-301`), and in
  `lower_guard_let` as `if cond {} else { else_body }` (`stmt.rs:164-172`).
- Type-infer: `generate.rs:351-386` — no constraint on condition type (validated in a
  later analyzer pass, matches lib1). Equates branches via `ctx.equal`. Skips
  else-equate for guard-let If (must diverge).
- MIR: `body_lower.rs:590-595` → `lower_if` (2817).

### HirExpr::Loop

- Produced by: `AstExpr::Loop` (`expr.rs:130-143`), desugared `AstExpr::While`
  (`desugar.rs:248-255`), `AstExpr::WhileLet` (`desugar.rs:316-323`), and `AstExpr::For`
  (`desugar.rs:458-465`).
- Type-infer: `generate.rs:426-430` — result is `Never`.
- MIR: `body_lower.rs:596` → `lower_loop` (2875).

### HirExpr::Match

- Produced by: **nine** distinct sources via `MatchSource`:
  - `UserMatch` — `AstExpr::Match` (`expr.rs:1252`).
  - `IfLet` — `AstExpr::If` with let-conditions (`expr.rs:1109-1125`).
  - `WhileLet` — `AstExpr::WhileLet` conditions (through `lower_if_conditions`
    at `expr.rs:1109-1125` with `source` override).
  - `GuardLet` — `AstStmt::GuardLet` (`stmt.rs:155`).
  - `ForLoop` — `AstExpr::For` iterator match (`desugar.rs:434-450`).
  - `LetDestructure` — `AstStmt::Let` with complex pattern (`stmt.rs:110-119`).
  - `ParamDestructure` — closure param with complex pattern (`expr.rs:1179-1188`).
  - `TryOp` — `AstExpr::Try` (`desugar.rs:590-606`).
  - `UnwrapOp` — `AstExpr::Postfix(Unwrap)` (`desugar.rs:672-688`).
- Type-infer: `generate.rs:388-424` — `gen_pat` on each arm's pattern with
  `scrutinee_tv` and `source`, equate arm bodies to result. Empty match poisons result.
- Solver: `solve_equal`; pattern constraints through `gen_pat`.
- MIR: `body_lower.rs:753-756` → `lower_match` (3319).
- Gotchas: analyzers check `MatchSource::is_desugared()` before running exhaustiveness
  or unreachable-arm checks. See `match_pattern_analyzer.md`.

### HirExpr::Break

- Produced by: `AstExpr::Break` (`expr.rs:151-154`). Also synthesized:
  - in `desugar_while` as the break exit (`desugar.rs:215-218`).
  - in `desugar_while_let` negation break (`desugar.rs:273-276`).
  - in `desugar_for_loop` for `.None => break` (`desugar.rs:428-431`).
- Type-infer: `generate.rs:432` — `Never`.
- MIR: `body_lower.rs:597` → `lower_break` (2909).

### HirExpr::Continue

- Produced by: `AstExpr::Continue` only (`expr.rs:155-158`).
- Type-infer: `generate.rs:432` — `Never`.
- MIR: `body_lower.rs:598` → `lower_continue` (2918).

### HirExpr::Return

- Produced by: `AstExpr::Return` (`expr.rs:159-165`). Also synthesized in `desugar_try`
  for the `.Break($early) => return .fromResidual(...)` arm (`desugar.rs:585-588`) and
  `desugar_throw` (`desugar.rs:627-630`).
- Type-infer: `generate.rs:434-445` — coerce value (or unit) to return type; result
  type is `Never`.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:599-605` → `Terminator::ret`.

### HirExpr::Assign

- Produced by: `AstExpr::Assignment` (`expr.rs:95-102`). 1:1.
- Type-infer: `generate.rs:448-457` — coerce value to target, result is unit.
- Solver: `solve_coerce`.
- MIR: `body_lower.rs:607-624` — setter dispatch first (`try_lower_setter_assign`), else
  direct `Place` write.

### HirExpr::Block

- Produced by: `AstExpr::Block` (`expr.rs:178-184`). Also synthesized:
  - Complex let-destructure wrapper (`stmt.rs:126-131`) — wraps the temp `Let` and the
    destructuring `Match` in one expression.
  - `desugar_for_loop` wraps body + loop in a block (`desugar.rs:417-420, 468-474`).
- Type-infer: `generate.rs:539` → `gen_block`.
- MIR: `body_lower.rs:625` → `lower_hir_block`.

### HirExpr::Error

- Produced by: `AstExpr::Error` (`expr.rs:186`). Also emitted at many HIR-lowering
  error sites:
  - Empty path (`expr.rs:229`).
  - Type args on a local variable (`expr.rs:261`).
  - Empty type argument brackets (`expr.rs:319`).
  - Ambiguous name (`expr.rs:411`).
  - Undefined path (`expr.rs:427`).
  - Instance method called on a type (`expr.rs:609`).
  - Missing binary/unary/compound-assign operator protocol
    (`desugar.rs:53, 74, 152, 194`).
  - Unwrap trap arm (`desugar.rs:670`).
  - Standalone rest pattern lowering (`pat.rs:235`) — but that's a pattern, not an
    expression.
- Type-infer: `generate.rs:541` — reports `InferError::FromHir`.
- MIR: `body_lower.rs:626` → `Immediate::error()`.
- Gotchas: `HirExpr::Error` is a concrete variant, not `null`/absence — analyzers and
  MIR must handle it, not assume it away.
