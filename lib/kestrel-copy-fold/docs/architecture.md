# kestrel-copy-fold Architecture

The copy-semantics decision tree: one kernel shared by every layer that
classifies a type as Copyable / Cloneable / NotCopyable.

## Pipeline Position

Source Text → Tokens → CST → AST Build → Name Res → HIR Lower → Type Infer → MIR → Codegen
                                                       ^^^^^^^^^^^^^^^^^^^^^^^^^
                                    consumed by kestrel-semantics, kestrel-type-infer,
                                    kestrel-analyze (via semantics re-exports), and
                                    kestrel-mir (direct) — this crate is a leaf below all of them

## Core Types

| Type | Module | Description |
|------|--------|-------------|
| `CopySemantics` | `lib.rs` | Tri-state classification (Copyable / Cloneable / NotCopyable); re-exported by kestrel-semantics |
| `CopyRequirement` | `lib.rs` | What a type-param bound demands; re-exported by kestrel-semantics; `From<CopyRequirement> for CopySemantics` dedups the 3-arm mapping |
| `CopySem` | `lib.rs` | Layer-native semantics value projecting to the tri-state; lets MIR's payload-bearing `CopyBehavior::Clone(Entity)` survive the base-passthrough path untouched |
| `CopyLayer` | `lib.rs` | One data source (HirTy / TyKind / ResolvedTy / TyId): base semantics, gating positions, member classification, native-value construction |
| `fold_members` | `lib.rs` | Canonical aggregate fold: NotCopyable dominates, else any Cloneable → Cloneable, else Copyable |
| `instance_semantics` | `lib.rs` | THE decision tree for nominal `entity[args]`: unconditional base wins (returned natively); `not Copyable` base + gating positions → fold the gating args; missing arg → NotCopyable |

## Module Map

| File | Responsibility |
|------|---------------|
| `src/lib.rs` | Everything: enums, hook trait, kernel, unit tests |

## Dependencies

| Crate | Usage |
|-------|-------|
| `kestrel-hecs` | `Entity` (nominal type identity in `CopyLayer` hooks) |

## The layers (who implements `CopyLayer`)

| Layer | Crate / site | Ty | Sem |
|-------|--------------|----|----|
| 1 | `kestrel-semantics::HirCopyLayer` | `HirTy` | `CopySemantics` |
| 2 | `kestrel-type-infer::solver::SolverCopyLayer` | `TyVar` | `CopySemantics` |
| 3 | `kestrel-analyze::body::move_tracking::MoveCopyLayer` | `ResolvedTy` | `CopySemantics` |
| 4a | `kestrel-mir::ty_query::MirCopyLayer` | `TyId` | `CopyBehavior` |
| 4b | `kestrel-mir::mono::MonoCopyLayer` | `TyId` | `CopyBehavior` |

Deliberate non-member: `TypeResolver::copy_semantics_of`
(kestrel-type-infer/src/resolve.rs) — intentionally permissive and
arg-independent ("never block the resolver"); unifying it would change
resolver behavior.

## Rules

- The member fold and the nominal-instance rule live here and ONLY here.
  Never re-implement the gating fold in a layer.
- Intentional layer divergences (solver tuple-Cloneable, MIR tuple rules,
  MIR TypeParam constraint-order precedence, …) stay in that layer's
  classifier arm with a `TODO(copy-drift #n)` comment.
- Layer `member_semantics` matches must stay exhaustive — no `_ =>`
  catch-alls — so a new type variant forces a per-layer decision.
