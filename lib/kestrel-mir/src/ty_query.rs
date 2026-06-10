use std::borrow::Cow;

use kestrel_copy_fold::{CopyLayer, CopySem, CopySemantics, instance_semantics};
use kestrel_hecs::Entity;

use crate::item::function::{WhereClause, WhereConstraint};
use crate::ty::{MirTy, TyArena};
use crate::{CopyBehavior, DropBehavior, MirModule, TyId};

/// Projection to the shared tri-state. `Clone` payloads are heterogeneous
/// (type entity from lowering, Cloneable proto from the clone-shim pass) and
/// must survive `instance_semantics`'s base-passthrough path untouched —
/// which is exactly what `CopySem` guarantees.
impl CopySem for CopyBehavior {
    fn class(&self) -> CopySemantics {
        match self {
            CopyBehavior::Bitwise => CopySemantics::Copyable,
            CopyBehavior::Clone(_) => CopySemantics::Cloneable,
            CopyBehavior::None => CopySemantics::NotCopyable,
        }
    }
}

/// `CopyLayer` over `TyId` — pre-mono MIR's hooks into the shared decision
/// tree (`kestrel_copy_fold::instance_semantics`, the single source of truth
/// for per-instantiation copy semantics across semantics / solver / analyze /
/// MIR). Layer-specific plumbing: precomputed `type_info.copy` /
/// `conditionally_copyable` instead of QueryContext, and the `where_clause`
/// threading for `TypeParam` bounds.
struct MirCopyLayer<'a> {
    arena: &'a TyArena,
    module: &'a MirModule,
    where_clause: Option<&'a WhereClause>,
}

impl CopyLayer for MirCopyLayer<'_> {
    type Ty = TyId;
    type Sem = CopyBehavior;

    fn base_semantics(&self, entity: Entity) -> CopyBehavior {
        self.module
            .structs
            .get(&entity)
            .map(|s| s.type_info.copy.clone())
            .or_else(|| self.module.enums.get(&entity).map(|e| e.type_info.copy.clone()))
            // Unknown entity -> Bitwise (current Named fallback).
            .unwrap_or(CopyBehavior::Bitwise)
    }

    fn gating_positions(&self, entity: Entity) -> Cow<'_, [usize]> {
        Cow::Borrowed(
            self.module
                .structs
                .get(&entity)
                .map(|s| s.conditionally_copyable.as_slice())
                .or_else(|| {
                    self.module
                        .enums
                        .get(&entity)
                        .map(|e| e.conditionally_copyable.as_slice())
                })
                .unwrap_or(&[]),
        )
    }

    fn member_semantics(&self, &ty: &TyId) -> CopyBehavior {
        // Re-enters the public classifier (where_clause threading included).
        copy_behavior(self.arena, self.module, ty, self.where_clause)
    }

    fn sem_from_class(&self, entity: Entity, class: CopySemantics) -> CopyBehavior {
        match class {
            CopySemantics::Copyable => CopyBehavior::Bitwise,
            // Container entity payload — its clone shim recurses into the
            // Cloneable gating field.
            CopySemantics::Cloneable => CopyBehavior::Clone(entity),
            CopySemantics::NotCopyable => CopyBehavior::None,
        }
    }
}

