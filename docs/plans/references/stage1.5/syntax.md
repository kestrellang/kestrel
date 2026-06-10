# Stage 1.5 — Syntax

## Named ref bindings (decided direction)

`let r = &ring.peek();` — the visible `&` lands exactly on the construct
that can outlive its referent (`references-syntax.md` §2 Option C).
Prefix-`&` exists **only** in `let`/`var`-initializer position, so the
`a & b` bitwise-AND ambiguity is confined to that one restricted spot.
Bindings are block-local (no cross-merge — E-REF-15 still applies).

## Call-as-place — NEEDS EXPLORATION

`arr.at(i) = v` as an assignment target requires call-expression-as-place
grammar and a reconciliation with the existing subscript-setter lowering
(`try_lower_setter_assign` / `field_subscript_set`): one surface, one
lowering, selected by signature (`-> &mutating T` accessor vs. a `set`
block). Precedence and interaction rules are unexplored — blank until
explored.
