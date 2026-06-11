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
- **2b. Refs in enums/tuples/STRUCTS** (structs ratified IN 2026-06-10
  — ref-storing iterators/cursors are wanted; this is full Hylo remote
  parts): `&T`/`&mutating T` as type arguments and field types
  (instantiation, substitution, conformance); layout — a ref
  payload/field is a pointer scalar, `Optional[&T]` can take the
  nullable-pointer niche; drop shim no-ops / clone is pointer-copy on
  ref components (the `MirTy::Ref` marker exists for the explicit
  skip); escape provenance through wrap (construction taints the
  aggregate with the ref's root) and unwrap/projection (extraction
  roots at the aggregate's root); tag+pointer return ABI as a strict
  extension of `ret_borrow`. `Optional` is the flagship; user
  enums/structs fall out of the same machinery. NOTE the verify-level
  consequence: a ref stored into an aggregate in a memory slot leaves
  SSA borrow tracking — soundness for those refs rests on type-level
  containment + scope rules, not per-value Check-4 tracking (E498-class
  blind spots grow; accepted).
- **2c. Ref-capturing closures** (maintainer requirement 2026-06-10):
  capture classification = the contains-ref predicate applied to the
  capture list; E212 relaxes from a ban to "this capture makes the
  closure non-Static"; the by-ref capture lowering is mechanically ready
  (the `is_protocol_self` path in closure.rs — `BeginBorrowAddr` on an
  env field, no load/own/drop). **Two-tier model**: Static closures get
  the planned Rc upgrade (storable, copy = retain); ref-capturing
  closures stay stack-env, called-then-dropped, never Rc'd or stored.
- **2d. Ref-bearing protocols + for-in** (old stage 3 PULLED IN
  2026-06-10 — ref-yielding iterators are a stated goal; stage 3
  dissolves into this): `type Item = &T` associated-type instantiation
  and `next() -> Optional[&T]` witness requirements. With 2b making
  `&T` an honest type, the ref rides IN the type — §9's side-channel
  borrow-annotation threading (`-> &Self.Item`) may largely dissolve
  into ordinary assoc-type machinery + E48x carve-outs; the
  witness-erasure hazard (shape mismatch between a ref-Item witness and
  an owned-Item witness — `witness_instantiation_collapse` class) is
  the standing watch-item. for-in desugar over ref Items; `refs()` /
  mutating-iteration surface TBD. Iterator COMBINATORS make the Static
  relaxation spelling load-bearing: `MapIterator[I, F]` is
  conditionally Static (the `ConditionalStaticParams` analog), and
  generic algorithms need "I need not be Static" bounds — this moves
  from open question to core 2a deliverable.

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
