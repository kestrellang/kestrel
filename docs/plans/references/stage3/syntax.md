# Stage 3 — Syntax

`T: not Static` — zero lexer/parser work: reuses
`WhereConstraint::NegativeBound` (the `not Copyable` path); `Static` is a
plain ident protocol. Protocols re-state `not Static` on inheritance (no
propagation). Conditional form:
`struct Box[T]: not Static where T: not Static` via a
`ConditionalStaticParams` analog (`references.md` §9).
