# References — Implementation Plans

Per-stage implementation plans for second-class references (`&T` /
`&mutating T`). The research, audits, and decision record live in
[`docs/references-prototype/`](../../references-prototype/references.md) —
read `references.md` (feasibility) and `references-gaps.md` (third audit;
adopted decisions §10; revised staging §11) first. These plans restate only
what implementation needs; the *why* stays in the research docs.

## Status matrix

| | requirements | syntax | errors | semantics | tests | compiler-arch |
|---|---|---|---|---|---|---|
| **stage0.5** — pointer capture + reserved ref syntax | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **stage1** — returnable refs | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| **stage1.5** — ergonomics | ✅ | ✅ COMPLETE | ✅ COMPLETE | ✅ COMPLETE | ✅ COMPLETE | ✅ COMPLETE |
| **stage2** — second-class composition (2a ✅ SHIPPED 2026-06-11) | ✅ | 2a ✅ | 2a ✅ | 2a ✅ | 2a ✅ | 2a ✅ |
| **stage3** — DISSOLVED into stage2 (Static→2a, witness refs→2d; closure-return carve remains) | ✅ scope only | ✅ | ⬜ | ⬜ | ⬜ | ✅ sketch |

✅ defined now · 🚧 partially defined (open sections marked inline) ·
⬜ blank — needs exploration (stub states the blocker)

## Implementation status

- **stage2a — SHIPPED 2026-06-11** (feature/115 branch): the `Static`
  containment bound. `@builtin(.Static)` marker protocol; structural
  staticness kernel (`kestrel-semantics/src/staticness.rs`, single
  source of truth; solver/analyze mirrors via `StaticLayer`); implicit
  `T: Static` on every generic param (relax: `where T: not Static` =
  need-not; `not Static` owners relax wholesale); `Pointer[T]` Static
  regardless of T (gained `T: not Static`); DoesNotConform "because"
  details; E505 statics/globals check; E212 widened to non-Static
  captures. Zero-breakage gate: full suite green with the bound live,
  compile-time flat. Details: `stage2/requirements.md`.
- **stage0.5 — SHIPPED** (79c48ca0): refs parse everywhere, rejected
  everywhere (E480–E489), `Pointer(to:)` pinned.
- **stage1.5 COMPLETE 2026-06-10** (feature/115 branch). Beyond items
  1+5 below, the same day shipped:
  - **Item 2 — named ref bindings** (ratified semantics): `let r = &expr;`
    holds any place across STATEMENTS — multiple reads through one
    borrow, `let s = r` decays to a copy, `let s = &r` re-borrows,
    `r = v` store-through on `&mutating` bindings, var-slot aliasing
    (writes visible — may-alias). ~~BLOCK-LOCAL~~ **bindings cross ALL
    control flow since 2026-06-11** ("1.75": threaded as @guaranteed
    block args through if/match/loops; end at lexical scope exit; root
    preserved so post-merge `return r` stays E494). Decl/use
    rules: E209 (let-only), E210 (`&mutating` needs a mutable place),
    E212 (no closure capture), E499 (no rvalue borrows), E482 kept for
    annotations.
  - **`&` pattern bindings**: `.Occupied(_, &v, _)` projects the enum
    payload IN PLACE (place-mode matches thread the pinned place's raw
    address through the decision tree; intra-block views; NotCopyable
    payloads readable without copies; `&mutating v` writes through).
    Match-arm-only (E211); mutable scrutinee place for `&mutating`
    (E210).
  - **Item 3 — dangle lint**: E504 WARNING on
    `return Pointer(to: <same-fn local>).value` shapes
    (analyze body/dangle_ref.rs; `RetRefPointerDerived` moved
    mir-lower → kestrel-type-infer to share the wrapper recognition).
  - **Item 4 — DISSOLVED, interim shipped**:
    `Dictionary.modify(key) { (mutating v) in … } -> R?` (bucket
    writeback under the hood; upgrades to in-place silently once
    `Optional[&T]` + bindings make `if let r = &dict.find(key)`
    expressible).
- **stage1.5 items 1 + 5 — IMPLEMENTED 2026-06-10** (feature/115 branch):
  - **Place accessors**: `ref { … }` / `mutating ref { … }` clauses on
    subscripts and computed properties. `ref` is CONTEXTUAL (parser state
    threads the source; `get`/`set` stay reserved tokens). Accessors build
    as `NodeKind::RefAccessor` child entities with synthesized `&T` returns;
    `PlaceAccessors` (hir-lower) is the single provider-discovery query.
    Per-operation routing: read → ref else get (read-provider reads type
    `&T`, all stage-1 decay rides free); `x(i) = v` → mutating ref else
    set; RMW + mutating-method receivers → mutating ref else get→op→set
    WRITEBACK (element copied out via the read provider into a temp slot,
    written back through the setter — witness-dispatched for protocol-
    extension members — with watermark-scoped drains at the call emitters
    + a statement-boundary safety net). Decl rules E619–E622.
  - **Resolution decision (supersedes the option-1 leaning)**: the labeled
    place form — Array gained inherent `subscript(at index: Int64)` with
    the ref pair; the unlabeled/`checked:`/… subscripts stay on
    `extend Slice[T]` get/set. Distinct labels route through the existing
    label-based fallback — ZERO resolve.rs changes. `arr(i) += v` works
    via writeback; `arr(at: i) += v` is in-place. `at(index:)`/
    `mutableAt(index:)` removed; tests migrated to `arr(at: i)`.
  - **Arm-value decay (item 5)**: `match c { 1 => b.peek(), _ => 0 }`
    decays to owned (4th value-context set in `bind_call_result`; the
    docs' "reuse Constraint::Decay" wording was stale — stage 1 shipped
    expr-id SETS, not a constraint). Return-position match in a `-> &T`
    fn shifts E497 → E494, pinned. The literal-element follow-up also
    landed: array/tuple/dict elements decay (set renamed
    `always_decay_exprs`), which emptied E492's inference surface
    (validation kept as backstop).
  - **Known carry-over gap**: a `&mutating` return from a direct FIELD
    projection (`mutating func m() -> &mutating T { self.v }`) still
    E494s — stage 1 verifies only param-rooted/Pointer-derived mutable
    roots — so mutating-ref accessor bodies use the Pointer bridge
    (`…ptr().offset(by:).mutatingValue`), like Array's.
