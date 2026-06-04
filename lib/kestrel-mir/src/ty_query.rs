use kestrel_hecs::Entity;

use crate::item::function::{WhereClause, WhereConstraint};
use crate::ty::{MirTy, TyArena};
use crate::{CopyBehavior, DropBehavior, MirModule, TyId};

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
        | MirTy::Error => CopyBehavior::Bitwise,

        MirTy::Tuple(elems) => {
            let elems = elems.clone();
            for &elem in &elems {
                match copy_behavior(arena, module, elem, where_clause) {
                    CopyBehavior::Bitwise => {},
                    other => return other,
                }
            }
            CopyBehavior::Bitwise
        },

        MirTy::Named { entity, type_args } => {
            let entity = *entity;
            let type_args = type_args.clone();
            if let Some(s) = module.structs.get(&entity) {
                return instantiated_copy_behavior(
                    arena,
                    module,
                    entity,
                    &s.type_info.copy,
                    &s.conditionally_copyable,
                    &type_args,
                    where_clause,
                );
            }
            if let Some(e) = module.enums.get(&entity) {
                return instantiated_copy_behavior(
                    arena,
                    module,
                    entity,
                    &e.type_info.copy,
                    &e.conditionally_copyable,
                    &type_args,
                    where_clause,
                );
            }
            CopyBehavior::Bitwise
        },

        MirTy::TypeParam(entity) => {
            let entity = *entity;
            if let Some(wc) = where_clause {
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

/// Refine a type's `copy` behavior for a concrete instantiation. A type whose
/// base is `not Copyable` (`None`) but which is *conditionally* Copyable
/// (`struct X: not Copyable` + `extend X: Copyable where T: Copyable`, captured
/// as `conditionally_copyable` gating positions) gets its behavior from the
/// gating args, matching the inference solver's per-instantiation conformance:
/// - any gating arg `None` (move-only) → the container is `None`;
/// - all gating args `Bitwise` → the container is `Bitwise` (bit-copyable);
/// - all gating args Copyable but at least one `Clone` (Cloneable) → the
///   container is `Clone(entity)` (copyable, but element-wise via clone — its
///   clone shim recurses into the Cloneable field).
///
/// For unconditional types (empty gating list) the base behavior is returned
/// unchanged, so this is inert until a type adopts the conditional pattern.
fn instantiated_copy_behavior(
    arena: &TyArena,
    module: &MirModule,
    entity: Entity,
    base: &CopyBehavior,
    conditionally_copyable: &[usize],
    type_args: &[TyId],
    where_clause: Option<&WhereClause>,
) -> CopyBehavior {
    if !matches!(base, CopyBehavior::None) || conditionally_copyable.is_empty() {
        return base.clone();
    }
    let mut saw_clone = false;
    for &pos in conditionally_copyable {
        match type_args
            .get(pos)
            .map(|&arg| copy_behavior(arena, module, arg, where_clause))
        {
            Some(CopyBehavior::Bitwise) => {},
            Some(CopyBehavior::Clone(_)) => saw_clone = true,
            // Move-only arg, or a missing/out-of-range position: not copyable.
            _ => return CopyBehavior::None,
        }
    }
    if saw_clone {
        CopyBehavior::Clone(entity)
    } else {
        CopyBehavior::Bitwise
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
                    {
                        if *type_param == entity
                            && (is_copyable_protocol(module, *protocol)
                                || is_cloneable_protocol(module, *protocol))
                        {
                            return false;
                        }
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