/// Copy behavior of a pre-mono MIR type. Nominal instances go through the
/// shared decision tree (`kestrel_copy_fold::instance_semantics` via
/// `MirCopyLayer`): an unconditional base (`type_info.copy`) wins; a `not
/// Copyable` base with `conditionally_copyable` gating positions folds the
/// gating args — any `None` → `None`, all `Bitwise` → `Bitwise`, else
/// `Clone(entity)` (copyable element-wise via the container's clone shim).
pub fn copy_behavior(
    arena: &TyArena,
    module: &MirModule,
    ty: TyId,
    where_clause: Option<&WhereClause>,
) -> CopyBehavior {
    match arena.get(ty) {
        MirTy::I8
        | MirTy::I16
        | MirTy::I32
        | MirTy::I64
        | MirTy::F16
        | MirTy::F32
        | MirTy::F64
        | MirTy::Bool
        | MirTy::Never
        | MirTy::Str
        | MirTy::Pointer(_)
        | MirTy::FuncThin { .. }
        // INTERIM: closures are treated as POD — a 2-word `{code, env}` value
        // bit-copied like a raw pointer, never owning its captured env (which
        // today holds only Copyable captures). This lets a borrowed `self`
        // hand out copies of a stored closure field (e.g. `SplitWhereView.iter`,
        // `IntersperseIterator` separator). Must stay in lockstep with the
        // `needs_drop` arm below — Bitwise + droppable would double-free the env.
        // NEXT VERSION: closures become Rc-boxed reference types; copy → retain.
        | MirTy::FuncThick { .. }
        // Ref never appears as a value type (signature-only; results register
        // as @guaranteed pointee values), so copy/drop on it is vacuous — a
        // pointer-scalar bit-copy keeps any defensive path harmless.
        | MirTy::Ref { .. }
        | MirTy::Error => CopyBehavior::Bitwise,

        // Canonical fold (copy-drift #3 resolved 2026-06-10): any move-only
        // element makes the tuple move-only regardless of position; else any
        // Clone element makes it Clone; else Bitwise. The Clone payload is the
        // first Cloneable element's (inert — never destructured; clone
        // elaboration recurses tuple elements structurally).
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            let mut first_clone = None;
            for &elem in &elems {
                match copy_behavior(arena, module, elem, where_clause) {
                    CopyBehavior::None => return CopyBehavior::None,
                    b @ CopyBehavior::Clone(_) if first_clone.is_none() => first_clone = Some(b),
                    _ => {},
                }
            }
            first_clone.unwrap_or(CopyBehavior::Bitwise)
        },

        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            instance_semantics(
                &MirCopyLayer {
                    arena,
                    module,
                    where_clause,
                },
                entity,
                &type_args,
            )
        },

        // First matching constraint decides (copy-drift #5 resolved 2026-06-10:
        // behavior kept, order-dependence asserted away). Declaration order
        // could only pick the answer if a positive Copyable/Cloneable bound
        // coexisted with `not Copyable` on the same param — asserted absent
        // below. (This scan is the MIR form of the per-layer Param hook —
        // precomputed where_clause instead of TypeParamCopyRequirement.)
        MirTy::TypeParam(entity) => {
            let entity = *entity;
            if let Some(wc) = where_clause {
                #[cfg(debug_assertions)]
                {
                    let positive = wc.constraints.iter().any(|c| {
                        matches!(c, WhereConstraint::Implements { type_param, protocol, .. }
                            if *type_param == entity
                                && (is_cloneable_protocol(module, *protocol)
                                    || is_copyable_protocol(module, *protocol)))
                    });
                    let negative = wc.constraints.iter().any(|c| {
                        matches!(c, WhereConstraint::NotImplements { type_param, protocol }
                            if *type_param == entity && is_copyable_protocol(module, *protocol))
                    });
                    assert!(
                        !(positive && negative),
                        "type param {entity:?} bounds both Copyable/Cloneable and `not Copyable` — declaration order would decide its copy behavior"
                    );
                }
                for constraint in &wc.constraints {
                    match constraint {
                        WhereConstraint::Implements {
                            type_param,
                            protocol,
                            ..
                        } if *type_param == entity => {
                            if is_cloneable_protocol(module, *protocol) {
                                return CopyBehavior::Clone(*protocol);
                            }
                            if is_copyable_protocol(module, *protocol) {
                                return CopyBehavior::Bitwise;
                            }
                        },
                        WhereConstraint::NotImplements {
                            type_param,
                            protocol,
                        } if *type_param == entity => {
                            if is_copyable_protocol(module, *protocol) {
                                return CopyBehavior::None;
                            }
                        },
                        _ => {},
                    }
                }
            }
            CopyBehavior::Bitwise
        },

        MirTy::AssociatedProjection { .. } => CopyBehavior::Bitwise,
    }
}

