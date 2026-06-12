# Stage 2 — Requirements (REDEFINED 2026-06-10: second-class composition)

> Supersedes the original "storable refs; default DON'T BUILD" framing.
> Maintainer-ratified 2026-06-10: the §8 lifetime-on-every-type design is
> **permanently rejected**; stage 2 is now the no-lifetime alternative —
> refs in enums/tuples and ref-capturing closures, contained by the
> `Static` bound (moved here from stage 3). Tradeoff record at the bottom.

## Goal API (maintainer-stated end state)

```kestrel
arr(i)              // &T — the unlabeled subscript becomes a place
arr(checked: i)     // Optional[&T]
dict(k)             // Optional[&V] — one probe
dict(unwrap: k)     // &V
arr.map { it * r }  // closures capture references
for x in arr.refs() // ref-yielding iteration (surface spelling TBD)
struct RefIter[T] { var buf: Pointer[T]; var item: &T; ... }
                    // iterators/cursors STORE refs — second-class structs
```

## The line that never moves

Refs — and ref-bearing values (enums, tuples, **structs**, closures) —
stay **second-class**: usable as locals, bindings, params, root-ruled
returns, payloads/fields, and closure captures; **never** storable in
heap/static/escaping positions (no `Array[Span]`-class storage, ever). Every
`&T` in a function context shares one uniform lifetime, so the type
system never needs `&'a T` vs `&'b T`: `MonoTypeKey` stays
`(Entity, Vec<TyId>)` — `(Optional, [&T])` and `(Optional, [T])` are
already distinct keys, so the §8 collapse class (shared drop shims across
referent lifetimes) cannot arise — and escape checking stays per-function
flow analysis. The cost center is structurally avoided, not deferred.

## Structure — Static bound FIRST (ordering inverted vs the old plan)

The moment `&T` is a legal type argument, generic code can store, return,
or capture it — containment must precede expressiveness:

- **2a. `Static` bound — ✅ IMPLEMENTED 2026-06-11** (commits 9da6b85c,
  167de3cb, 6a21a563 + the enforcement pair; suite 3260+ green with the
  bound live everywhere; compile-time flat). Shipped shape: staticness
  kernel in `kestrel-semantics/src/staticness.rs` (NominalStaticness with
  structural gating positions + cycle guard, TypeParamStaticRequirement
  with owner-negation folding, `hir_type_is_static`, `StaticLayer` +
  `instance_is_static` shared by solver TyVar / analyze ResolvedTy
  mirrors); solve_conforms Static-first branch + Ref-arm intercept;
  conforms_to permissive arm; type_satisfies intercept; wellformedness
  filter widened; DoesNotConform "because" details; injection in
  `inject_implicit_static_bounds` (extensions + intrinsics excluded);
  `Pointer[T]` gained `T: not Static` (Static regardless of T —
  position-gated arg recursion); E505 statics decl check; E212 widened
  to non-Static captures. Function types are Static throughout (the
  capture-derived bit is 2c, ratified). The bound delivers the
  containment as designed: `RcBox`/`Array` storage, statics, and
  escaping positions require `Static` — heap-container-outlives-referent
  is excluded **by the bound, not by analysis** — and zero-breakage held
  (every existing type IS Static until 2b/2c mint non-Static values).
  The relaxation spelling resolved to **reusing `not Static`**: on a
  param it means need-not (exactly `T: not Copyable` — `Pointer[T]`
  precedent), on a nominal it means is-not; there is no must-not on
  params and no `?Static` sigil. Gating positions are computed
  STRUCTURALLY inside `NominalStaticness` (which params appear in stored
  child types) — the declared-extension-scan `ConditionalStaticParams`
  analog dissolved into that. Static never reaches MIR (no runtime
  semantics) — three layers, not five.
