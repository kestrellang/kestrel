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

### Phases 6–8: type checking (M3) — done

Landed on `feature/existentials`, after merging `bidi-p0-clean` (`c63bef15`).
As built:

- **Conversions** (`kestrel-type-infer/src/existential.rs::try_convert`),
  called from `solve_coerce` at the check leaf, before `FromValue`
  promotion. Box (sized `T: P`), Unsize (`B[T]`), Upcast (same box, every
  target bound refined by a source bound, bindings equal). Recorded in
  `InferCtx::existential_conversions` and surfaced as
  `TypedBody::existential_conversions` (`ConversionKind`, resolved
  `from`/`to`) for MIR. The old `T → Protocol` / `SelfType` coercion arm is
  gone.
  - E147 for a different box; E151 when a pinned associated type differs
    (the source's projection is solved on the spot, so the message names
    the actual type); a non-conforming payload is the ordinary E107.
- **Dispatchability** (`kestrel-semantics/src/existential.rs`):
  `RequirementDispatch` (static / initializer, generic, `Self` outside the
  receiver, or the associated types it needs) and `ExistentialLayout`
  (own requirements in declaration order + parents). Declaration-only
  queries; MIR reads the same layout for slot order in M4.
- **Member lookup.** `peel_for_member`: on `B[rawany P]`, a requirement of
  `P` is checked (E145, E148) and the box is peeled eagerly, before the
  box's own members. Extension methods, and every member an operator or
  `for-in` reaches, resolve with the **box** as `Self` through
  self-conformance (`self_conformance_member`). The resolver treats
  `RawExistential` like `Opaque` (bounds) for members and declared
  conformance.
- **Self-conformance** (`entail_layer.rs::existential_supply`): holds for
  a protocol in the bounds' declared-refinement closure when every
  requirement is dispatchable and the box grants `MutableIndirection` /
  `ConsumingIndirection` as needed; a protocol reached by a blanket
  `extend Q: R` (`Iterator → Iterable`) holds through `Q`. E146 names the
  blocking requirement. `reify_kind` / `solver_ety` now keep a raw
  existential's bounds instead of erasing it to `Error`.
- **Associated types.** A projection on `rawany P[A = X]` (by entity or
  name) or on its box (by entity only, so `Indirection.Target` is never
  shadowed) returns the binding.
- **New builtins:** `MutableIndirection`, `OwnedBox` (for the E148 hint).
- **Not done here:** the `Protocol`-is-`Copyable`/`Static` arms stay. A
  bare protocol is no longer a value type (E144), and the arms still serve
  `Protocol` kinds in bound and `Self` positions, so deleting them buys
  nothing. Mutation through `any[CowBox] P` waits for M4
  (`CowBox: MutableIndirection` is sized-only until a payload can be
  cloned through its table).
- **Open:** whether an existential converts implicitly into a different
  box via `any P: P` (`nested_existential_exec`) or that is E147
  (`any_box_mismatch`). The two tests disagree; E147 is what's built.

### Phases 9–11: runtime (M4) — done

As built. The plan's `VTableKey`/`VTableDef`/`Callee::Dynamic` items became
templates plus ordinary loads and thin calls, so mono and codegen gained no
new call kind:

- **MIR type.** `MirTy::RawExistential { bounds, bindings }`, lowered from
  `HirTy`/`ResolvedTy`. A raw existential value is a borrowed view: copy is
  `None`, never dropped.
- **Fat values.** One rule, `ty::is_unsized` / `ty::is_fat` over a
  `TailLookup`: a pointer or reference to an unsized type, or a borrowed
  raw existential, is two words (data, table). Generic layout uses
  `GenericTails`; post-mono `MonoModule::is_fat` uses `MonoTails`. Codegen
  treats a fat value as a 16-byte aggregate. A struct with an unsized last
  field (`RcBoxStorage[rawany P]`) gets `StructLayout::unsized_tail` and a
  prefix-only layout.
- **Tables** (`kestrel-mir/src/item/vtable.rs`). Header: drop thunk (also
  the type's identity for `as?`), size, align, clone (filled only when the
  bounds refine `Cloneable`). A single-protocol table then holds one table
  pointer per declared ancestor (canonical entity order), then its own
  dispatchable requirements in `ExistentialLayout` order. A composition
  table holds the header and one table pointer per bound. Templates are
  built in mir-lower, generic over the payload; a `VTable { template,
  payload }` literal is instantiated by mono collection (`scan_vtable`),
  interned per `(template, payload)` (`mono/vtable.rs`), and emitted as a
  read-only global with relocations by both backends.
- **Thunks** (`kestrel-mir-lower/src/existential.rs`): one per requirement,
  generic over the protocol entity as `Self`. Borrow / `mutating` receivers
  view the payload in place; a `consuming` receiver takes it. Drop and clone
  thunks are generic over a synthetic `Payload`.