/// True when a type's copyability is **not** fully determined in the current
/// (generic) context — it can still resolve to `None` (move-only) at some
/// monomorphization. Such a value must be MOVED, never bitwise-copied, inside a
/// generic body: a `CopyValue` baked in here turns into an illegal alias once
/// the type monomorphizes to a `not Copyable` instantiation (e.g. `Result[T,E]`
/// with a non-Copyable `T`), and the surviving original double-frees on drop.
///
/// Distinct from `copy_behavior == None`: a *conditionally* Copyable container
/// gated on an unconstrained type param reports `Bitwise` here (its gating args
/// default to `Bitwise`), yet is unsound to copy. Moving is always correct — the
/// frontend forbids reusing a value that isn't *guaranteed* Copyable, so a
/// mono-dependent value is necessarily single-use.
///
/// - bare type param / associated projection: mono-dependent unless a
///   `Copyable`/`Cloneable` bound guarantees duplicability;
/// - conditionally Copyable container: mono-dependent iff a gating arg is;
/// - tuple: mono-dependent iff any element is;
/// - everything else (primitives, unconditional types): determined → `false`.
pub fn copy_is_mono_dependent(
    arena: &TyArena,
    module: &MirModule,
    ty: TyId,
    where_clause: Option<&WhereClause>,
) -> bool {
    match arena.get(ty) {
        MirTy::TypeParam(entity) => {
            let entity = *entity;
            if let Some(wc) = where_clause {
                for constraint in &wc.constraints {
                    if let WhereConstraint::Implements {
                        type_param,
                        protocol,
                        ..
                    } = constraint
                        && *type_param == entity
                        && (is_copyable_protocol(module, *protocol)
                            || is_cloneable_protocol(module, *protocol))
                    {
                        return false;
                    }
                }
            }
            true
        },
        MirTy::AssociatedProjection { .. } => true,
        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems
                .iter()
                .any(|&e| copy_is_mono_dependent(arena, module, e, where_clause))
        },
        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            let gating = module
                .structs
                .get(&entity)
                .map(|s| s.conditionally_copyable.clone())
                .or_else(|| {
                    module
                        .enums
                        .get(&entity)
                        .map(|e| e.conditionally_copyable.clone())
                });
            let Some(gating) = gating else {
                return false;
            };
            // Only gating positions affect the container's copyability.
            gating.iter().any(|&pos| {
                type_args
                    .get(pos)
                    .is_some_and(|&arg| copy_is_mono_dependent(arena, module, arg, where_clause))
            })
        },
        _ => false,
    }
}

pub fn needs_drop(arena: &TyArena, module: &MirModule, ty: TyId) -> bool {
    match arena.get(ty) {
        MirTy::I8
        | MirTy::I16
        | MirTy::I32
        | MirTy::I64
        | MirTy::F16
        | MirTy::F32
        | MirTy::F64
        | MirTy::Bool
        | MirTy::Never
        | MirTy::Str
        | MirTy::Pointer(_)
        | MirTy::FuncThin { .. }
        // INTERIM: closures are POD — see `copy_behavior`. A FuncThick never
        // owns its captured env (Copyable captures only, today), so it needs no
        // drop. Must match the Bitwise arm in `copy_behavior`. NEXT VERSION:
        // Rc-boxed closures need drop → release.
        | MirTy::FuncThick { .. }
        // Ref: signature-only borrow view — never owns, never drops.
        | MirTy::Ref { .. }
        | MirTy::Error => false,

        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            elems.iter().any(|&e| needs_drop(arena, module, e))
        },

        MirTy::Named { entity, .. } => {
            let entity = *entity;
            if let Some(s) = module.structs.get(&entity) {
                return s.type_info.drop != DropBehavior::None;
            }
            if let Some(e) = module.enums.get(&entity) {
                return e.type_info.drop != DropBehavior::None;
            }
            false
        },

        MirTy::TypeParam(_) => true,
        MirTy::AssociatedProjection { .. } => true,
    }
}

pub fn is_cloneable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .get(&entity)
        .is_some_and(|p| p.name.ends_with("Cloneable"))
}

pub fn is_copyable_protocol(module: &MirModule, entity: Entity) -> bool {
    module
        .protocols
        .get(&entity)
        .is_some_and(|p| p.name.ends_with("Copyable"))
}

pub fn find_cloneable_protocol(module: &MirModule) -> Option<Entity> {
    module
        .protocols
        .values()
        .find(|p| p.name.ends_with("Cloneable"))
        .map(|p| p.entity)
}
