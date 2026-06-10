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
| **stage1.5** — ergonomics | ✅ | 🚧 | ⬜ | ⬜ | ⬜ | ⬜ |
| **stage2** — storable refs | ✅ scope only | ⬜ | ⬜ | ⬜ | ⬜ | ⬜ |
| **stage3** — Static bound | ✅ scope only | ✅ | ⬜ | ⬜ | ⬜ | ✅ sketch |

✅ defined now · 🚧 partially defined (open sections marked inline) ·
⬜ blank — needs exploration (stub states the blocker)

## Implementation status

- **stage0.5 — SHIPPED** (79c48ca0): refs parse everywhere, rejected
  everywhere (E480–E489), `Pointer(to:)` pinned.
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
  - Dict ref accessors deferred to 1.5+ (maintainer decision; Bucket
    enum-payload layout has no stable-address path).
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
2. **Call-as-place lowering**: `arr.at(i) = v` through a mut-ref accessor
   vs. the existing subscript-setter path. Blocks most of stage1.5.
3. **Stage-2 commitment**: the standing default is *don't build*
   (`references-gaps.md` §11); its files stay blank unless users hit the
   stage-1.5 ceiling.

## Gating order

stage0.5 → stage1 → stage1.5 (on demand). stage2 only if re-litigated;
stage3 only after stage2.
