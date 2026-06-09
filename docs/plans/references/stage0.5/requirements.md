# Stage 0.5 — Requirements

**Goal:** ship the pointer-capture half of the Pointer bridge and reserve
the reference *type syntax* with **zero new analysis** — no escape checker,
no verifier changes, no `ret_borrow`, no type-inference changes, and no
change to any existing signature.

## The parameter decision (permanent, not a deferral)

Reference types are **disallowed in parameter position by design**.
Parameter passing has exactly one spelling per mode — `x: T` (borrow, the
default), `mutating x: T`, `consuming x: T` — and `x: &T` /
`x: &mutating T` are rejected in this stage and every later one. `x: &T`
would be semantically identical to `x: T` and `x: &mutating T` a second
spelling of `mutating`; one way to say each thing. `&T` is
*return-position* syntax, opened in stage 1. Decision record:
`references-gaps.md` §10.6.

## Deliverables

1. Front-end plumbing for ref types: `&T` / `&mutating T` parse in type
   position (SyntaxKind/parser/AST/HIR variants) but are accepted in **no**
   position this stage. Parsing-then-rejecting buys real diagnostics with
   LSP-grade recovery now and the landing slot for stage-1 returns.
2. Rejection diagnostics for ref types in **every** type position —
   parameters (permanent), returns, bindings, fields, tuples, generic args,
   function types (`errors.md`). One type-position walk; stage 1 carves
   only the return position out of it.
3. `Pointer(to: x)` / `Pointer(mutating: x)` inits — address capture from a
   borrowed place (`withUnsafePointer` without the closure).

## Non-goals (stage1+)

Reference returns; `Pointer.value`/`.mutatingValue`; named ref bindings;
`&` in expression position; any `TyKind::Ref` reaching type inference.

## Success criteria

- Zero change to existing code or behavior — all `mutating`-param tests and
  signatures untouched.
- New tests in `tests.md` green via `/triage`.
- `TyKind::Ref` is never constructed and no `HirTy::Ref` survives HIR
  lowering — every occurrence is rejected; assert it.

## Dependencies / effort

No prerequisites. ~2-3 weeks (the parameter-syntax/normalization half of
the original ~3-5 wk estimate is deleted). Sources: `references-gaps.md`
§11 (staging), §10.2 (bridge inits), §10.6 (parameter decision);
`references-syntax.md` §1/§4.
