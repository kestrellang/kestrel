# Stage 1.5 — Compiler architecture

**Status: BLANK — needs exploration.**

Known unknowns: `add_guaranteed_block_param` is still only a panic-string
aspiration (`builder.rs:95,125`) — needed if bindings ever cross blocks;
the setter-lowering reconciliation site for call-as-place; where the dangle
lint runs (body analyzer vs. verify pass).
