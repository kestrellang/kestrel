# kestrel-structural Architecture

The rules for the structural builtin protocols `Static` and `Sized`: one
shape enum and one rule function each, shared by every layer that judges
them.

## Pipeline Position

Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Codegen
                                                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
                          consumed by kestrel-semantics (HIR layer, re-exported to the
                          solver and analyzer) and kestrel-mir (mono) — a leaf below all

## Why a leaf crate

`Static` and `Sized` are computed from a type's structure; no declaration or
witness supplies them. The front end judges them over `HirTy`, `TyVar` and
`ResolvedTy`; mono judges them over concrete `TyId`s when it decides a
witness's `where T: Static` / `where T: Sized` clause. `kestrel-mir` cannot
depend on the front end, so the rules live here (the `kestrel-copy-fold`
precedent for copy semantics).

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `Staticness` | `staticness.rs` | Per-nominal classification: Static, NotStatic, or ConditionalOn(positions) |
| `StaticShape` | `staticness.rs` | A layer's type as staticness sees it |
| `StaticLayer` | `staticness.rs` | Layer hooks: nominal classification, shape, param bound, raw-existential refinement |
| `ty_is_static` / `instance_is_static` | `staticness.rs` | THE rule |
| `Sizedness` | `sizedness.rs` | Per-nominal classification: Sized, or ConditionalOn(tail positions) |
| `SizedShape` | `sizedness.rs` | A layer's type as sizedness sees it |
| `SizedLayer` | `sizedness.rs` | Layer hooks: nominal classification, shape, param bound, associated-type bound |
| `ty_is_sized` / `instance_is_sized` | `sizedness.rs` | THE rule |

## Implementors

| Layer | Crate | Type |
|-------|-------|------|
| `HirStaticWalk` / `HirSizedWalk` | kestrel-semantics | `HirTy` |
| `SolverStaticLayer` / `SolverSizedLayer` | kestrel-type-infer | `TyVar` |
| `ResolvedStaticLayer` | kestrel-analyze | `ResolvedTy` |
| `MirStructuralLayer` | kestrel-mir (`ty_query`) | `TyId` |

The per-nominal classifications are computed once by the front end
(`NominalStaticness`, `NominalSizedness`) and carried into MIR on
`StructDef` / `EnumDef`.
