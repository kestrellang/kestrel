# Closure Kinds Implementation Plan

**Design:** [closures.md](../../design/closures.md) ·
[shared-box.md](../../design/shared-box.md) ·
[closures-stdlib-audit.md](../../design/closures-stdlib-audit.md)
**Method:** TDD — the full test matrix (Phase 0, ~100 files) is baselined red
before implementation begins.
**Revision 2** — reworked after two adversarial reviews; every blocker below
is incorporated. Rev-1 mistakes worth remembering are marked ⚠.

This plan does not restate the design. It records the *implementation*
architecture, the phase order, the lockstep constraints, and the decisions
taken where the design is silent.

## Architecture decisions

### D1. The kind enum and where it lives

```rust
// kestrel-ast/src/ast_type.rs, next to ParamConvention (same rationale)
pub enum FnTypeKind { #[default] Normal, Mutating, Consuming, Escaping }
```

⚠ Rev-1 had a fifth `Bare` variant for named-function values; review showed a
wildcard kind that survives unification poisons merge points (`let x = c ?
namedFn : escapingClosure;` would type `x` Bare → wrong copy class → bit-copied
handle). Instead, **bareness is a TyVar-level flex set**, not a kind:
`InferCtx.kind_flex: HashSet<TyVar>` (the `closure_flex` precedent), populated
where `ctx.function(...)` types named `Def`s and enum-case constructors. A
flex side always *adopts* the concrete side's kind (slot rewrite) in unify and
coerce; each `HirExpr::Def` use gets a fresh instantiation TyVar, so in-place
adoption never retroactively changes another use site. MIR keeps modeling
bare values as `FuncThin` + empty-`ApplyPartial` coercion.

Threading map (every layer gains a `kind` field):

| layer | type | construction sites |
|---|---|---|
| parser | `TyVariant::Function` (+kind slot) | `ty_parser` paren_types branch |
| CST | kind token emitted as **direct child of `TyFunction`, before `TyList`** | `emit_function_type` |
| AST | `AstType::Function { kind, .. }` | ast-builder `ast_type_from_cst` (single site) |
| HIR | `HirTy::Function { kind, .. }` | hir-lower `lower_ast_type` + reject_ref_types rebuild |
| type-infer | `TyKind::Function { kind, .. }` / `ResolvedTy::Function { kind, .. }` | `ctx.function` (flex), `ctx.function_conv(+kind)` — 7 call sites |
| MIR | `MirTy::FuncThick { kind, .. }` | mir-lower `ty.rs` ×3 paths |

The CST placement is deliberate: the ast-builder pairs bare `Mutating` tokens
inside `TyList` with the *following* param type by positional scan; a kind
token must sit outside `TyList` or it is misattributed as a param convention.

### D2. Grammar

`fn_type ::= ['mutating' | 'consuming' | 'escaping'] '(' params ')' '->' ty`

- `mutating` / `consuming` are already hard keywords — no lexer change.
  `escaping` is a **contextual keyword** via `contextual_keyword("escaping")`
  (the `ref` accessor-clause precedent) — never reserved globally.
- The paren-group/tuple ambiguity: a kind prefix commits only when `->`
  follows; a kind keyword before a tuple/grouping is a parse **error**, never
  silently dropped.
- ⚠ **The `elem` parser hazard** (review): the per-param parser at
  ty/mod.rs:180-188 greedily consumes `Mutating` via `or_not()` with no
  backtracking, so `(mutating () -> ())` silently parses as a *grouping* that
  drops the marker, and `(mutating (T) -> R) -> U` parses as a
  MutBorrow-convention param of a *normal* fn type. Fix in Phase A: `elem`
  attempts the full `ty` (which now includes kind prefixes) FIRST, falling
  back to `Mutating? + ty`; parse tests pin `(mutating () -> ())`,
  `(mutating (T) -> R) -> U`, `(escaping () -> ()) -> U`.
  `memory_model/mutating_closures/**` (9 files) is the mandatory green gate
  at the end of Phase A.
- `func f(consuming g: consuming () -> ())` must parse (access mode + kind
  adjacent). Kind-on-literal syntax is not added. `(consuming T) -> U` param
  conventions inside fn types remain unsupported (out of scope).

### D3. Inference: kinds in unify/coerce + the coercion side-table

- `unify` gains **kind equality** for `Function × Function` (the `Ref`
  precedent), with the flex-adoption rule from D1 (flex side adopts, never
  wins).
