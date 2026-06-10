# Stage 1.5 — Errors

**Status: BLANK — needs exploration.**

Depends on the call-as-place design (`syntax.md`) and the binding-scope
rules (`semantics.md`). Known candidates: binding a ref past its referent's
scope; `&` of a temporary in a let-initializer; write-through-binding
mutability.
