# Stage 1.5 — Requirements

Ergonomics follow-ons, scheduled **on demand** after stage 1 ships. Items
are independently shippable; do not bundle.

1. **Call-as-place**: `arr.at(i) = v` writing through a mut-ref accessor,
   reconciled with the existing subscript-setter lowering into one path
   (`references-gaps.md` §10.4). Includes compound assignment through a
   ref-returning call (`arr.mutableAt(index: i) += v`) — stage 1's
   syntactic place check rejects call-shaped LHS before types exist, so
   both need the same typed place carve-out.
2. **Named ref bindings**: `let r = &expr;` with the visible-`&` cue
   (`references-syntax.md` §2 Option C), block-local.
3. **Dangle lint**: same-function `Pointer(to: local)` returned as a ref
   (`references-gaps.md` §10.3).
4. **Shared-read projection sugar** over `Pointer.with` / Design-B closures
   — covers the `Optional[&T]`-shaped lookup APIs stage 1 cannot express
   (`references-gaps.md` §5.3).

Entry: stage 1 shipped + concrete demand. ~4-6 wk total.
