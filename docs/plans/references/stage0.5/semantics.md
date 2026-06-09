# Stage 0.5 — Semantics

## No new semantics (the load-bearing rule)

Every ref-type occurrence is rejected at HIR lowering: `TyKind::Ref` is
never constructed and no `HirTy::Ref` survives into signatures or bodies
(assert it). `ParamConvention` stays the **single source of truth** for
parameter passing exactly as today — with no type-side spelling there is no
normalization step and no two-truths risk (`references-gaps.md` §5.3 is
satisfied vacuously; §10.6 is the decision).

## Behavior

Existing convention behavior is untouched:

- Borrow params are call-scoped `BeginBorrow` / `PassMode::ByRef`.
- `mutating` params require a mutable place (existing E200-class check) and
  may alias — passing the same `var` to two `mutating` params stays legal.

## Pointer inits

- `Pointer(to: x)` captures the address of the borrowed place. The pointer
  is a plain Copyable value and does **not** keep `x` alive (existing
  Pointer contract; `# Safety` documented on both inits).
- `Pointer(mutating: x)` is identical at machine level; it exists so write
  intent requires a mutable place — its argument goes through the same
  E200-class place-mutability check as any `mutating` param. Writing
  through a `to:`-derived pointer is the documented const-cast footgun.
