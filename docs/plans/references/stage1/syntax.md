# Stage 1 — Syntax

## Return types

- `func first() -> &Element;` · `func cell(at i: Int) -> &mutating Cell;`
- Provenance is **inferred, single-source**: the unique reference-eligible
  parameter root (overwhelmingly `self`). No annotation syntax.
  - Two eligible sources (`func pick(a: T, b: T) -> &T` — both borrow
    params are eligible roots) → clean rejection. Reserve a `from:`-style
    annotation slot for later; do not build it.
  - A `consuming` receiver cannot return a ref of `self` (error).
  - `-> &mutating T` requires a **mutable root**: a `mutating` receiver, a
    `mutating` param, or `.mutatingValue` (`references-gaps.md` §10.4).
- Receivers: `borrowing` (default) and `mutating` receivers may yield `&T`;
  a `mutating` receiver may yield `&mutating T`.

## Use sites

- A ref-typed expression is a **place** (Q8 decided: transparent place —
  `semantics.md`): it works as the receiver of any member/operator/subscript
  form, and *decays* (copy-out) in value contexts — so
  `let r = ring.peek();` compiles and `r` is an owned copy, **not** a
  borrow. The explicit keep-the-ref binding (`let r = &expr;`) is stage 1.5.
- Still no `&` in expression position; call sites stay convention-blind.

## Pointer bridge accessors

```kestrel
var value: &T                   // borrow of the pointee; no T: Copyable needed
var mutatingValue: &mutating T
```

Computed properties — which makes property getters ref-returning functions.
The same root rules apply to any user-written `var x: &T { ... }`.
