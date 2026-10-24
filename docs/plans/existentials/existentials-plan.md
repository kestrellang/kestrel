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
- [ ] `any_box_mismatch.ks`: E147, `any[OwnedBox] P` passed where
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
      the or-pattern binding-type mismatch (landed in M1, regression test
      `patterns/pattern_types/or_pattern_binding_type_mismatch.ks`);
      `is Circle or is Square` is accepted.
- [ ] `is_match_not_exhaustive.ks`: a match on `any Shape` without `_`
      is the existing non-exhaustive error, and a repeated `is Circle` arm
      warns as unreachable.
- [ ] `is_always_fails.ks`: E149 `// WARNING:` on the operator and on a
      pattern.

**MIR snapshots** (written in Phase 10, once the MIR shape exists to
snapshot):

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
      `any[OwnedBox] P`, payload deinit counted exactly once.
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
- [ ] `nested_existential_exec.ks`: `any[OwnedBox] Shape` built from an
      `any Shape` through self-conformance.
- [ ] `downcast_exec.ks`: `as?` hit and miss; `if let` / `guard let`.
- [ ] `consuming_downcast_exec.ks`: `consuming x as? T` on
      `any[OwnedBox] P`. A hit moves the value out; a miss returns `.Err`
      with the existential intact (call a method on it afterward).
- [ ] `empty_protocol_exec.ks`: `any Marker` plus `as?`.
- [ ] `is_operator_exec.ks`: `is` and `is not` in `if`, in `filter`,
      and combined with `and`/`or`.
- [ ] `is_custom_bool_exec.ks`: `let f: Flag = x is Circle` with a local
      `Flag: ExpressibleByBoolLiteral, BooleanConditional`.
- [ ] `is_patterns_exec.ks`: `is T`, `c is T`, `is T { .. }` and
      `r is T { .. }` in `match`, `if let`, `guard let` and `while let`;
      `x @ is T` binds the whole existential.

- [ ] Run `/triage` with pattern `types/existential` and record the
      baseline.

### Phases 1–3: front end (M1) — done

Landed on `feature/existentials`. Where it differs from the original plan,
the change and its reason:

- **Lexer / CST.** `is` (reserved) and `as?` are tokens. **`any` is
  contextual**, not a token: the stdlib and several tests call
  `any(where:)`, so reserving it would have broken them. The parser
  recognizes it in type position only when a bound, or `[` box `]` and then
  a bound, follows (`types.rs::at_any_type`). New nodes: `TyAny`,
  `ExprCast`, `ExprTypeTest`, `IsPattern`, and `TypeArgBinding`
  (`Item = Int64`).
- **Associated-type bindings are new syntax.** `some P[A = X]` never
  existed, so `TypeArgumentList` gained `TypeArgBinding`. It is only legal
  in an `any` bound; elsewhere it is the new parse error **E810**.
- **Parser.** `as?` binds at 80 (tighter than every binary operator), `is`
  at comparison level (30), both folded by `exprs.rs::type_operator`. The
  cast target takes no `?` / `??` / `throws` suffix (`cast_target_ty`), so
  `x as? T ?? d` is `(x as? T) ?? d`. `consuming` is an operand prefix only
  before `as?`. `is` patterns reuse the struct-pattern field list. Tests:
  `lib/kestrel-parser/tests/existentials.rs`, which also checks every
  `types/existential/` program parses cleanly.
- **AST.** `AstType::Any { box_ty, bounds: Vec<AnyBound>, span }`;
  `AnyBound { protocol, bindings }`; `AstType::any_children()` is the one
  walker the analyzers share. `kestrel_ast::pretty` renders it.
- **Builtins.** Only `DefaultBoxType` landed (alias kind, alias target),
  bound by `@builtin(.DefaultBoxType) public type DefaultBox[T] = RcBox[T]`
  in `rcbox.ks`. `Sized`, `ExistentialBox` and `ConsumingIndirection`
  arrive in M2 with the protocols they name.
