//! Which arguments a call's ref-carrying result can borrow from.
//!
//! A callee can only hand back a borrow of something its arguments hold, so
//! an argument whose type cannot contain any type the result points at is
//! never a source: in `xs.slice(from: 0, count: 2)`, an `Int64` can't hold
//! an `Array`, so the result borrows `xs` alone. Decided once per callee
//! SIGNATURE, so the analyzers (E509) and MIR lowering (E494 roots) agree.

use std::collections::HashSet;

use kestrel_ast_builder::TypeParams;
use kestrel_hecs::{Entity, QueryContext, QueryFn};
use kestrel_hir::HirTy;
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes};

use crate::collect_child_types;

/// Per declared parameter (receiver excluded — it is always a possible
/// source): can the callee's result carry a reference into this argument?
/// `None` when the signature can't be read; callers then treat every
/// argument as a source.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct RefSourceParams {
    pub callee: Entity,
    pub root: Entity,
}

impl QueryFn for RefSourceParams {
    type Output = Option<Vec<bool>>;

    fn execute(&self, ctx: &QueryContext<'_>) -> Option<Vec<bool>> {
        let params = ctx.query(LowerCallableTypes {
            entity: self.callee,
            root: self.root,
        })?;
        let ret = ctx.query(LowerCallableReturnType {
            entity: self.callee,
            root: self.root,
        });
        let mut pointees = Vec::new();
        let known = ref_pointees(ctx, self.root, &ret, &mut pointees, &mut HashSet::new());
        Some(
            params
                .iter()
                .map(|p| {
                    let Some(p) = p else { return true };
                    !known
                        || pointees
                            .iter()
                            .any(|t| may_hold(ctx, self.root, p, t, &mut HashSet::new()))
                })
                .collect(),
        )
    }

    fn describe(&self) -> String {
        format!("RefSourceParams({:?})", self.callee)
    }
}

/// Collects the pointee of every reference a `ty` value carries, fields and
/// payloads included. `false` = unknowable (a closure's captures, an opaque
/// or generic type that may itself carry refs).
fn ref_pointees(
    ctx: &QueryContext<'_>,
    root: Entity,
    ty: &HirTy,
    out: &mut Vec<HirTy>,
    expanding: &mut HashSet<Entity>,
) -> bool {
    match ty {
        HirTy::Ref { inner, .. } => {
            out.push((**inner).clone());
            // `&View` also reaches whatever `View` borrows.
            ref_pointees(ctx, root, inner, out, expanding)
        },
        HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
            if !expanding.insert(*entity) {
                return true; // a cycle adds no new reference on its own
            }
            // Args matter only through the fields they're substituted into;
            // a nominal with no visible storage (a builtin) may carry its args.
            let children = instance_children(ctx, root, *entity, args);
            let members = if children.is_empty() {
                args.as_slice()
            } else {
                &children
            };
            let ok = members
                .iter()
                .all(|c| ref_pointees(ctx, root, c, out, expanding));
            expanding.remove(entity);
            ok
        },
        HirTy::Tuple(elems, _) => elems
            .iter()
            .all(|e| ref_pointees(ctx, root, e, out, expanding)),
        HirTy::Alias { expanded, .. } => ref_pointees(ctx, root, expanded, out, expanding),
        HirTy::Function { .. }
        | HirTy::Param(..)
        | HirTy::SelfType(..)
        | HirTy::AssocProjection { .. }
        | HirTy::AbstractAssoc { .. }
        | HirTy::Opaque { .. }
        | HirTy::RawExistential { .. }
        | HirTy::Protocol { .. }
        | HirTy::Infer(_)
        | HirTy::Error(_) => false,
        HirTy::Never(_) => true,
    }
}

/// Whether a value of `ty` may hold (inline, behind a pointer, or through a
/// stored reference) a value of type `target`. Nominals match by head
/// (`Array[Int64]` may be `Array[T]`); generic and opaque types may hold
/// anything.
fn may_hold(
    ctx: &QueryContext<'_>,
    root: Entity,
    ty: &HirTy,
    target: &HirTy,
    expanding: &mut HashSet<Entity>,
) -> bool {
    if let HirTy::Alias { expanded, .. } = target {
        return may_hold(ctx, root, ty, expanded, expanding);
    }
    if !is_concrete_head(target) {
        return true;
    }
    match ty {
        HirTy::Struct { entity, args, .. } | HirTy::Enum { entity, args, .. } => {
            if head_entity(target) == Some(*entity) {
                return true;
            }
            if !expanding.insert(*entity) {
                return false;
            }
            let found = args
                .iter()
                .any(|a| may_hold(ctx, root, a, target, expanding))
                || instance_children(ctx, root, *entity, args)
                    .iter()
                    .any(|c| may_hold(ctx, root, c, target, expanding));
            expanding.remove(entity);
            found
        },
        HirTy::Ref { inner, .. } => may_hold(ctx, root, inner, target, expanding),
        HirTy::Tuple(elems, _) => {
            matches!(target, HirTy::Tuple(..))
                || elems
                    .iter()
                    .any(|e| may_hold(ctx, root, e, target, expanding))
        },
        HirTy::Alias { expanded, .. } => may_hold(ctx, root, expanded, target, expanding),
        HirTy::Never(_) => false,
        // Generic, opaque, existential, closure: may hold anything.
        _ => true,
    }
}

/// A target we can rule out by structure: a nominal or tuple. Anything
/// else (a type param, `Self`, a projection) could be any type.
fn is_concrete_head(ty: &HirTy) -> bool {
    matches!(
        ty,
        HirTy::Struct { .. } | HirTy::Enum { .. } | HirTy::Tuple(..)
    )
}

fn head_entity(ty: &HirTy) -> Option<Entity> {
    match ty {
        HirTy::Struct { entity, .. } | HirTy::Enum { entity, .. } => Some(*entity),
        _ => None,
    }
}

/// Stored field / payload types of a nominal instance, its args substituted.
fn instance_children(
    ctx: &QueryContext<'_>,
    root: Entity,
    entity: Entity,
    args: &[HirTy],
) -> Vec<HirTy> {
    let params = ctx
        .get::<TypeParams>(entity)
        .map(|tp| tp.0.clone())
        .unwrap_or_default();
    let subs: Vec<(Entity, HirTy)> = params.into_iter().zip(args.iter().cloned()).collect();
    collect_child_types(ctx, entity, root)
        .into_iter()
        .map(|(_, t)| t.substitute_params(&subs))
        .collect()
}