- **2b. Refs in enums/tuples/STRUCTS — ✅ IMPLEMENTED 2026-06-11**
  (structs ratified IN 2026-06-10 — full Hylo remote parts). Shipped
  shape (10 commits, each full-suite green; final 3299+):
  - **Rulings**: refs are COPYABLE as payload/field values (bit-copy
    aliases, may-alias); NO Optional[&T]↔Optional[T] decay; both `&`
    and `&mutating` bits; machinery-only (no Array/Dict API adoption).
  - **Formation**: `RefPolicy{Strict,AllowAggregate}` two-mode walk in
    hir-lower — refs legal at Field/TupleElement/GenericArg + enum
    payloads; bare positions (E480/E482/E486/E487/E490) and Strict
    entries (alias RHS, protocol args, extension targets, where-clause
    types — a previously UNGUARDED gap, now closed) still reject.
    `.Some(&x)` borrow-args stay E488 — payloads are built from named
    bindings / ref-returning calls; construction needs a type-side pin
    (`let o: Optional[&T] = .Some(r)`), unpinned `.Some(r)` decays.
  - **Inference**: ref type args come from the TYPE side only.
    `Constraint::EqualDecayed` (arm results + tuple-literal elements,
    LOCAL reads only) and the deferred arm-2 Coerce decay + decay-
    defaulting pass let annotations win the race; `solver_copy_class`
    Ref→Copyable; ConformsOrigin{Expr,TypeArg} gate: a ref TYPE ARG
    satisfies only Copyable (other bounds → clean DoesNotConform with a
    "because" detail; `type_satisfies` Ref arm mirrors it) — ref-Item
    witnesses are 2d. The mir-lower peel seam split
    (`lower_resolved_ty_preserving`) keeps `(Optional,[&T])` ≠
    `(Optional,[T])`.
  - **Provenance**: wrap stamps the aggregate root = JOIN of ref-operand
    roots (`RootProvenance::join`, most-restrictive over escape/
    consuming/mutability); unwrap = @guaranteed pointee, borrow_source
    NONE (consuming the aggregate is harmless), rooted at the
    aggregate's root, joins the binding registry; taint carries through
    copies/moves, if/match merges, threading, caller-side call results,
    and (the G1 closure) VAR SLOTS — stores taint the slot monotonely,
    loads inherit, so `var o = .Some(&local); return o` is still E494
    while param-rooted cursor-in-var stays returnable. verify
    `check_escapes` gained the owned-return Carrier mode (E494/E495/
    E496 reused with carrier wordings; untainted `.None` returnable).
  - **Codegen** (both backends): ref slot = pointer scalar stored RAW
    (resolve_slot_value); extraction LOADS the slot only for
    POINTEE-typed results — Ref-typed results (clone/drop shim
    projections) keep the slot address (representation contract: owned
    Ref value = the pointer; @guaranteed Ref value = slot address);
    tuple layout from the RESULT type; ref-typed param args pass the
    address raw. NO new return ABI (tag+pointer rides Direct/Sret).
  - **Generic `-> T` at `T=&U`** (`unwrap()`, `identity[&U]`): mono
    ret_borrow derives from the DECLARED ret; the caller detects the
    instantiated-ref return from the declared `-> T` + type args and
    registers the result like a ret_borrow result.
  - **Stdlib**: Optional + the formation cascade (Result.T,
    ControlFlow.C, OptionalIterator.T, ResultIterator.T) relaxed
    `T: not Static`; compile-time A/B flat.
  - **Known gap**: protocol-dispatched OPERATORS at ref instantiations
    (`a == b` on `Optional[&Int64]`) bypass both gates (member dispatch
    never consults the extension's conformance clause) and surface as a
    post-mono "Callee::Witness not resolved" ICE — still REJECTED,
    wrong message (`references/composition/README.md`).
  - NOTE the accepted verify-level narrowing stands: a ref stored into
    an aggregate leaves SSA borrow tracking — soundness rests on
    type-level containment + scope rules, not per-value Check-4
    tracking (E498-class blind spots grow; G2 consume-after-packaging
    accepted; G1 var-laundering CLOSED via slot taint).
- **2c. Ref-capturing closures** (maintainer requirement 2026-06-10):
  capture classification = the contains-ref predicate applied to the
  capture list; E212 relaxes from a ban to "this capture makes the
  closure non-Static"; the by-ref capture lowering is mechanically ready
  (the `is_protocol_self` path in closure.rs — `BeginBorrowAddr` on an
  env field, no load/own/drop). **Two-tier model**: Static closures get
  the planned Rc upgrade (storable, copy = retain); ref-capturing
  closures stay stack-env, called-then-dropped, never Rc'd or stored.
- **2d. Ref-bearing protocols + for-in — ✅ IMPLEMENTED 2026-06-11**
  (8 commits ed8dd65a..; every commit full-suite green; final 3360+).
  The §9 prediction held: with 2b's honest ref types, witness refs
  dissolved into ordinary assoc-type machinery + targeted carve-outs —
  the whole arc needed ~5 small compiler changes:
  - **Formation carves**: `type Item = &T` / `&mutating T` legal on
    TRIVIAL member aliases (the assoc-binding shape; eager use-site
    expansion re-applies position rules, so `let x: Foo.Item` is E482
    exactly like a written `&T` — anti-smuggling free); where-clause
    EQUALITY RHS may be a ref (`where I.Item = &Int64`, the generic-
    algorithm spelling) via shared `reject_ref_types_allowing_top_ref`.
    Protocol assoc DEFAULTS, non-trivial aliases, protocol-bound args
    stay Strict.
  - **Witness seam**: assoc bindings/projections were ALREADY ref-clean
    (resolve→LowerTypeAnnotation→TyKind::Ref; mono `substitute` walks
    Ref) — concrete for-in over ref Items worked with ZERO mir/mono
    changes once formation landed (note: for-in's desugared
    `iter()`/`next()` ProtocolCalls dispatch Callee::Witness even on
    concrete receivers). Bare-ref requirements (`-> &Self.Item`) needed
    ONE arm: emit_call derives ret_ref_mutating for Callee::Witness
    from the protocol method's declared return (it was Direct-only —
    generic dispatch read pointer bits as the pointee). The
    witness-erasure hazard became the E458 exact-shape rule: ref-return
    shape AND mutability must match the requirement exactly (faithful
    Ref normalization in compare.rs replaced a stale stage-0.5
    debug_assert).
  - **Two inference fixes shaken out by the literal-defaulting race**:
    pattern-binder gate (a binder's type comes from its PATTERN, never
    its uses — use-site Coerces from a still-unresolved ImplicitPat
    binder defer while the gate is up; drops next to the AssignTarget
    stall-breaker) and solver-side STATIC formation wellformedness in
    `lower_hir_ty_sub` (member-RESULT instantiation is a formation
    site: `refs().collect()` materialized `Array[&Int64]` out of
    inference — heap refs past the line that never moves; now a clean
    DoesNotConform anchored at the signature span — synthetic spans
    render as NOTHING, builds fail silently).
  - **Operator gap CLOSED** (the 2b carry-over): `a == b` at
    `Optional[&Int64]` is a clean `!: Equal` DoesNotConform. The bypass
    was `nominal_satisfies` trusting INDIRECT conformance sources
    unconditionally — blanket extensions on protocols
    (`extend Equatable: Equal[Self]`, the actual `==` route) and
    refinement parents (`Comparable: Equatable`) now both gate on
    `type_satisfies(recv, parent_protocol)` (depth-guarded recursion).
  - **Stdlib surface**: `RefSliceIterator[T]` / `MutRefSliceIterator[T]`
    (pointer.ks; ptr+remaining, Pointer-bridge next() bodies, Static —
    no relaxation needed) + `Array.refs()` / `Array.mutableRefs()`
    (mutableRefs runs `ensureUnique()` first — COW-safe, pinned).
    `for x in arr.mutableRefs() { x += 1 }` mutates in place.
  - **Residual gaps (recorded)**: G3 — generic `-> I.Item` bodies are
    not escape-re-checked post-mono (accepted narrowing, G2 precedent;
    both shipped iterators yield heap PointerDerived refs; follow-up
    sketch: abstract-aware taint joins at mir-lower body/mod.rs:2496 /
    control.rs:143 / pattern.rs:277 + mono-side carrier re-run, blocked
    on a mono user-diagnostic channel). G4 — the Static formation
    wellformedness covers MEMBER dispatch; free-function generic calls
    still bypass (instantiated-signature wf residual, same family as
    the Copyable mono-substitution gap). Peel-and-forward witnesses
    (`&U: Protocol` via pointee forwarding — would make `==` on
    `Optional[&T]`, contains(), sorts WORK instead of clean-reject) are
    the ruled follow-up — ✅ SHIPPED 2026-06-12, see the next bullet.
    Ref-Item closure combinators (map/filter) are
    out of scope pending 2c. Tail-position decay of ref-returning calls
    (`func f() -> Int64 { b.fetch() }`) is a pre-existing stage-1 gap,
    unrelated to witnesses (bind via `let` first).