- The directional passing table lives in **`solve_coerce`** (+
  `reconcile_fn_convention` for calls-through-values): when both sides are
  resolved Functions, check kind table + conventions, unify params/ret
  pairwise, return Solved without structurally unifying the two fn types.
- Closure literals build at `Normal`; the expected type retrofits the kind in
  place (`set_function_kind`, gated on `closure_literal_exprs`). The kind
  check is a whole-type scalar check placed **before** the per-param loop and
  **above** the `closure_flex`/`closure_it` early-return arm. ⚠ The gate must
  **unwrap `HirExpr::Sugar`/`Block` wrappers** to find the literal id, or
  trailing-closure call sites silently miss the retrofit (spurious
  E624/E603); a Phase-0 test uses trailing-closure syntax against an
  `escaping` parameter.
- **`kind_coercions: HashMap<HirExprId, (FnTypeKind, FnTypeKind)>`** on
  `InferCtx`, surfaced on `TypedBody` (the `IndirectionPeel`/`promotions`
  shape). ⚠ Rev-1 had no channel telling MIR a conversion happened —
  mir-lower reads `expr_types` (the *source* type) and would silently store a
  2-word value into a wider slot. Every accepted cross-kind cell records an
  entry; mir-lower replays it (adapter/truncation/widening, D5).
- Body-implied kinds select diagnostics only; no silent rebuild.
- Kind participates in `ResolvedTy` equality (witness matching kind-exact).
  **Phase B requires no stdlib change**: the only closure-typed protocol
  *requirements* in lang/std are `Coalesce.coalesce`, `And.logicalAnd`,
  `Or.logicalOr` — all staying normal; everything the audit changes is an
  extension default, concrete init, or free method. No kind-lenient interim
  matching is needed. (⚠ Rev-1 claimed the opposite.)
- `kind_to_tyvar_sub` and every `function_conv` caller preserve the kind
  through generic instantiation (the `conventions.clone()` precedent).

### D4. Capture modes

`CaptureKind` stays `Read | Write`; the closure's kind (from
`TypedBody.expr_types`, settled before `build_result`) decides capture mode:

- **View tier (normal / mutating):** every captured place is captured by
  address; body reads load through the pointer **at each use** (bind captured
  locals the way MutBorrow params are bound), so reentrant writes are
  observed. Var-locals and addressable lets alias their slot; non-addressable
  **bitwise-copyable** lets materialize once (observationally identical —
  immutable binding). ⚠ **View materialization is bitwise-only**: Cloneable
  and non-Copyable captures must alias the source's storage (anchoring the
  source into an addressable temp if needed), never `clone()` — a view env
  owns nothing and `needs_drop == false`, so a clone would leak. Non-Copyable
  lets alias the original slot **without `Take`** and no move is recorded.
- **Owning tier (consuming / escaping):** per place at creation: bit-copy
  Copyable, `clone()` Cloneable, move non-Copyable (E500 thereafter). Reject:
  values carrying frame provenance (view closures etc.); non-owned
  non-Copyables; ⚠ **any capture whose type has `escape_carry(ty).any_ref`**
  (a `&param`-derived reference is Copyable with `Param` provenance — it
  passed rev-1's filter and produced a returnable closure holding a dangling
  ref); **protocol-`Self`-typed captures** (no owning mechanism exists —
  explicit E-kind capture error in v1; audit confirms no stdlib lazy builder
  needs one).
- Static-eligibility of an owning closure is conditional on all captures
  being Static (not unconditional).
- **v1 limitation (pinned by test):** nested-closure captures still collapse
  to whole-local (`force_whole` TODO in captures.rs) — transitive
  narrowest-place capture through nested closures is a named follow-up.

### D5. MIR representation, conversions, and type-erased clone/drop

