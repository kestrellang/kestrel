# kestrel-structural

The rules for the structural protocols `Static` and `Sized`. See `docs/architecture.md`.

- **`instance_is_static` and `instance_is_sized` are THE rules.** A layer
  (`HirTy`, `TyVar`, `ResolvedTy`, `TyId`) implements `StaticLayer` /
  `SizedLayer` and calls them. Never re-implement the fold in a layer.
- **Don't merge staticness into the copy kernel.** It is a two-state AND whose
  gating positions are structural and legal on any base; copy's are declared
  and only exist on a `not Copyable` base (see the header of `staticness.rs`).
- **Recurse only into gating args.** `Pointer[T]` and `lang.ptr[T]` store no
  `T`, so they are Static for any `T`. That is the unsafe escape hatch.
- **A missing arg at a gating position is false**, not a guess.
- The per-nominal classifications (`NominalStaticness`, `NominalSizedness`) are
  computed once by `kestrel-semantics` and carried into MIR on `StructDef` /
  `EnumDef`. Mono reads those; it doesn't recompute them.
- This crate depends only on `kestrel-hecs`, so `kestrel-mir` can use it.
  Keep it that way.