- **Conversions** (`body/existential.rs`), from `existential_conversions`:
  Box (`ExistentialBox.init` into a slot), Unsize (rebuild the box's
  pointer as `(data, table literal)`), Upcast (replace the table via the
  ancestor / bound words). Upcast to a composition is not supported.
- **Dynamic calls** (`emit_dynamic_call`): load the table, hop through
  ancestor / bound words, load the slot, `Callee::Thin`. A `consuming`
  requirement takes the box by value, views it through `pointeeRef`, and
  frees it afterwards with `consumePointee` and a no-op body.
- **Fat legalization** (`mono/fat.rs`, end of mono): pointer casts and
  queries on fat pointers, `sizeof_val` / `alignof_val` (table, plus the
  prefix for an unsized-tail struct), `ptr_with_meta`, `destroy_addr` of an
  unsized pointee (table drop), and `clone_val_into` (table clone, or the
  clone thunk at a sized type).
- **Self-conformance witnesses.** For each `B[rawany P]` in the module,
  each protocol in the bounds' closure without a blocker gets a witness
  whose requirements peel the box and call through the table; extension
  defaults bind directly.
- **Stdlib.** `lang.sizeof_val`, `lang.alignof_val`, `lang.ptr_with_meta`,
  `lang.clone_val_into`; `Pointer.withMetadata(of:)`; `RcBox.valuePtr`
  carries the table; `RcBox.forkStorage()` clones any payload into fresh
  storage, so `CowBox: MutableIndirection` no longer needs `T: Sized`.
- **Tests:** every `types/existential/*_exec.ks` except the downcast / `is`
  tests and `nested_existential_exec` (open question in M3) passes on
  Cranelift and LLVM.
- **Not done:** MIR snapshot tests for table layout; post-mono table
  verification; type descriptors as separate statics (the drop-thunk
  address is the identity instead).

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

### Phases 13–14: downcasts, `is`, docs (M5) — done

As built. The `is` pattern is the one primitive; the expression forms are
hir-lower desugarings onto it (`kestrel-hir-lower/src/existential.rs`), so
typing, borrows and exhaustiveness are the ordinary match machinery:

- **HIR.** `HirPat::TypeTest { binding, target, sub }` (`c is T { .. }`;
  `binding` borrows the payload, `sub` is a struct pattern or `_`) and
  `HirExpr::TakePayload { expr, target }`. `x is T` → `match x { is T =>
  true, _ => false }` (swapped for `is not`), so it types exactly like `true`.
  `x as? T` → a match ascribed `Optional[&T]`. `consuming x as? T` →
  `{ let $boxed = x; let $result: T throws _ = if $boxed is T {
  .Ok(TakePayload($boxed)) } else { .Err($boxed) }; $result }`. The
  annotations are built as `AstType`s and lowered like user sugar. E155 is
  retired.
- **Typing** (`generate.rs`): the sub-pattern is typed at `T`, the binding
  at `&T`; the scrutinee's type is recorded in `TypedBody::type_tests`.
- **Diagnostics** (`kestrel-analyze/src/body/type_test.rs`): E152 (not an
  existential, or a generic parameter), E149 warning (via
  `kestrel_type_infer::type_test_blocker`, shared with MIR), and E148 for a
  `consuming as?` on a box that isn't `ConsumingIndirection`.
- **Exhaustiveness** (`kestrel-pattern-matching`): `Constructor::TypeTest`
  over an open domain (`TypeShape::Infinite` for a box over a raw
  existential) and `PathElement::Payload`.
- **Lowering** (`kestrel-mir-lower`): the type identity is the **table
  pointer** — tables are interned per (shape, payload), so the box's table
  equals the table literal at `T` exactly when the payload is a `T`; no
  separate type descriptor. An unmatchable target (E149) folds to `false`.
  An `is` switch is a sequential test chain shared with string literals
  (`emit_test_chain`); type tests always lower in place mode, so they never
  move the scrutinee. `TakePayload` calls `consumePointee` with the
  synthetic `__existential.take[S, T]`.
- **Fixed on the way:** the test chain carried the pre-chain live set into
  later tests (a later miss forwarded the stale scrutinee; a later hit
  destroyed the rebound one). Struct patterns with a literal field
  (`Rect { w: 0, .. }`) lowered the positional field as a tuple extract and
  failed codegen — new test `patterns/pattern_types/struct_pattern_with_literal_exec.ks`.
- **Test corrections:** `is_operator_exec.ks` used `!` on `Bool` (Kestrel's
  logical negation is `not`) and a trailing closure for `filter` (which
  needs `where:`).
- **Docs:** `docs/language/existentials.md`, the type-test section of
  `pattern-matching.md`, cross-references in `opaque-types.md` and
  `design/shared-box.md`, `error-codes.md`, the crate architecture docs, and
  the `write-kestrel` skill.
- **Not done:** `OwnedBox: Cloneable` at an unsized payload (clone through
  the table, design §3) — `OwnedBox`'s `Cloneable` is still `T: Sized`.
  `is any Q` / `as? any Q` stay out of scope. Open: `nested_existential_exec`
  (see M3).

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