- **stage1 — IMPLEMENTED 2026-06-10** (feature/115 branch): `-> &T` /
  `-> &mutating T` returns, root-rule escape checker (E494–E497, user-facing
  MIR verify diagnostics), `ret_borrow` ABI on both backends, transparent
  place + binding/scrutinee decay, `Pointer.value`/`.mutatingValue` bridge,
  `Array.at(index:)` / `mutableAt(index:)`. Deltas discovered while
  implementing:
  - `PointerDerived` originates at the `lang.ptr_ref`/`ptr_mut_ref`
    intrinsics, not at the `Pointer` nominal. It crosses exactly one call
    seam: a thin intrinsic wrapper (every return-position expression is a
    direct intrinsic call — the `RetRefPointerDerived` query) stamps its
    call-site result `PointerDerived`; every other ref-returning call roots
    at its borrow source, which is the verified discipline.
  - ~~Compound assignment through a ref-returning call rejected~~ SHIPPED
    2026-06-10: `arr.mutableAt(index: i) = v` and `+= v` both write through
    any `&mutating T`-returning call/getter (E202/E207/E208 reject the
    non-place and shared-ref forms). Value-subscript writeback
    (`arr(0) += 1`) remains stage 1.5.
  - Dict ref adoption waits for `Optional[&T]` (decision 2026-06-09;
    the address path turned out to already exist — `EnumPayload` on a
    `@guaranteed` operand projects in place, both backends; surface =
    `&` pattern bindings, stage1.5 docs).
  - The two formerly-uncoded guards are now coded diagnostics (2026-06-10),
    and the references suite has zero skips: copy-out of a NotCopyable
    pointee = **E503** (the MIR-lowering backstop of the front-end move
    checker's code), consume-while-borrowed = **E498** (verify `try_consume`,
    coded only when a live ref chains to the consumed value — an
    unattributable conflict stays an ICE).

## What blocks the blanks

1. ~~**Q8 — use semantics**~~ **Decided 2026-06-09: transparent place (a),
   no evaporate detour** — rules in `stage1/semantics.md`, decision record
   `references-gaps.md` §10.5. Stage 1 is now fully defined.
2. ~~**Call-as-place lowering**~~ **SHIPPED 2026-06-10** as the `ref` /
   `mutating ref` accessor kinds + writeback fallback (see implementation
   status above). The subscript-resolution question was settled by
   DECISION, not code: the labeled `at:` place form — no resolution
   change needed or made (the type-aware fallback-tier option stays
   unbuilt). ~~Item 2 blocks on ratification~~ — RATIFIED + SHIPPED
   2026-06-10 (see implementation status). Stage 1.5 has no blanks left.
3. ~~**Stage-2 commitment**: the standing default is *don't build*~~
   **REDEFINED 2026-06-10** (maintainer-ratified): stage 2 is no longer
   §8's storable-refs/lifetime design (that is rejected *permanently*,
   not deferred). New scope = **second-class composition** toward the
   goal API (`arr(i) -> &T`, `arr(checked: i) -> Optional[&T]`, dict
   equivalents, ref-capturing closures, ref-yielding + ref-STORING
   iterators): 2a `Static` bound (moved from stage 3, lands FIRST as
   containment; need-not-be-Static spelling + ConditionalStaticParams
   are core) → 2b refs in enums/tuples/structs (second-class struct
   values ratified IN) → 2c ref-capturing closures (two-tier Rc carve)
   → 2d ref-bearing protocols/witnesses + for-in (old stage 3 pulled
   in). Prerequisites: type-aware fallback-tier resolution, Dict split
   storage, cross-block ref flow ("1.75"). Full decision + tradeoff
   record: `stage2/requirements.md`.

## Gating order

stage0.5 ✅ → stage1 ✅ → stage1.5 ✅ → pre-2 work (resolution
fallback-tier fix, Dict split storage, cross-block refs) → stage2
(2a Static → 2b aggregates → 2c closures → 2d witnesses/iterators).
Stage 3 is dissolved; only the closure-return root-rule carve remains
as a possible follow-on.