- **HIR.** `HirTy::RawExistential { bounds, bindings: Vec<HirAssocBinding>,
  span }`; `any[B] P` lowers to `Struct { B, [RawExistential] }` in
  `hir-lower/src/ty.rs::lower_any_type`, bounds sorted by entity. E404 for
  an applied or non-name box, E141 when there is no default box, E154 for
  a non-protocol bound, **E150 for an unknown binding (moved up from M3:
  the binding is resolved here).**
- **E144** comes from `reject_bare_protocol` on value positions. Bound-like
  positions lower through `lower_bound_type` instead: `some` / `any` bounds,
  `extend P` targets (used throughout the stdlib), and the `P` of a qualified
  `P.Item` (a unit test caught this one). Unused declarations are reached
  by `type_annotation_resolution`, which forces the lowering when an
  annotation names a protocol in a value position — the same gate it
  already used for reference types.
- **`as?`, `is`, `is` patterns** lower to `HirExpr::Error` / `HirPat::Error`
  with **E155 "… are not supported yet"** (plus E153 for `var c is T`). The
  `HirExpr::Cast` / `HirExpr::TypeTest` / `HirPat::TypeTest` variants move to
  Phase 13, where they get semantics; adding them now would have meant a
  stub arm in every crate that matches on HIR.
- **Pulled forward from Phase 4** so the workspace builds:
  `TyKind::RawExistential`, `ResolvedTy::RawExistential`, structural
  unification, and arms in every consumer. Copy class is NotCopyable and
  Static is false everywhere (one answer per layer: semantics, solver,
  analyze). MIR maps it to the error type until Phase 9. Diagnostics print
  `any P` / `any[B] P`.
- **Or-pattern binding types** (folded into M1): each later alternative
  unifies with the first; a conflict is E109 with `Reason::OrPatternBinding`.
  Regression test: `patterns/pattern_types/or_pattern_binding_type_mismatch.ks`.
- **Not done: the two old bare-protocol tests.** Both are `stdlib: false`,
  but plain `any P` needs the stdlib's default box, and in the second one
  `any Slice[T]` would make the body ill-typed (`isEqual(to: Self)` is not
  dispatchable). They now fail with E144 — the correct behaviour — pending
  a decision on how to rewrite them.

**M1 result** (`cargo test --release -p kestrel-test-suite --test
file_tests -- types/existential`): 3 / 37 pass — `any_bare_protocol_type_rejected`,
`any_box_applied_rejected`, `any_assoc_unknown`. The rest need later
milestones. `patterns/` 277 / 277. Unit tests of every changed crate pass,
except `kestrel-lsp`'s integration tests, which need a debug
`kestrel-lsp` binary this sandbox doesn't build.

### Phases 4–5: `Sized`, box protocols, box analyzers (M2) — done

Landed on `feature/existentials`. As built, and where it differs from the
original plan:

- **Builtins.** `Sized` (`lang/std/core/sized.ks`, marker,
  `implicit_conformance`), `ExistentialBox` (`std.memory`, in
  `sharedbox.ks`), `ConsumingIndirection` (`indirection.ks`, labelled
  `consumePointee(body:)`). `SharedBox: ExistentialBox, Cloneable,
  MutableIndirection`; its `init` requirement moved into `ExistentialBox`.
- **One walker.** `kestrel_semantics::param_bound_contexts` walks a type
  parameter's where-clause scopes; the copy, static and size requirements
  all use it.
- **Sizedness** (`kestrel-semantics/src/sizedness.rs`).
  `TypeParamSizeRequirement`: `not Sized` relaxes; a positive `T: Sized`
  or a *bitwise-copy* bound (`Copyable` and refinements, **except** the
  `Cloneable` family, whose copies run `clone()`) wins. Intrinsic params
  may be unsized. `NominalSizedness` handles unsized-tail structs.
  `inject_implicit_sized_bounds` sits next to the Copyable injection
  (kept as its own function, not merged/renamed). Solver, resolver and
  entailment layers answer `Sized` from the same helpers; `Copyable`
  requires `Sized`.
