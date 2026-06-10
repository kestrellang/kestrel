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
| **stage1.5** — ergonomics | ✅ | ✅ items 1+5 | ✅ items 1+5 | ✅ items 1+5 | ✅ items 1+5 | ✅ items 1+5 |
| **stage2** — storable refs | ✅ scope only | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| **stage3** — Static bound | ✅ scope only | ✅ | ⬜ | ⬜ | ⬜ | ✅ sketch |

✅ defined now · 🚧 partially defined (open sections marked inline) ·
⬜ blank — needs exploration (stub states the blocker)

## Implementation status

- **stage0.5 — SHIPPED** (79c48ca0): refs parse everywhere, rejected
  everywhere (E480–E489), `Pointer(to:)` pinned.
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
    fn shifts E497 → E494, pinned.
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
   unbuilt). Item 2 still blocks on ratifying the named-binding fine
   semantics (store-through, no rebind — proposed in stage1.5/syntax.md).
3. **Stage-2 commitment**: the standing default is *don't build*
   (`references-gaps.md` §11); its files stay blank unless users hit the
   stage-1.5 ceiling.

## Gating order

stage0.5 → stage1 → stage1.5 (on demand). stage2 only if re-litigated;
stage3 only after stage2.