The design is silent on type-erased clone/drop (an `escaping () -> Int64`
param doesn't name its env type). Decisions:

- **Layouts.** View kinds and bare values: 2 words `{fn, env_ptr}` (status
  quo). Owning kinds: **4 words `{fn, env_handle, retain_fn, release_fn}`**;
  the shim pointers are per-`E` monomorphized functions whose bodies are
  ordinary MIR calling the box API (`clone()` / drop) — refcount semantics
  stay in Kestrel; only the dispatch (null-check + load + indirect call) is
  compiler-emitted. Capture-free values at owning kinds carry
  `env_handle == null`, never allocate; erased ops null-check first.
  `compile_thick_call` reads `{fn@0, env@ptr}` for every kind — call paths
  unchanged.
- **`InstKind::ApplyPartial` grows `retain: Option<Callee>,
  release: Option<Callee>`** — mono `scan_callee`/`rewrite_callee`, the
  thunk-pass rewrite, and both `compile_apply_partial`s must handle them
  (they hard-error on unresolved callees; these are new mono roots).
- **Representation conversions** (driven by `kind_coercions`, D3) — each
  needs an explicit MIR shape:
  1. bare → owning: widen 2→4 words, null handle, null/nop shims;
  2. escaping → normal: **truncating view** `{fn, handle-as-ptr}`; the value
     is frame-bound, rooted at the source handle's local — the freeze rule
     covers the handle like any viewed place. ⚠ When the source is a
     borrowed *param*, the truncated view must NOT be returnable:
     `check_escapes` rejects closure-carrier returns whose root is a
     borrow-param **when the return slot's kind is a view kind** (an owned
     escaping return rooted at a param is a retained copy and stays legal);
  3. normal → mutating: same 2-word views, exclusive-call type only;
  4. escaping → consuming: one-shot adapter owning one retained handle.
- **Consuming tier: unique heap env, NOT the shared box.** ⚠ Rev-1 reused
  RcBox with count pinned at 1; its release runs `dropInPlace` on the whole
  env, double-freeing capture slots the one-shot body moved out. Instead: a
  unique heap allocation; the **call consumes the closure value and
  transfers env ownership to the callee**, whose body has static knowledge
  of `E` — per-slot drop flags in the body frame, per-slot destroys + free
  on every exit (existing partial-move machinery). The caller emits no
  DestroyValue after the call. Drop-without-call releases via `release_fn`
  (drops all slots + frees).
  - **AS SHIPPED (E2b)**, with one deliberate deviation. The container is
    `@builtin(.UniqueBox)` → `std.memory.UniqueBox[E]`: one allocation
    holding a liveness word plus the payload, `takeValue()` (move the
    payload out, mark the block empty) and `destroy()` (drop the payload if
    still present, then free) — no refcount, no `deinit`. The call
    function's prologue `takeValue()`s `E` out and **`DestructureStruct`s it
    into per-capture @owned locals in the body frame**, which is how "per-slot
    drop flags in the body frame" is realized: each capture becomes an
    ordinary local, so moving one out while the rest drop is the existing
    machinery rather than a new one.
  - **Deviation:** the *block* is reclaimed by `release_fn` (`destroy()`),
    which the CALLER emits right after the call — not by the callee. The
    drop-without-call path emits the same `release_fn` at scope exit, where
    it still finds the payload and drops it, so each allocation is freed
    exactly once either way. That one uniform rule is what makes conversion
    4 (`escaping` → `consuming`) work with **no synthesized adapter**: the
    coerced value already *is* "a unique one-shot value owning ONE retained
    shared handle" (the ordinary Cloneable arg-copy supplies the retain),
    and the caller's post-call release is the one that gives it back. A
    callee-frees protocol cannot do this — word 0 of a coerced value is the
    *escaping* call function, which never frees, so it would leak.
  - Layout: `Consuming` joins `FnKind::is_boxed()` and reuses E2a's 4-word
    owning shape unchanged (`func_thick_words` stays the single source,
    lockstep 9; `compile_thick_call` untouched). The new
    `FnKind::is_shared()` (Escaping only) splits "boxed" from "duplicable":
    `is_boxed` drives layout/drop/erased dispatch, `is_shared` drives
    Cloneable classification and the retain path. A `consuming` value's
    `retain` word is the shared NO-OP shim and is unreachable by
    construction (`not Copyable`), guarded by a `debug_assert!` in
    `expand.rs`'s CopyValue arm.
- **Escaping env parameter ownership.** ⚠ The env param must be
  **borrowed/guaranteed** (or a raw storage pointer), never
  `Consuming`/owned — an owned handle param would release once per call and
  free a multi-call environment after the first call. Projection inside the
  body goes through `sharedMutRef` semantics (borrow, never consume). A
  Phase-0 test calls one escaping closure 3× and asserts shared state
  survives; refcount stays 1.
