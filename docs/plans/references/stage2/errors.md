# Stage 2 — Errors

> **2a SHIPPED 2026-06-11**:
> - **Static bound failures** ride the existing uncoded
>   `InferError::DoesNotConform` with a Static-specific "because" detail
>   (result.rs `describe_static_failure`): `declared 'not Static'` /
>   `field 'x' is non-Static` / `type argument 'T' is non-Static` /
>   `a reference is never Static` / relaxed-param wording.
> - **E505 `static_requires_static_type`** (decl/static_value_type.rs) —
>   module-level value decls + `static` members must have Static types.
> - **E212 widened** to `non_static_capture` (body/closure.rs) — any
>   non-Static captured root; ref roots keep the original wording.
>   Relaxes in 2c.
> - NOTE the allocation drift found while shipping: E426–E429 are TAKEN
>   (duplicate-signature/case/label) despite gaps in the AGENTS.md table
>   — always re-grep `id: "E…"` before allocating.

**Remaining (2b/2c/2d): needs exploration.**
