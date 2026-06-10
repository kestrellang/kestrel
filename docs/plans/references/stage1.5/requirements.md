# Stage 1.5 — Requirements

Ergonomics follow-ons, scheduled **on demand** after stage 1 ships. Items
are independently shippable; do not bundle.

1. **Call-as-place**: ~~assignment through a mut-ref accessor~~ **SHIPPED
   EARLY (2026-06-10)**: both `arr.mutableAt(index: i) = v` (PtrTo on the
   @guaranteed ref result + StoreAssign) and
   `arr.mutableAt(index: i) += v` (desugar admits call-shaped LHS;
   assignment analyzer validates — E202 for plain-value calls, E207/E208
   for `&T`) work through any `&mutating T`-returning call or getter.
   REMAINING here: value-subscript writeback (`arr(0) += 1` — read-modify-
   write through the getter/setter pair) and its reconciliation with the
   subscript-setter lowering (`references-gaps.md` §10.4).
2. **Named ref bindings**: `let r = &expr;` with the visible-`&` cue
   (`references-syntax.md` §2 Option C), block-local.
3. **Dangle lint**: same-function `Pointer(to: local)` returned as a ref
   (`references-gaps.md` §10.3).
4. **Shared-read projection sugar** over `Pointer.with` / Design-B closures
   — covers the `Optional[&T]`-shaped lookup APIs stage 1 cannot express
   (`references-gaps.md` §5.3).

Entry: stage 1 shipped + concrete demand. ~4-6 wk total.
