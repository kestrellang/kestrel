# Stage 3 — Compiler architecture (sketch)

Near-1:1 clone of working machinery (`references.md` §9):

- `Builtin::Static` with `implicit_conformance: true` (the `builtin.rs:647`
  precedent) → the negative-conformance path turns on for free.
- `inject_implicit_static_bounds` / `TypeParamStaticRequirement` — direct
  clones of the Copyable templates in `where_clauses.rs`.
- `WorldResolver.conforms_to` gets a `Static` special-case: **structural**
  (contains-a-reference), not declared-conformance lookup.

Genuinely new (the ~3-4 wk core): ref-returning protocol methods through
witness lowering — lifetime erasure at mono is the easy direction, but a
shape mismatch between a protocol method returning `&Self.Item` and a
concrete witness returning owned `T` surfaces as codegen corruption, not a
clean error (the `witness_instantiation_collapse` class). Per-instantiation
`(Entity, Vec<TyId>)` keying in `expand.rs` is the right shape for
per-instantiation Static reasoning.