- **Members needing `Sized`** are explicit, not implicit: by-value members
  of `not Sized` types say `where T: Sized`, and generic methods that
  can't constrain the outer `T` (E437) moved into
  `extend X[T] where T: Sized`. E403 enforces it.
- **Associated types.** `type X: not Sized` parses (`opt_conformance_list`
  in `type_alias_decl`); `Indirection.Target` is `not Sized`.
- **Stdlib.** `Pointer`, `RcBox`/`RcBoxStorage`, `CowBox` are
  `not Sized` and **`not Static`**: `rawany P` is only `Static` when `P`
  is (design §10), so a box that required `Static` payloads could not
  hold `any Shape`. They are now `Static` exactly when `T` is.
  `Layout.of`, `Layout.ofValue` and `RawPointer.cast`/`Pointer.cast`
  relax `Static` too. `CowBox: ExistentialBox` added. RcBox's payload
  offset is `lang.alignof_val` (new intrinsics `sizeof_val` /
  `alignof_val`, folded to the static values in MIR until Phase 9).
- **E402** (`decl/existential_box.rs`, rules in
  `kestrel_semantics::ExistentialBoxDefect`): one param declared
  `not Sized`, conforms to `ExistentialBox`, no stored field holding `T`
  by value (no tail exemption — a box is a fixed-size handle). Checks
  signatures (DeclCheck) and `let` annotations (BodyCheck, borrows E402).
  An invalid box's Copyable/Static formation bounds on `rawany P` are
  skipped (`generate.rs::is_invalid_existential_box`): E402 is the root
  cause.
- **E403** (`decl/unsized_param.rs`): a `not Sized` param by value in a
  signature or non-tail field. Intrinsics are exempt.
- **Moved to M3:** rejecting ref decay on unsized `T` (no `&rawany P` can
  reach a decay site until the existential member peel), and deleting the
  `Protocol`-is-`Copyable`/`Static` arms (dead now that a bare protocol is
  E144; removed with the M3 conformance work).
- **`OwnedBox`** (new, `lang/std/memory/ownedbox.ks`) replaces `UniqueBox`
  as the unique existential box. `UniqueBox` is the compiler-internal
  `consuming`-closure handle with no `deinit`, so it can't own a payload;
  it is unchanged. The existential tests use `any[OwnedBox] P`.
  `consumePointee[R]` relaxes `R: not Copyable, not Static` so it can hand
  back a move-only payload.
- **Mono fix.** The instantiation collector queued a type's `clone` impl
  on every `copy_value`, even at instances that aren't Cloneable
  (`OwnedBox[File]`). It now asks the shared copy classifier first
  (`ty_query::CopyEnv` / `copy_behavior_in`).
- **Known bug, test added:** a field write on `consuming self` lands on a
  copy (`memory_model/deinit/consuming_self_field_assign_seen_by_deinit.ks`).
  `OwnedBox.consumePointee` works around it with `var me = self`.

Exit (at the M2 commit): `types/existential` 6/37 — the remaining 31 need
M3+ (conversions, member lookup, dispatch, casts).
`stdlib/` 299/299, `types/pointer` 45/45, `memory_model` 363/374 and
closures 273/277 with every failure also failing on base `80297742`.

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
- [ ] `cargo fmt` and `cargo clippy` clean on every changed crate. Format
      only the hunks you changed (`rustfmt --check` per file, compared with
      the base): the base tree is not rustfmt-clean, and a whole-crate
      `cargo fmt` rewrites files other agents are editing.

### Phase 13: `as?`

Files: `lib/kestrel-type-infer/src/` (cast constraint),
`lib/kestrel-analyze/src/body/` (E149), `lib/kestrel-mir-lower/src/body/expr.rs`

- [ ] **HIR variants** (deferred from Phase 3): `HirExpr::Cast { expr,
      target, consuming }`, `HirExpr::TypeTest { expr, target, negated }`,
      `HirPat::TypeTest { binding, target, fields }`, replacing the E155
      stub in `hir-lower` (`expr.rs`, `pat.rs`). Remove E155 from
      `docs/error-codes.md`.
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