- **Ref conformances (`extend &T: P`) — ✅ IMPLEMENTED 2026-06-12**
  (commits 08707392, 6f1a19fc, 8ad4bc0b, 4b273479 + stdlib; every
  commit full-suite green, final 3366+). The peel-and-forward follow-up,
  RATIFIED 2026-06-11 as a language construct over compiler-synthesized
  witnesses ("no special cases"): ref types are extension targets and
  the stdlib AUTHORS the forwarding conformances in Kestrel. (The
  third option, auto-deref, was assessed and rejected: refs already
  auto-peel at every expression position — the gap is TYPE-ARGUMENT
  conformance, which only a conformance mechanism reaches.)
  - **Mechanism**: generic synthetic entities `lang.&`/`lang.&mutating`
    (seed_ptr pattern, pointee param `T`; extension LHS args bind BY
    NAME to the target's declared params, so ref extensions spell the
    pointee `T`). The entities exist only at decl/name-res; every type
    chokepoint maps them back to Ref (try_lang_primitive,
    build_self_type→lower_named_type, create_extension_self_type —
    Self inside a ref extension IS `TyKind::Ref{Param}`, which keeps
    the transparent-place peel dispatching bodies on the POINTEE; a
    leaked entity type would have made `self.isEqual(to:)` recurse).
  - **Acceptance**: the TypeArg ref gate became declared+satisfies —
    conforms_to gained a TyKind::Ref arm (ConformingProtocols of the
    lang entity, declares-only); type_satisfies' Ref arm routes to
    nominal_satisfies with args=[pointee], so the extension's
    `where T: P` evaluates at the REAL pointee through the existing
    extension_bounds_hold (zero new bound-eval code). Static/Copyable/
    Cloneable builtin arms untouched (copy-fold kernel owns them). NO
    `&mutating` ← `&` subsumption — each mutability conforms via its
    own extension (match_pattern matches mutability exactly).
  - **Dispatch**: mono match_pattern Ref arm (structural recursion,
    exact mutability). Witness lowering needed ZERO new code — lang.&
    is a module struct, so lower_witnesses already iterates it and the
    C2 mapping makes implementing_type = `Ref{TypeParam}`.
  - **Bodies/ABI**: ref-typed SIGNATURE params (`other: Self`) keep
    their ref type in the body's value table (resolve_local_type peels
    — the expr seam — and mismatched the ABI) and peel ONCE at entry
    into the let-ref VIEW representation via a fused begin_borrow,
    registered as named ref bindings (all existing use paths apply).
    Codegen (both backends) learned the two ref-arg representations:
    fused begin_borrow peel (ref operand, pointee result → load the
    stored address; owned ref → pass through) and the call-arg carve
    refined by `arg_is_ref_typed` (a ref-typed @guaranteed arg IS the
    address ByRef wants; only pointee-typed views spill). A param
    spelled literally `Self` may carry the target's top-level ref;
    written `&T` params stay E480.
  - **Stdlib surface (v1)**: `core/ref.ks` — `extend &T/&mutating T:
    Equatable/Comparable where T: …`; bodies are one-liners through
    the transparent place. Consumers shipped: `Optional[&Int64]`
    ==/!=/<, `Result[&T,E] ==`, `xs.refs().contains()`/`.min()`,
    user-defined `extend &T: P`. Hashable still rejects (method-level
    `[H]` type param — ref-extension requirements with method generics
    are a follow-up); rejection wording now names the `extend &T:`
    rule.
  - **Known carve-outs**: ref-extension INHERENT members are
    dot-unreachable (the eager receiver peel resolves members on the
    pointee) — conformance dispatch is the supported surface. Struct
    ANNOTATION formation still only checks Static (custom bounds check
    at generic-call instantiation — G4 family). Extensions must spell
    the pointee `T` (the entity's declared param name).

1. **Type-aware fallback-tier resolution** (recorded option 1 in
   stage1.5/compiler-arch.md) + Array adoption — `arr(i)` as a place.
   No stage-2 content; un-dodges the `at:` decision.
2. **Dict split storage** (parallel meta/keys/values buffers) — gives V
   an address; `dict(at:)`/`dict(unwrap:)` become real places;
   `modify` becomes truly in-place.
3. **Cross-block ref flow — ✅ IMPLEMENTED 2026-06-11** ("stage 1.75").
   Named ref bindings now thread through ALL control flow (if/match
   arms+merges, loop headers/back-edges, break/continue) as @guaranteed
   block args — they joined the existing LiveTracker threading
   (`all_live_tracked`/`rebind_scope_values`), with the param stamped
   from the forwarded value (borrow_source remapped through the same
   rebind, provenance ROOT preserved → `return r` after a merge is
   still E494). Verify gained one narrow rule: a forwarding-consume is
   exempt from borrow-blocking when the borrow is forwarded by the same
   terminator (var slot + its borrow travel together). Codegen: block
   args destined for @guaranteed params pass the ADDRESS (both
   backends' terminator arg resolution; the param machinery already
   handled Guaranteed). Bindings now end at lexical scope exit, not at
   block boundaries; the binding-E497 survives only as a fallback for
   non-tracker-pattern jumps.

## Ratified cuts (2026-06-10)

- **Non-Static STRUCTS are in scope** — second-class struct values
  (ref fields legal; the struct is then non-Static and second-class).
  Motivation: ref-storing iterators/cursors are a stated goal.
- **Ref-yielding iterators are in scope** (pulls old stage 3's witness
  ref-return work into 2d; stage 3 dissolves to the closure-return
  root-rule carve).
- **Function-typed parameters default non-Static** (the Swift
  non-escaping lesson): HOFs accept ref-capturing closures with zero
  annotation — the callee body simply can't store the param; a HOF that
  stores opts in with `F: Static`. Generic (non-function-typed) params
  keep the Static default — a closure passed through generic `T` still
  needs Static (accepted wrinkle).
- **Returning a ref-capturing closure: banned in stage 2.** The root
  rule could permit param-rooted cases later — a clean, purely additive
  relaxation (stage-3-adjacent).
- **Rc-closure migration ordering**: lands with or after 2a, scoped to
  Static closures from day one — otherwise it gets built uniform and
  partially unwound.

## Accepted costs (the tradeoff record)

- **Remaining expressiveness ceiling** (shrunk by the structs/iterators
  ratification): views, cursors, and ref-iterators exist as
  locals/params/transients — but **never in heap storage**
  (`Array[Span]`, ref-bearing struct fields inside Static types,
  Rc-boxed views). Mojo-style stored-view code stays inexpressible;
  that is now the whole ceiling.
- **May-alias temporal window WIDENS — iterator invalidation becomes
  the flagship case**: ref safety here is "no use-after-scope," not
  "no use-after-invalidation" — under decided may-alias,
  `for x in arr.refs() { arr.append(1) }` can realloc under the live
  iterator (§10.4 class). NOT a new hazard — today's value iterators
  already store raw `Pointer`s with a documented "don't retain across
  mutations" contract — but ref iterators institutionalize it across
  every loop body. Mitigable by E504-style lints and (option, undecided)
  debug-build generation stamps on COW storage. No exclusivity backstop
  exists — that was traded away deliberately, and this is where users
  will meet the consequence.
- **API-evolution hazard** (the @escaping class): flipping a parameter
  non-Static → Static (a HOF that starts storing) breaks callers passing
  ref-capturing closures. Library authors must anticipate storage.
- **Default asymmetry**: `T: Static` everywhere except function-typed
  parameter positions — a real teaching cost, and bound-failure
  diagnostics must carry "because it captures `r`, a reference"
  provenance or they'll be miserable deep in generic instantiations.
- **Two closure lowerings forever** (Rc'd Static env vs borrowed stack
  env), with a subtle copy-semantics seam: Rc'd copies share captured
  state; non-Static copies bit-copy pointers (aliasing makes this
  near-equivalent observably, but it's spec surface).
- **The contains-ref predicate is the soundness keystone** with the
  implicit-Copyable blast radius (§9 warning): a false-positive Static
  silently reopens every dangling-storage hole stdlib-wide. Must be
  conservative + per-instantiation-folded from day one (the 3-layer
  Copyable lesson).
- **2a is an infrastructure trough**: multi-week, no user-visible
  feature until 2b/2c. Prerequisites 1–2 ship user value first.

## Open questions (decision stubs for the planning round)

- **`Optional[&T]` decay**: none initially (consume via patterns /
  ref-aware `or`) vs deep-decay to `Optional[T]` on value use.
  Leaning: no-decay — smaller commitment.
- **`Optional[&mutating T]`** in checked subscripts vs read-only
  `checked:` first. Leaning: read-only first.
- **Static relaxation spelling**: "need not be Static" (`?Sized`-shaped
  default-removal) vs reusing `not Static` negative bounds — must not
  conflate with "must NOT be Static". (Now a CORE 2a deliverable —
  combinators need it — but the spelling itself is undecided.)
- ~~Non-Static structs~~ RATIFIED IN 2026-06-10 (see cuts).
- **for-in surface for ref Items**: transparent (`for x in arr.refs()`
  binds x as a ref because Item is a ref) vs an explicit `&x`-pattern
  cue; and whether plain `for x in arr` ever switches to by-ref.
- **Mutating iteration** (`&mutating` Items, Rust `iter_mut`-shaped):
  in 2d or a follow-on.
- **Debug-build invalidation guards** (generation stamps on COW
  storage, C#-style): worth the word-per-collection cost?
- **Transparent-place peel inside generic bodies** at `V = &T`
  (Equatable/Formattable instantiations dispatch on ref receivers) —
  audit item.
- **Copyability of refs as payload values** — tolerable under may-alias
  either way; deserves an explicit ruling, not an accident.

## Superseded framing (for the record)

The original stage 2 — lifetime-carrying types, refs in struct fields,
`MonoTypeKey` lifetime provenance (§8: ~16–26 wk, redesign-scale) — is
rejected permanently, not deferred. Its remaining preconditions list
dissolves: the Rc-closure co-design is resolved by the two-tier model
(2c); the fixpoint-verifier upgrade is not needed while checking stays
per-function single-pass over second-class values; Hylo remote parts
were evaluated and ARE this redefinition.
