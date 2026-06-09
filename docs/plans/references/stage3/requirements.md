# Stage 3 — Requirements (scope only; contingent on stage 2)

The `Static` default bound: every type param implicitly `T: Static`
(escapable); `T: not Static` lifts it. Meaningless without stage 2's
lifetime representation — do not schedule independently.

The template is decided: clone the Copyable machinery
(`implicit_conformance: true`, `inject_implicit_*_bounds`,
`WhereConstraint::NegativeBound`) per Swift SE-0427's rule set — default
present, suppression *lifts* (never requires), no inheritance propagation,
restricted conditional form (`references.md` §9;
`references-prior-art.md` §3.2).