- Provenance stamp in `emit_apply_partial`, gated on kind: view kinds join
  `Local(cap)` per capture (unchanged). ⚠ Owning kinds: **filter out
  self-rooted captures** (`value(c).root == Local(c)` — an owned snapshot
  contributes nothing) and join only the survivors; no survivors →
  `Static`. (Rev-1's "join captures' own roots" could never return: a fresh
  copy's root is `Local(copy_id)`, still frame-bound, and `join` can never
  rank below `Local`.)
- `copy_propagation` verification item: eliding a CopyValue+DestroyValue
  pair on a shared handle is sound only under the pass's only-remaining-use
  precondition — re-verify for owning FuncThick before enabling.
- Mangling, witness `match_pattern`, `substitute`, layout,
  `collect_named_type_from_ty` (add FuncThick recursion), and both codegens
  gain kind arms; kind is part of the mangled symbol.

### D6. Copy/drop policy

Per-layer `member_semantics` arms (kernel untouched):

| layer / site | normal | mutating | consuming | escaping | bare/thin |
|---|---|---|---|---|---|
| HirCopyLayer, SolverCopyLayer, MoveCopyLayer | Copyable | NotCopyable | NotCopyable | Cloneable | Copyable |
| MIR `copy_behavior` | Bitwise | None | None | `Clone(cloneable_proto)` | Bitwise |
| MIR `needs_drop` | false | false | **true** | **true** | false |

- ⚠ **No frontend Cloneable promotion.** Rev-1 added an escaping-closure arm
  to `nominal_copy_semantics_impl`; both reviewers rejected it — the
  frontend deliberately promotes only via declared conformance (a `String`
  field doesn't promote either), and a closure-only exception is a
  single-source-of-truth violation that also breaks `lower_copy_behavior`
  (base `Clone(entity)` with no clone impl) and risks the conditional-
  container debug_assert. The route is the `String`-field route:
  `hir_type_copy_semantics` answers NotCopyable for mutating/consuming fn
  types (NotCopyable propagation), and **`ty_needs_clone_shim` gains a
  FuncThick-escaping arm** so the existing shim synthesis +
  `type_info.copy = Clone(cloneable_proto)` overwrite fires in MIR.
- The four "carries resources" predicates + two more change as ONE atomic
  edit (lockstep 1): `ty_query.rs copy_behavior` + `needs_drop`,
  `expand.rs ty_needs_drop`, `audit.rs mono_needs_drop`,
  `clone_shim.rs ty_needs_clone_shim`, `drop_fix.rs field_needs_drop`
  (via needs_drop), `mono/mod.rs concrete_copy` (kill the `_ => Bitwise`
  catch-all for FuncThick).
- `expand.rs` CopyValue/DestroyValue gain owning-FuncThick arms emitting the
  erased retain/release dispatch; the legacy `closure_captures` teardown
  stays for view kinds only and is gated off for owning kinds in the same
  commit.
- Verify assertion: extend `mono/verify.rs verify_copyable_containment`
  (Inv-3b) — no Bitwise/Clone container holds a NotCopyable-kind closure
  field; no Bitwise closure repr owns a droppable env.
- Staticness: view-kind closures are not Static; owning closures are Static
  iff all captures are (D4). Separate kernel, separate arms.

### D7. Escape provenance

- `escape_carry`/`contains_closure` stay type-driven; owning-kind returns
  pass via self-rooted values (the D5 stamp). `carry_ref_taint` on copies of
  owning closures preserves self-rootedness.
- View kinds keep E494 exactly as today; message gains a fix-it **note**
  ("consider `escaping (…) -> …` or `consuming (…) -> …`").
- The truncated-view return rejection from D5(2) lands in `check_escapes`'s
  closure-carrier branch.
- Phase E verifies the stamp by execution tests (returned escaping closure
  runs; view closure return still E494) — 44 testdata files reference E494;
  the whole set re-baselines in Phase E.

### D8. Freeze rule and diagnostics

Move-checker implementation, **place-granular** (`HashSet<PlaceKey>` +
`is_prefix_of` overlap — the design sells place-based capture; a
LocalId-granular freeze would freeze all of `self` when `{ self.data }`
captures one field; ⚠ rev-1 chose coarse).

- `State.frozen: HashMap<PlaceKey, FreezeInfo>` populated by the
  `HirExpr::Closure` arm for view-kind closures; consulted at the top of
  `record_move` AND in the `HirStmt::Deinit` arm (which bypasses
  `record_move`).
- ⚠ **Scope-depth rule, not just move sites** (rev-1's freeze missed silent
  scope-exit dangling: `g = { r.v }` inside a block, `r` dies at block exit,
  `g()` after = use-after-free with no diagnostic). `FreezeInfo` records the
  captured place's scope depth; every propagation point (Let/Assign/
  aggregate-store/call-arg/Return) rejects storing a view-carrying value
  into a binding whose scope is **shallower** than the captured place
  (E507: "a closure viewing `r` cannot outlive `r`"). Block exit
  restores the frozen set after that check.
