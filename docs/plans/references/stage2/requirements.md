# Stage 2 — Requirements (scope only; default: DON'T BUILD)

Storable references: lifetime-carrying types, refs in
structs/enums/tuples/closures. This is the cost center (`references.md` §8:
`MonoTypeKey` lifetime-provenance collapse, the Rc-closure collision, RcBox
soundness with no exclusivity backstop).

**Standing position** (`references-gaps.md` §11): a Hylo-shaped stop at
stage 1.5 is a legitimate end state. The §10 decisions removed collection
accessors — the biggest original pull — from Stage 2's motivation list.

**Re-litigate only when** users concretely hit: `Optional[&T]` lookup
returns, ref-bearing fields (Span/cursor types), or bindings crossing
scopes — *and* the projection sugar (stage 1.5 item 4) has been tried
first.

**Preconditions if ever started:** the Rc-closure migration settled and
co-designed with ref capture; the verifier upgraded toward fixpoint
dataflow; Hylo "remote parts" evaluated as the cheaper alternative to
lifetime-on-every-type (`references-prior-art.md` §6).
