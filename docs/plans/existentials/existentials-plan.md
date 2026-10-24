# Existentials (`any P`) Implementation Plan

> Companion to [`existentials-design.md`](existentials-design.md); section
> numbers (§N) refer to it. Base commit `80297742`.

## Ordering and milestones

The feature is large, so it lands as **five milestones**. Each one is green
on its own targeted tests and leaves the suite no worse than before.

| Milestone | Phases | What works at the end |
|---|---|---|
| **M1: Front end** | 0–3 | `any` / `any[B]` / `as?` parse and desugar. A bare protocol type is E144. Nothing runs yet |
| **M2: `Sized` and boxes** | 4–5 | `Sized` / `not Sized`, the box protocols and stdlib changes, box validity (E402, E403) |
| **M3: Type checking** | 6–8 | Conversions, member lookup, dispatchability, self-conformance, associated types, composition, all diagnostics |
| **M4: Runtime** | 9–12 | Fat pointers, unsized-tail layout, function tables, thunks, dynamic calls, on both backends. Execution tests pass |
| **M5: Downcasts and docs** | 13–14 | `as?`, consuming `as?`, the `is` operator and `is` patterns, user docs |

**Dependency on bidirectional checking.** The conversions (§4) live at
the check leaves of the bidirectional checker, which is being reworked on
`bidi-p0-clean` right now (`8029774`, "push expectations into every check
leaf"). M3 should start from a commit where that leaf rule is settled.
Merge the working branch into `feature/existentials` before Phase 6, and
coordinate with whoever owns that work if the leaf API is still moving.
M1 and M2 don't depend on it.

## Test strategy

- **Location:** `lib/kestrel-test-suite/testdata/types/existential/`,
  next to `types/opaque/`. Naming follows opaque types:
  - `any_*.ks` for diagnostics tests
  - `*_exec.ks` for execution tests
- **Diagnostics tests:** one per distinct error, with the full message in
  `// ERROR:` (or `// WARNING:`) on the offending line. Use
  `// stdlib: false` where the test only needs local protocols and boxes.
- **Execution tests:** `// stdlib: true`, `@main func main() -> lang.i64`,
  result checked through `// expect-exit:` or `// expect-stdout:`. Each
  must pass on Cranelift and LLVM, using the suite's existing backend
  coverage.
- **MIR snapshot tests** (`// test: mir`) for the table layout and the
  thunk shape, so a slot-order change shows up as a snapshot diff instead
  of a wrong-method call at runtime.
- **Running:** only through `/triage`, with a pattern such as
  `types/existential`. Do a full run before each commit.
- **Test format:** see the `write-tests` skill.
- **Tests document state.** Phase 0 writes everything up front; most of it
  fails until its phase lands. Never edit a test to make it pass. The
  exceptions are the two existing bare-protocol tests, which were approved
  for update in Phase 3.

## Phases

### Phase 0: Tests (fail first)

**Diagnostics** (`types/existential/`):

- [ ] `any_bare_protocol_type_rejected.ks`: E144. One `// ERROR:` per
      position: a local annotation, a parameter, a return type, a field,
      and a generic argument (`Array[Shape]`).
- [ ] `any_box_applied_rejected.ks`: E404 (`any[RcBox[Int64]] Shape`).
- [ ] `any_box_invalid.ks`: E402, all three variants. `any[Array] P`
      (parameter not `not Sized`); a `not Sized` struct that isn't an
      `ExistentialBox`; a box with a field `var items: T`.
- [ ] `any_unsized_by_value.ks`: E403, `T: not Sized` used as a local,
      parameter, return type, and non-tail field.
- [ ] `any_box_mismatch.ks`: E147, `any[UniqueBox] P` passed where
      `any P` is expected, and the reverse.
- [ ] `any_nondispatchable_calls.ks`: E145, all four variants: static,
      generic, `Self` parameter (`any Equatable` `==`), and unbound
      associated type (`next()` on plain `any Iterator`).
- [ ] `any_self_conformance_fails.ks`: E146, `any Equatable` passed to
      `func f[T: Equatable]`.
- [ ] `any_consuming_through_shared_box.ks`: E148, consuming requirement
      on `any P` (RcBox) and on `any[CowBox] P`.
- [ ] `any_cowbox_requires_cloneable.ks`: `any[CowBox] P` where `P` isn't
      `Cloneable` (the error comes from `CowBox`'s own bound).
- [ ] `any_cast_always_fails.ks`: E149 `// WARNING:`.
- [ ] `any_assoc_unknown.ks`: E150.
- [ ] `any_assoc_mismatch.ks`: E151.
- [ ] `any_consuming_cast_shared_box.ks`: consuming `as?` on `any P`
      (RcBox) is E148.
- [ ] `is_non_existential_lhs.ks`: E152, with a concrete left operand
      and with a generic `T`.
- [ ] `is_var_binding_rejected.ks`: E153.
- [ ] `is_struct_pattern_without_is.ks`: `Rect { width, .. }` on
      `any Shape` is the existing struct-pattern type mismatch.
- [ ] `is_or_pattern_binding_types.ks`: `c is Circle or c is Square` is
      the existing or-pattern binding-type error; `is Circle or is Square`
      is accepted.
- [ ] `is_match_not_exhaustive.ks`: a match on `any Shape` without `_`
      is the existing non-exhaustive error, and a repeated `is Circle` arm
      warns as unreachable.
- [ ] `is_chained_parse_error.ks`: `a is Circle == b` is a parse error.
- [ ] `is_always_fails.ks`: E149 `// WARNING:` on the operator and on a
      pattern.

**MIR snapshots:**

- [ ] `vtable_layout_mir.ks`: one protocol with a parent and two
      requirements. Snapshot the `VTableDef`: header, parent pointer, and
      slots in declaration order.
- [ ] `vtable_composition_mir.ks`: `any P and Q`. Snapshot the empty slot
      section and the canonical parent order.

**Execution:**

- [ ] `basic_dispatch_exec.ks`: `Array[any Shape]` with two conformers;
      sum of `area()`.
- [ ] `rcbox_aliasing_exec.ks`: copy an `any Counter`, mutate one copy,
      observe it through the other.
- [ ] `cowbox_value_semantics_exec.ks`: the same with `any[CowBox]`
      (`P: Cloneable`); copies stay independent.
- [ ] `unique_box_consuming_exec.ks`: consuming requirement through
      `any[UniqueBox] P`, payload deinit counted exactly once.
- [ ] `payload_deinit_exec.ks`: dropping the last `any P` runs the
      payload's `deinit` once; with RcBox, not while another copy lives.
- [ ] `move_only_payload_exec.ks`: a non-Copyable payload in the default
      box; reads and mutating calls work.
- [ ] `parent_upcast_exec.ks`: `any Child` → `any Parent`; dispatch
      through the parent table.
- [ ] `composition_exec.ks`: `any P and Q`, call both sides, upcast to
      each.
- [ ] `self_conformance_generic_exec.ks`: pass `any P` to
      `func f[T: P]` and to a `some P` parameter; call a protocol extension
      method on `any P`.
- [ ] `iterator_assoc_exec.ks`: `any Iterator[Item = Int64]` over two
      iterator types; `for-in` and `map` through self-conformance.
- [ ] `protocol_member_first_exec.ks`: a requirement named `isUnique` on
      `any P` calls the payload's method, not `RcBox.isUnique()`.
- [ ] `generic_payload_exec.ks`: `Box[Int64]: Shape` and
      `Box[String]: Shape` in one array (two tables).
- [ ] `nested_existential_exec.ks`: `any[UniqueBox] Shape` built from an
      `any Shape` through self-conformance.
- [ ] `overaligned_payload_exec.ks`: a payload with alignment > 8 in each
      stdlib box; fields read back intact.
- [ ] `downcast_exec.ks`: `as?` hit and miss; `if let` / `guard let`.
- [ ] `consuming_downcast_exec.ks`: `consuming x as? T` on
      `any[UniqueBox] P`. A hit moves the value out; a miss returns `.Err`
      with the existential intact (call a method on it afterward).
- [ ] `empty_protocol_exec.ks`: `any Marker` plus `as?`.
- [ ] `is_operator_exec.ks`: `is` and `is not` in `if`, in `filter`,
      and combined with `and`/`or`.
- [ ] `is_custom_bool_exec.ks`: `let f: Flag = x is Circle` with a local
      `Flag: ExpressibleByBoolLiteral, BooleanConditional`.
- [ ] `is_patterns_exec.ks`: `is T`, `c is T`, `is T { .. }` and
      `r is T { .. }` in `match`, `if let`, `guard let` and `while let`;
      `x @ is T` binds the whole existential.

**Stdlib regression** (`stdlib/memory/`, independent of existentials):

- [ ] `rcbox_overaligned_payload_exec.ks` and the `UniqueBox` equivalent.
      These document the fixed-offset bug (design §3) that exists today.

- [ ] Run `/triage` with pattern `types/existential` and record the
      baseline. Everything except the stdlib regression should fail.

### Phase 1: Lexer and CST

Files: `lib/kestrel-lexer/src/lib.rs`, `lib/kestrel-syntax-tree/kinds.txt`,
`lib/kestrel-syntax-tree/kestrel.ungram`

- [ ] **`any` and `is` keywords.** Add `#[token("any")] Any` and
      `#[token("is")] Is` next to `Some` (`lexer/src/lib.rs:232`).
      - `is` is reserved (decided).
      - First list every stdlib, example and test identifier named `is` or
        `any`, then rename them in the same commit, before the tokens
        land.
- [ ] **CST kinds.** Append to `kinds.txt` (it is append-only):
      - `token Any 'any'`
      - `node TyAny`
      - `node CastExpr`
      - `token AsQuestion 'as?'`, or reuse `As` plus `Question`, whichever
        the lexer can produce without breaking `?` and `??`.
      - `token Is 'is'`, `node TypeTestExpr`, `node IsPat`.
- [ ] **Grammar.** `kestrel.ungram`:
      - `TyAny = 'any' TypeArgumentList? Ty ('and' Ty)*`
      - `CastExpr = 'consuming'? Expr 'as?' Ty`
      - `TypeTestExpr = Expr 'is' 'not'? Ty`
      - `IsPat = Name? 'is' Ty ('{' (StructPatternMember (',' StructPatternMember)* ','?)? '}')?`,
        reusing `StructPattern`'s member list (`kestrel.ungram:595`). It
        takes a full `Ty` rather than `'ident'`, so `is Box[Int64] { .. }`
        works.
- [ ] **Regenerate:**
      `UPDATE_GENERATED=1 cargo test -p kestrel-syntax-tree --test sourcegen`.

### Phase 2: Parser

Files: `lib/kestrel-parser/src/grammar/types.rs`,
`lib/kestrel-parser/src/grammar/exprs.rs`

- [ ] **Type start.** `at_type_start` (`types.rs:31`) includes `K::Any`.
- [ ] **Type dispatch.** `base_ty` (`types.rs:99`): `Some(K::Any) => any_type(p)`,
      modelled on `some_type` / `some_bound` (`types.rs:297-326`).
      - Parse an optional `[` *single type path* `]` as the box.
      - Then bounds joined by `and`, reusing `some_bound` so
        `Iterator[Item = Int64]` parses the same as under `some`.
- [ ] **`as?`.** Add it in `postfix` (`exprs.rs:284`) as a postfix
      operator that takes a type operand.
      - It binds tighter than binary operators and looser than calls, so
        `shapes(0) as? Circle` parses as `(shapes(0)) as? Circle`.
      - Add a binding-power entry next to `K::QuestionQuestion`
        (`exprs.rs:137`) so `x as? T ?? d` groups as `(x as? T) ?? d`.
- [ ] **Consuming cast.** A `consuming` prefix, accepted only immediately
      before a `CastExpr`. There is no general `consuming <expr>` form
      today, so nothing else may parse.
- [ ] **`is` operator.** A binary operator at comparison precedence,
      non-associative. The right operand is parsed as a type, not an
      expression. `is not` is one operator.
- [ ] **`is` patterns** in the pattern grammar (`grammar/patterns.rs`):
      an optional binding name, then `is`, a type, and optional struct
      fields. `var` before it parses, so the error comes from HIR
      lowering as E153 rather than a vague parse error.
- [ ] **Errors.** A parser error for `any` with no bound. Applied boxes
      (`any[RcBox[T]] P`) parse fine; E404 is raised in HIR lowering.

### Phase 3: AST, builtins, HIR desugaring, E144 / E404

Files: `lib/kestrel-ast/src/ast_type.rs`, `lib/kestrel-ast-builder/src/ast_type.rs`,
`lib/kestrel-hir/src/builtin.rs`, `lib/kestrel-hir/src/ty.rs`,
`lib/kestrel-hir-lower/src/ty.rs`, `lib/kestrel-hir-lower/src/expr.rs`,
`lang/std/memory/*.ks`

- [ ] **AST.** `AstType::Any { box_ty: Option<AstPath>, bounds, span }`,
      next to `AstType::Some` (`ast_type.rs:141`), built in the
      ast-builder (`ast_type.rs:~97`).
- [ ] **New builtin slots** (`builtin.rs`): `DefaultBoxType` (alias kind,
      plus an arm in `denotes_alias_target` at `:735`), `Sized` (marker
      protocol, `implicit_conformance: true`), `ExistentialBox`, and
      `ConsumingIndirection`.
      - Add `name()`, `from_attribute_name()` and `kind()` arms for each.
      - The inert-annotation unit test (`:985+`) must stay green.
- [ ] **Stdlib default binding.** In `lang/std/memory/rcbox.ks`:
      `@builtin(.DefaultBoxType) public type DefaultBox[T] = RcBox[T]`.
      - It must be generic, like `DefaultArrayLiteralType`.
      - The protocols arrive in Phase 4. The alias only needs `RcBox`.
- [ ] **HIR type.** `HirTy::RawExistential { bounds: Vec<(Entity, Vec<HirTy>, Vec<(Name, HirTy)>)>, span }`
      in `kestrel-hir/src/ty.rs`, holding each protocol, its arguments,
      and its associated-type bindings.
- [ ] **HIR lowering of `any`** (`hir-lower/src/ty.rs`, next to the
      `Opaque` arm at `:175-214`):
      - Resolve the box path. Omitted means `ResolveBuiltin(DefaultBoxType)`.
      - Lower to `HirTy::Struct { entity: box, args: [RawExistential] }`.
      - Canonicalise the bound order (sort by entity) here, so
        `any P and Q` and `any Q and P` are the same type everywhere after.
      - Box path has type arguments → E404.
- [ ] **E144: bare protocol in a value position.** In HIR type lowering,
      when `HirTy::Protocol` would be produced for a value-position type:
      a local, a parameter, a return type, a field, or a generic argument.
      - Bound positions (`T: P`, `where`, `some P`, the bounds of `any`)
        are unchanged.
      - Emit E144 and lower to the error type, so the rest of the body
        doesn't cascade.
- [ ] **`as?` expression.** `HirExpr::Cast { expr, target: HirTy, consuming: bool }`
      in `lower_expr_node`, recorded in the `BodySourceMap`.
- [ ] **`is` expression.** `HirExpr::TypeTest { expr, target, negated }`.
- [ ] **`is` pattern.** `HirPat::TypeTest { binding: Option<Binding>, target, fields }`
      in `lower_pat_node`. `var` → E153.
- [ ] **Diagnostic codes.** Allocate E144 and E404 per
      `lib/kestrel-analyze/AGENTS.md` (registry, `docs/error-codes.md`).
      Reserve E145–E153 and E402–E403 together, so later phases don't
      collide with other agents.
- [ ] **Update the two bare-protocol tests to `any P` (approved):**
      - `declarations/extensions/extension_conformance_on_generic_type.ks`
      - `declarations/extensions/protocol_extension_satisfies_conformance_via_transitive_protocol.ks`

      Then confirm they fail the same way the other existential tests do,
      until M4.
- [ ] **Triage:** `types/existential/any_bare_protocol` and
      `any_box_applied` pass; `declarations/extensions` is no worse.

### Phase 4: `Sized` and `not Sized`

Files: `lib/kestrel-type-infer/src/where_clauses.rs`,
`lib/kestrel-semantics/src/lib.rs`, `lib/kestrel-copy-fold/src/lib.rs`,
`lib/kestrel-type-infer/src/ty.rs`

- [ ] **Raw existential type.** `TyKind::RawExistential { bounds, bindings }`
      in `type-infer/src/ty.rs`, lowered from `HirTy::RawExistential`.
      Unification is structural on the canonical bound list.
- [ ] **Sizedness query.** `TypeParamSizeRequirement` (or a
      `Sized`/`MaybeUnsized` field next to `TypeParamCopyRequirement`,
      `semantics/lib.rs:269`), driven by `where T: not Sized`. **One
      source of truth:** add a single negative-bound walker that returns
      both, rather than a second walker of the same where-clauses.
- [ ] **Implicit bound.** `inject_implicit_copyable_bounds`
      (`where_clauses.rs:296`) also injects `T: Sized` unless opted out.
      Rename it to `inject_implicit_bounds` to match.
- [ ] **`type_satisfies(…, Sized)`.** False for `RawExistential` and for
      an unsized-tail struct instantiated at an unsized argument; true for
      everything else.
- [ ] **Members needing `Sized`.** On a `not Sized` declaration, a member
      whose signature uses `T` by value carries an implicit `T: Sized`
      requirement. It is checked like a conditional extension member, so
      `UniqueBox.takeValue()` is simply unavailable at `T = rawany P`.
- [ ] **Associated types.** Associated types are implicitly `Sized`
      unless declared `type X: not Sized`; that opt-out is parsed and
      stored next to the type-parameter one. Requirements that use an
      associated type by value get the implicit `Sized` requirement.
      Declare `type Target: not Sized` on `Indirection` here, and check
      that `try_indirection_peel` (`solver.rs:4035`) still resolves
      `Target` on every existing conformer.
- [ ] **Unsized references in type-infer.** The ref-decay rule (`&T`
      used as `T`) never applies when `T` is unsized; reject it with a
      type error instead.
- [ ] **Remove the wrong arms.** Delete the `Protocol`-is-`Copyable` arms
      (`solver.rs:2805`, `resolve.rs:2541`, `semantics/lib.rs:612`) and
      the `Static` arm (`staticness.rs:368`).
      - `RawExistential` is never `Copyable`.
      - It is `Cloneable` iff a bound refines `Cloneable` (§1).
      - It is `Static` iff a bound refines `Static` (§10).

### Phase 5: Box protocols, stdlib changes, and box analyzers

Files: `lang/std/core/indirection.ks`, `lang/std/memory/{sharedbox,rcbox,cowbox,uniquebox,pointer}.ks`,
`lib/kestrel-analyze/src/decl/existential_box.rs` (new),
`lib/kestrel-analyze/src/decl/unsized_param.rs` (new),
`lib/kestrel-analyze/src/decl/mod.rs`

- [ ] **Protocols.**
      - `ExistentialBox: Indirection { init(consuming value: Target) }`
        in `std.memory`, with `@builtin(.ExistentialBox)`.
      - `ConsumingIndirection` in `std.core/indirection.ks`, with
        `@builtin(.ConsumingIndirection)`. Exact shape in §3.
- [ ] **`SharedBox` refines it.** `SharedBox: ExistentialBox, Cloneable, MutableIndirection`.
      Its `init` requirement moves into `ExistentialBox`. Check that the
      escaping-closure box binding (`closure_box.rs:121`, which matches
      members by requirement shape) still resolves `init`.
- [ ] **`not Sized` on the boxes.** Add it to `Pointer`, `RcBox` and
      `RcBoxStorage`, `CowBox`, and `UniqueBox` and `UniqueBoxStorage`.
- [ ] **Fix payload offsets.** Replace the fixed
      `valueOffset = sizeof(Int64)` (`rcbox.ks:110`, and the same in
      `uniquebox.ks`) with an offset computed from the payload's alignment.
      - Sized `T`: `Layout.of[T]().align`.
      - Unsized `T`: a new `lang.alignOfVal(ptr)`, lowered in Phase 9.
      - The stdlib overaligned-payload regression tests should pass after
        this, independent of existentials.
- [ ] **`UniqueBox` conformances:** `Indirection`, `MutableIndirection`,
      `ExistentialBox`, `ConsumingIndirection`, and
      `extend UniqueBox[T]: Cloneable where T: Cloneable`.
      - At a sized `T`, `clone` allocates and clones the value.
      - At an unsized payload, `clone` allocates with `sizeOfVal`/`alignOfVal`
        and copies through the table's `clone_into` entry. That path is
        finished in Phase 10, once tables exist.
      - `consumePointee` frees the block without running the payload's
        deinit.
      - `takeValue()` becomes `consumePointee { p in p.move() }`.
      - Its `consuming`-closure clients (`closure_box.rs:141`) must be
        unaffected.
- [ ] **Box-validity analyzer** (`decl/existential_box.rs`, E402). This
      runs at each `any[B] P` use site, but the per-declaration structural
      check is a query keyed on `B`, so it runs once per box. It checks:
      - exactly one type parameter, declared `not Sized`;
      - `B` conforms to `ExistentialBox`;
      - every stored field that mentions `T` is `Pointer[T]`,
        `Pointer[S[T]]` where `S` is an unsized-tail struct ending in `T`,
        or itself a valid box of `T`.
- [ ] **Unsized-by-value analyzer** (`decl/unsized_param.rs`, E403).
      Inside a `not Sized` declaration, `T` must not appear by value as a
      local, parameter, return type, or non-tail field.
- [ ] Register both in the analyzer list, with codes and
      `docs/error-codes.md` entries.
- [ ] **Triage:** `any_box_invalid`, `any_unsized_by_value`, and
      `stdlib/memory` (no regressions in the boxes, the `escaping` closure
      tests, or the `consuming` closure tests).

### Phase 6: Conversions at check leaves

Files: `lib/kestrel-type-infer/src/solver.rs`, `lib/kestrel-type-infer/src/ctx.rs`,
`lib/kestrel-type-infer/src/result.rs`

- [ ] **Replace the old coercion.** Remove the `T → Protocol` coercion
      (`solver.rs:2080-2091`), including the `SelfType{P}` arm. Replace it
      with the three rules of §4, applied when the expected type is
      `B[RawExistential]`:
      - **Box:** found a sized `T` with `T: every bound` and the
        associated-type bindings satisfied.
      - **Unsize:** found `B[T]` with the same `B`.
      - **Upcast:** found `B[RawExistential Q]` where `Q`'s bounds
        imply the expected bounds (a parent, or a subset of a composition).
- [ ] **Record the conversion.** One side table,
      `ctx.existential_conversions[expr] = ExistentialConversion { kind: Box | Unsize | Upcast { path }, concrete: TyId, target: TyId }`,
      surfaced through `InferResult` the way `PromotionInfo` is. Single
      source of truth for MIR lowering.
- [ ] **Errors.**
      - Different box → E147.
      - Associated-type binding not satisfied → E151.
      - Unknown associated-type name → E150. Checked when lowering the
        type, in HIR or the type resolver, whichever owns `some P[A = X]`
        today; use the same place.
- [ ] **Error paths poison.** Failed conversions poison the expression's
      type variable rather than unifying with the target. See the
      `cascading_infer_errors` memory.
- [ ] **New `InferError` variants** need arms in `error.rs`, in
      `result.rs` `describe_error`, and in the driver's
      `describe`/`format_error` (`lib/kestrel-type-infer/AGENTS.md`).

### Phase 7: Dispatchability and member lookup

Files: `lib/kestrel-semantics/src/` (new query), `lib/kestrel-type-infer/src/solver.rs`,
`lib/kestrel-type-infer/src/resolve.rs`

- [ ] **`ExistentialLayout(protocol)` query.** It returns:
      - the ordered dispatchable requirements;
      - the reason each excluded requirement is excluded (static,
        generic, `Self` outside the receiver, or the names of the
        associated types it mentions);
      - the parent list.

      It depends only on the protocol's declaration (design §5,
      incremental). Type-infer (for errors) and MIR (for slot indices)
      both read it; nobody recomputes slot order.
- [ ] **Protocol-first peel.** In `solve_member`, when the receiver is
      `B[RawExistential]`, peel *before* wrapper lookup, like the eager
      `&T` arm (`solver.rs:4255-4290`), and resolve against the bounds.
      Fall back to the box's own members only on a miss. Record it in
      `ctx.indirection_peels` the same way, so the MIR replay
      (`mir-lower/src/body/expr.rs:617-690`) needs no new path for the
      peel itself.
- [ ] **Requirement access check.**
      - A resolved requirement that isn't dispatchable → E145, with the
        reason from `ExistentialLayout`.
      - The box lacks the access the requirement needs (mutating with no
        `MutableIndirection`, consuming with no `ConsumingIndirection`)
        → E148.
- [ ] **Extension methods.** Protocol extension methods resolve through
      self-conformance (Phase 8), not the table.

### Phase 8: Self-conformance, associated types, composition

Files: `lib/kestrel-type-infer/src/entail_layer.rs`,
`lib/kestrel-type-infer/src/resolve.rs`, conformance queries in
`lib/kestrel-semantics/src/`

- [ ] **Synthesized conformances.** In the conformance query (the
      `conforms_to` / `ConformingProtocolInstantiations` path):
      - `RawExistential{bounds}` conforms to each bound and each bound's
        parents when every requirement is dispatchable.
      - `B[RawExistential]` conforms when, in addition, `B` grants every
        requirement's access.
- [ ] **Remove the old `Holds` shortcut.** Drop the "protocol-typed
      receiver is an existential" case (`entail_layer.rs:218-220`); the
      synthesized conformance replaces it.
- [ ] **E146.** When a `T: P` obligation fails only because
      self-conformance doesn't hold, report E146 naming the blocking
      requirement (from `ExistentialLayout`), not a generic "does not
      conform".
- [ ] **Associated-type projection.** On `RawExistential` (and on the
      box, through self-conformance), projecting an associated type
      returns the bound type. An unbound associated type has no
      projection; members mentioning it are excluded by Phase 7.
- [ ] **Operators and `for-in`** on existentials go through
      self-conformance (`ctx.protocol_dispatch_members` stays unpeeled).
- [ ] **Triage:** every diagnostics test in `types/existential/` passes.
      This is the **M3 exit**.

### Phase 9: MIR types, fat pointers, unsized layout

Files: `lib/kestrel-mir/src/ty.rs`, `lib/kestrel-mir/src/passes/layout.rs`,
`lib/kestrel-mir-lower/src/ty.rs`, `lib/kestrel-mir-lower/src/body/call/intrinsic.rs`,
`lib/kestrel-ast-builder/src/lang_module.rs`

- [ ] **MIR type.** `MirTy::RawExistential { bounds: Vec<(Entity, Vec<TyId>)>, bindings }`.
      Lower it from `HirTy::RawExistential` and `ResolvedTy`
      (`mir-lower/src/ty.rs:130`, `result.rs:227`). Remove the
      `HirTy::Protocol` → `Named` path; E144 already guarantees it can't
      be reached in a value position.
- [ ] **Two-word pointers.** `lang.ptr[X]` where `X` is unsized
      (`RawExistential`, or an unsized-tail struct at an unsized argument)
      is two words: (data, table).
      - The layout pass (`passes/layout.rs:121`, `:154-171`) and
        `mono_size_and_align` both use one helper, `pointee_is_unsized(ty)`,
        as the single source of truth.
      - `Pointer[T]` and every box built on it become two words at an
        unsized `T` with no source change.
      - `&X` and `&mutating X` at an unsized `X` are two words too, using
        the same helper.
- [ ] **Escape checker on two-word references.** It treats them like any
      other reference: provenance flows through the data word, and the
      table word carries none. Add a MIR test that borrows through
      `pointeeRef()` on an existential box and then tries to let the
      reference escape. It must be rejected the same way as for a sized
      payload.
- [ ] **Unsized-tail field offset.** The offset of the last field of an
      unsized-tail struct is `round_up(prefix_size, table.align)`,
      computed at runtime. All other field offsets are static.
- [ ] **Intrinsics.** New entries in `lang_module.rs` next to
      `sizeof`/`alignof` (`:683-684`): `lang.sizeOfVal(ptr)`,
      `lang.alignOfVal(ptr)`. Extend `lang.drop_in_place` on a two-word
      pointer to call the table's drop slot. Lowered in `intrinsic.rs`
      (`:1316`, `:1417`). On a sized pointee they fold to the static values.
- [ ] **Unsized function-table pointer.** `Pointer[RcBoxStorage[rawany P]]`
      carries the `(RcBox, T, P)` table. The table pointer passes
      through the unsized-tail struct unchanged.

### Phase 10: Function tables, thunks, dynamic calls

Files: `lib/kestrel-mir/src/item/` (new `vtable.rs`), `lib/kestrel-mir/src/inst.rs`,
`lib/kestrel-mir/src/callee.rs`, `lib/kestrel-mir/src/mono/` (new `vtable.rs`,
plus `mod.rs`, `expand.rs`, `verify.rs`), `lib/kestrel-mir-lower/src/body/`

- [ ] **New MIR items.**
      - `VTableKey { boxed: Entity, concrete: TyId, existential: TyId }`,
        where `boxed` is the box `B`
      - `VTableDef { key, entries: Vec<VTableEntry> }`, with
        `VTableEntry = FuncAddr(InstanceId) | Size | Align | TypeDescriptor(TyId) | Parent(VTableKey) | Absent`
      - `Op::FuncAddr(InstanceId)`, `Op::VTableAddr(VTableKey)`
      - `Callee::Dynamic { table: Value, slot: u32 }`
- [ ] **Lower conversions** (`mir-lower/src/body/`, from
      `existential_conversions`):
      - **Box:** call `B[T].init(consuming:)`, then unsize.
      - **Unsize:** rebuild `B`'s pointer fields as two-word pointers with
        `Op::VTableAddr`.
      - **Upcast:** load the parent table pointer at the composition or
        parent index from `ExistentialLayout`.
- [ ] **Lower requirement calls** on a peeled raw existential to
      `Callee::Dynamic`, with the slot index from `ExistentialLayout`.
      The receiver is the data word.
- [ ] **Mono table synthesis** (`mono/vtable.rs`). On the first
      `VTableAddr(key)` reached with a concrete `T`:
      - Intern the key in a single map on the mono state.
      - Build the header (drop shim from `drop_shim.rs`, size/align from
        layout, the type descriptor, a clone shim when the existential is
        `Cloneable`).
      - Build the parent keys, recursively and on demand.
      - Build one thunk per slot.
- [ ] **Thunks.** Each thunk is an ordinary MIR function synthesized like
      `synthesize_env_shims` (`closure_box.rs:~467`). It re-types the data
      word as `Pointer[T]`, adapts `self` to `T`'s witness convention, and
      calls the witness, which it resolves through the existing
      `resolve_witness_call` (`mono/witness.rs:397`). Thunks become mono
      roots.
- [ ] **Consuming-requirement thunks** go through
      `ConsumingIndirection.consumePointee` on the key's box. The thunk
      receives the box by value. Non-consuming thunks don't depend on `B`,
      so they are interned per `(T, requirement)` and shared between the
      tables of different boxes.
- [ ] **Type descriptors.** One `TypeDescriptor` static per concrete type
      that appears in any table. Its address is the identity.
- [ ] **Verify.** Post-mono checks in `mono/verify.rs`: no `VTableAddr`
      with a non-concrete key, and no `Callee::Dynamic` slot out of range.
- [ ] **Tracing.** Add `ktrace!("vtable", …)` at table creation and slot
      resolution, and add `vtable` to the category list in `CLAUDE.md`.

### Phase 11: Codegen (Cranelift and LLVM)

Files: `lib/kestrel-codegen-cranelift/src/{ty,inst,context}.rs`,
`lib/kestrel-codegen-llvm/src/{ty,inst,context}.rs`

- [ ] **Value types.** Two-word pointer values in `ty.rs`, the same
      handling as the existing multi-word closure values
      (`func_thick_words`).
- [ ] **Tables.** Each `VTableDef` becomes a read-only global with
      function relocations. Internal linkage, matching mono functions
      (`codegen-llvm/src/context.rs:300`, `codegen-cranelift/src/context.rs:309`).
- [ ] **Instructions.**
      - `Op::FuncAddr` and `Op::VTableAddr` become symbol addresses.
      - `Callee::Dynamic` becomes a load of slot `n`, then an indirect call.
      - `Op::TypeDescriptor` statics are 1-byte globals, so they are
        guaranteed distinct.
- [ ] **Triage:** all `types/existential/*_exec.ks` except the downcast
      tests pass on both backends, along with the MIR snapshots. This is
      the **M4 exit**.

### Phase 12: Full regression pass

- [ ] **Full `/triage` run.** Compare against the Phase 0 baseline. The
      areas likely to regress:
      - escaping and consuming closures (box protocol changes);
      - `stdlib/memory`;
      - anything that relied on a bare protocol type.
- [ ] `cargo fmt` and `cargo clippy` clean on every changed crate.

### Phase 13: `as?`

Files: `lib/kestrel-type-infer/src/` (cast constraint),
`lib/kestrel-analyze/src/body/` (E149), `lib/kestrel-mir-lower/src/body/expr.rs`

- [ ] **Typing.**
      - `x as? T` with `x: B[RawExistential]` and a concrete, sized `T`
        has type `Optional[&T]`.
      - `consuming x as? T` has type `Result[T, B[RawExistential]]` and
        requires `B: ConsumingIndirection` (E148 otherwise).
      - `as?` on a non-existential is a type error (reuse the existing
        mismatch message).
      - `as?` to an existential target is rejected; it is out of scope
        (design edge cases).
- [ ] **`is` operator typing.** The result is a fresh literal slot
      (`fresh_literal`) with a new `LiteralKind::Bool` origin, or the
      existing boolean-literal kind if there is one. It goes through
      `default_literal_var` like `true`. The left operand must be
      `B[RawExistential]`, otherwise E152 (concrete or generic).
- [ ] **`is` pattern typing.** The binding is `&T`. Fields check against
      `T`'s struct pattern rules. The scrutinee must be an existential
      (E152).
- [ ] **Exhaustiveness** (`lib/kestrel-pattern-matching`). A
      `TypeTest(TyId)` constructor over an open domain, modelled on
      integer literals in `usefulness.rs`, so the non-exhaustive and
      unreachable diagnostics come for free.
- [ ] **E149 warning.** When `T` can't conform to every bound, warn in a
      body analyzer. One check covers `as?`, the `is` operator and `is`
      patterns.
- [ ] **Lowering.**
      - Load the table's type-descriptor slot and compare it with
        `T`'s descriptor address.
      - **Hit:** a reference to the payload re-typed as `&T`.
      - **Consuming hit:** `consumePointee { p in p.cast[T]().move() }`.
      - **Miss:** `.None` / `.Err(x)`.
      - **`is` operator:** the same comparison as an `lang.i1`, passed to
        the chosen type's `init(boolLiteral:)`. `is not` inverts it.
      - **`is` patterns:** a decision-tree branch on the comparison; on a
        hit, bind `&T` and continue with the field sub-patterns.
      - One helper emits the descriptor comparison for all three.
- [ ] **Triage:** the downcast and `is_*` execution tests, and the
      `any_cast_*` and `is_*` diagnostics, pass. This is the **M5 exit**, together with Phase 14.

### Phase 14: Documentation

- [ ] **`docs/language/existentials.md`** (user-facing): `any P`,
      `any[B] P`, choosing a box, dispatchability, `as?`, `is`, the RcBox
      aliasing gotcha.
- [ ] **`docs/language/pattern-matching.md`:** an `is` patterns section.
- [ ] **Cross-references.** Update `docs/language/opaque-types.md`'s
      comparison paragraph, and point `docs/design/shared-box.md`'s `any P`
      section (`:146`, `:175-188`) at the design doc.
- [ ] **Crate architecture docs.** `docs/architecture.md` in
      `kestrel-mir` (new items, mono table synthesis), `kestrel-type-infer`
      (`RawExistential`, conversions, peel order), and the codegen crates.
- [ ] **`write-kestrel` skill:** `any` syntax and the
      "boxes are part of the type" gotcha.
- [ ] **Tracking.** `ROADMAP.md` (move existentials to 0.18, matching the
      milestone), `docs/plans/index.md` status, issue #29.

## Verification

- [ ] Targeted triage green at each milestone exit (M1–M5).
- [ ] Full triage before each commit, with no new failures outside
      `types/existential/`.
- [ ] `cargo fmt` and `cargo clippy -p <changed crates>` clean.
- [ ] Crate `docs/architecture.md` files updated where pipeline position
      or core types changed.

## Risks

- **The bidirectional checker is moving underneath Phase 6.** See
  [Ordering and milestones](#ordering-and-milestones).
- **`Sized` touches every generic.** The implicit bound is injected
  everywhere, so a mistake shows up as unrelated generic failures. Land
  Phase 4 alone and run the full suite before continuing.
- **The `SharedBox` refactor touches escaping closures.** `closure_box.rs`
  finds members by requirement shape. Moving `init` into a parent protocol
  must keep that lookup working. Run the closure tests right after Phase 5.
- **Two-word references.** The escape checker and the ref-decay rule
  have only ever seen one-word references. Expect assumptions about
  reference size in MIR lowering and both codegens; search for them before
  Phase 9 rather than finding them by crash.