- Propagation through copies/aggregates/params/joins unions frozen sets;
  joins union; loop back-edge re-analysis threads a **separate**
  freeze-reported set (the shared one-per-local set would collide with
  E500/E503/E506).
- **E507** `freeze_violation` (next free E5xx): E498-modeled wording,
  variants for move / consuming-arg / deinit / outlives-scope.
- ⚠ **E624 splits into three homes** (rev-1 overloaded one code):
  - **E624** `closure_kind_mismatch` — the passing-table rejection only.
    `InferError::KindMismatch { span, expected, actual }` mirrored across
    the five mandated files, real span, coerce-path reporting.
  - **E625** `closure_kind_convention_pairing` — a `mutating`-kind param
    must be on a `mutating` convention, `consuming`-kind on `consuming`.
    Signature-level fact ⇒ **DeclCheck analyzer** (a bodiless decl never
    generates a Coerce). Phase-0 tests asserting E624 for pairing are
    updated to E625.
  - Non-exclusive `mutating` call (calling a mutating-kind closure held in
    `let`) routes through the existing **mutability band** (`classify_
    mutability`: calling a mutating-kind closure is a mutating use of the
    callee binding → the E203 family). Phase-0 tests asserting E624 there
    are updated accordingly.
- **E500 extension sites named**: the `HirExpr::Call` arm records a move of
  the callee local when the callee's resolved type is consuming-kind, and
  MIR `lower_indirect_call` consumes the callee value for that kind
  (otherwise the second-call test ICEs in OSSA instead of E500).
- E603 kept for normal bodies + fix-it note; lifted for mutating/escaping.
  E506 kept for normal/mutating/escaping; lifted in consuming bodies —
  with a replacement guard: a consuming body moving a *non-owned* capture
  still errors (not an ICE).
- E212 retires only in the phase that makes ref-binding captures work as
  view slots (lockstep 6). Ledger + docs/error-codes.md updated with
  E507/E624/E625 in the same commits.

### D9. SharedBox / RcBox / lang item

- `@lang(sharedBox)` = **`Builtin::SharedBox`**: three arms in
  `kestrel-hir/src/builtin.rs`; `name()` uses the non-resolvable sentinel
  `"SharedBox"` (never `"RcBox"` — the name-based fast path would resolve
  RcBox without the attribute and defeat swappability). One builtin for v1.
- Missing/duplicate builtin is silent today → add a compilation check:
  escaping closure present + builtin unresolved = real diagnostic.
- **Swappability of the Direct-call lowering**: the callee's receiver
  *entity* comes from `ResolveBuiltin{Builtin::SharedBox}`, members looked
  up by requirement name/label on that entity — never a hard-coded RcBox
  reference. (Design rule 2 is about what the lowering *names*, and it
  names the binding.)
- `RcBox` gains `sharedMutRef() -> &mutating T` — non-mutating receiver is
  legal, but for a narrow reason that must not be refactored away: the
  `PointerDerived{mutable:true}` root exists only because every
  return-position expr is a direct `Pointer.mutatingValue`-style intrinsic
  call (`RetRefPointerDerived` is Callee::Direct-only, all-returns-must-
  qualify). **The body stays literally `self.valuePtr().mutatingValue`.**
  Also `isIdentical(to:)` (storage-pointer compare) — the conformance
  `extend RcBox[T]: SharedBox` lives **in rcbox.ks** (it reads the private
  `ptr` field). Do not touch `pointeeMutRef`'s `mutating` (E459); do not
  redeclare `type Target` (E462; inherited binding resolves through
  extensions). Protocol init labels must match `RcBox.init(consuming
  value:)` **exactly** (witness matching is label-exact; a mismatch is a
  silent no-binding + post-mono codegen error).
- Phase-0 validation tests: `witness_nonmutating_mut_ref_return.ks`
  (exists) and a new one for a protocol init requirement whose param type
  is an **inherited associated type** (`P: Q`, `Q.Target`, `P` requires
  `init(consuming value: Target)`) — shared-box.md itself names
  `static func create(consuming:)` as the fallback if that cannot pass.
