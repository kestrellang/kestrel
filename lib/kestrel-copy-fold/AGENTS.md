# kestrel-copy-fold

The copy-semantics decision tree (Copyable / Cloneable / NotCopyable) shared
by every layer. See `docs/architecture.md`.

- **Never decide copyability yourself.** `fold_members` and
  `instance_semantics` live here and only here. A layer implements `CopyLayer`
  and calls them.
- **A deliberate layer divergence** (solver tuples, MIR tuple rules, MIR
  type-param precedence) stays in that layer's classifier arm with a
  `TODO(copy-drift #n)` comment.
- **Layer `member_semantics` matches stay exhaustive.** No `_ =>` arm, so a new
  type variant forces a decision in every layer.
- **`CopySem` keeps a layer's own value.** MIR's `CopyBehavior::Clone(Entity)`
  must come back out unchanged on the base-passthrough path.
- `TypeResolver::copy_semantics_of` in kestrel-type-infer is intentionally not
  a layer (permissive, arg-independent). Don't fold it in.
- Depends only on `kestrel-hecs`, so MIR can use it. Keep it a leaf.
