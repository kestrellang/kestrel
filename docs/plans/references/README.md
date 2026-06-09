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