- CowBox generalization **deferred** (no v1 client; indirection peel
  doesn't work on bare type params, so `CowBox[T, B: SharedBox]` would
  lose `box.field` sugar internally). Protocol + RcBox conformance ship.
- Compiler-side constraint checks (pointer-only handle layout;
  Cloneable-never-Copyable via `NominalCopySemantics`'s explicit-Cloneable
  distinction; no self-dependence) as a DeclCheck modeled on
  `builtin_marker_protocol.rs` (E419), not on the dead E502 stub.

### D10. Stdlib sweep (audit checklist)

As specified in the audit, plus survey extras:

- 4 closure-copy-out-of-field sites become `.clone()` (collections/views.ks
  :397; text/views.ks :3681, :3736, :3740). `ArraySplitWhereView.count`/
  `.toArray()` are the concrete `sharedMutRef` consumers.
- **Intra-G atomicity** (replaces rev-1's wrong lockstep 5): each
  adapter/view *family* changes as one unit — field ↔ initializer(s) ↔ lazy
  builder ↔ copy-out sites (iter/adapters.ks 9×3; collections/views.ks
  fields/inits + slice.ks:1028 + the clone at :397; text/views.ks
  fields/inits + str.ks:379 + clones at :3681/:3736/:3740).
- The 9 adapter structs stay non-Copyable via the inner iterator
  (NotCopyable dominates the fold — verify per-instantiation).
- The 5 mutating-API conversions keep receiver conventions (`tryForEach`
  stays `mutating func`) and pair `mutating` convention with `mutating`
  kind. `sort(byKey:)`-style wrapper literals stay normal.
- lang/perch has 2 stored closure fields needing `escaping` later — out of
  scope here.

## Phase order

- **Phase 0 — TDD matrix** (dirs: `expressions/closures/kinds/`,
  `memory_model/closure_kinds/{view,mutating,consuming,escaping}/`,
  `stdlib/shared_box/`, `stdlib/closure_kinds/`, plus the two
  `references/witnesses/` validation tests): DONE — 94 files + supplements
  from the review (dangling-scope-exit counterexample, param-truncated-view
  return rejection, `&param` owning-capture rejection, 3×-call refcount,
  trailing-closure retrofit, the three D2 parse pins, inherited-assoc-type
  init witness, nested-closure force_whole pin; E625/E203 annotation
  updates). Baseline red via targeted triage.
- **Phase A — parser/CST/AST/HIR threading** (D1, D2), including the `elem`
  reorder. Green gate: parse tests + `memory_model/mutating_closures/**`.
- **Phase B — type-infer** (D3): kind field, unify equality + kind_flex,
  coerce table, literal retrofit (wrapper-unwrapping gate), kind_coercions
  side-table, E624, renderers (6 sites incl. kestrel-doc `ty()`).
  No stdlib change required.
- **Phase C — SharedBox protocol + RcBox conformance + builtin + checks**
  (D9). May overlap A/B (disjoint files), but its lang/std edits invalidate
  the triage cache — batch its triage with the next phase's run.
- **Phase D-front — front-end copy classification** (HirCopyLayer,
  SolverCopyLayer, MoveCopyLayer arms). Safe early; pure classification.
- **Phase E — MIR tiers + D-mir + front-end move-checker capture change**
  (D4, D5, D7 + the MIR half of D6). One commit covers: view-tier rework
  (address captures, at-use loads, no-Take) **with** move_tracking's
  view-capture non-move change (#177 lockstep); owning tier (unique env /
  boxed env, shims, erased dispatch, ApplyPartial extension); provenance
  stamp; conversions/adapters; thunk-pass keying on FunctionKind; the
  atomic predicate edit (lockstep 1); expand arms + legacy-teardown gating
  (lockstep 3); layout/codegen widening (lockstep 9); Inv-3b; both
  backends. Existing-test flips caused here (view semantics, rcbox
  refcount, E494 set re-baseline) are updated here.
- **Phase F — freeze rule + diagnostic lifts + E212 retirement** (D8).
  After E (needs ref-binding view captures + kind info). Not concurrent
  with E.
- **Phase G — stdlib sweep** (D10) + its test flips (forEach/terminal_
  operations, inspect call sites, split-view tests).
- **Phase H — verification + docs**: full triage green; **second full run
  with `KESTREL_BACKEND=llvm`** (forced re-run); fmt/clippy; docs
  (docs/language, memory-model supersession, error-codes.md,
  architecture.md, write-kestrel skill); **regenerate stdlib docs via the
  `stdlib-docs` skill** (24 public signatures changed).

## Existing tests the design flips — by phase

| phase | files |
|---|---|
| E | `capture_by_value_semantics.ks` (→ view), `stdlib/rcbox/implicit_clone_refcount.ks` §4/§5, `stdlib/rcbox/clone_field_capture_noncopyable_receiver.ks` (comment), `use_after_capture_noncopyable.ks` (→ legal), the **23** `ERROR(E494)` closure files (positive `escaping` siblings; keep normal-kind rejections), 44 E494-referencing files re-baselined, `references/static_bound/tuple_and_function_types_are_static.ks` (Static predicate), `references/ret_borrow/closure_tail_ref_decay.ks`, `coalesce_rhs_ref_decay.ks`, `memory_model/copy_semantics/**` + `memory_model/deinit/**` watch-list (clone shims may appear) |
| F | E212 trio, `cannot_mutate_captured_variable.ks` (message+note), `move_captured_noncopyable_out_of_closure.ks` (+consuming sibling) |
| G | `stdlib/iterator/terminal_operations.ks` (→ execution), `stdlib/optional/optional_extended.ks:30`, `stdlib/iterator/intersperse_with_adapter.ks:28` |
| E (confirmed hole) | `expressions/closures/capture_from_nested_scope.ks` — verified 2026-08-07: passes with ZERO diagnostics; closure provenance is lost at the if/else CFG merge (block params self-root, defeating the taint). Pre-existing unsoundness (dangling stack env if called). Phase E's provenance work must propagate closure roots through block params; annotate the test with E494 once the anchor line is known. |

## Lockstep constraints (single-commit units)

1. The six resource predicates: `copy_behavior` ↔ `needs_drop` ↔
   `expand.rs ty_needs_drop` ↔ `audit.rs mono_needs_drop` ↔
   `clone_shim.rs ty_needs_clone_shim` ↔ `concrete_copy` catch-all
   (+ `field_needs_drop` via needs_drop).
2. Front-end capture-move removal ↔ MIR no-Take view captures (#177) —
   both inside Phase E.
3. Owning expand CopyValue/DestroyValue arms ↔ kind-gating of the legacy
   closure_captures teardown.
4. Kind in `MirTy::FuncThick` ↔ mangling ↔ witness match_pattern ↔ collect.
5. *(deleted — Phase B does not break lang/std; replaced by intra-G family
   atomicity, see D10)*
6. E212 retirement ↔ ref-binding view-capture lowering.
7. Owning-kind provenance stamp ↔ owning-tier lowering (stamp is gated on
   kind; view kinds keep today's stamp throughout).
8. Escaping Cloneable classification ↔ `ty_needs_clone_shim` arm ↔ Inv-3b.
9. Owning-kind 4-word layout: `passes/layout.rs` ↔ cranelift `ty.rs` ↔
   llvm `ty.rs` ↔ both `compile_apply_partial` pair slots ↔ the FuncThick
   note in kestrel-codegen-llvm/AGENTS.md.

## Decisions taken where the design is silent (review these)

1. **4-word owning-closure layout** with inline per-env retain/release shim
   pointers (type-erasure gap). Alternative: 2-word + static descriptor.
2. **Consuming = unique heap env, callee-owned at the call**; caller-side
   drop via `release_fn` only when never called. *(E2b revised this: the
   callee still takes ownership of the environment at the call, but the
   caller emits `release_fn` after EVERY call to reclaim the emptied block —
   the uniform rule that makes `escaping → consuming` adapter-free. See D5.)*
3. **Bareness = TyVar flex set**, not a kind variant.
4. **`@lang(sharedBox)` = `Builtin::SharedBox`** on `@builtin` machinery.
5. Body-implied kinds select diagnostics only.
6. CowBox generalization deferred.
7. Fix-its are `notes` strings.
8. **E507** freeze, **E624** passing table, **E625** pairing (DeclCheck),
   non-exclusive mutating call via the E203 mutability family.
9. Owning capture of protocol `Self` and of ref-carrying values: rejected.
10. Freeze is **place-granular** with scope-depth outlives checking.
11. Nested-closure whole-local capture collapse stays (pinned limitation).
