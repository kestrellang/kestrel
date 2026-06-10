# Stage 1.5 — Semantics

**Status: BLANK — needs exploration.**

Open: binding-scope rules vs. deferred-`EndBorrow` drain points; interaction
with `diamond_conditional_move_let_drop_timing` (still-open bug — a
conditionally-consumed `let r = &x` inherits it); call-as-place evaluation
order vs. setter lowering.
