//! ResolvedTy → MirTy conversion.
//!
//! ResolvedTy comes from type inference results. It has the same shape as HirTy
//! but without spans and without Infer.

use kestrel_ast_builder::{Name, TypeParams};
use kestrel_mir::MirTy;
use kestrel_type_infer::result::ResolvedTy;
use kestrel_type_infer::InferBody;

use crate::context::LowerCtx;
use crate::ty::lower_named_type_from_entity;

/// Convert a ResolvedTy (from type inference) to a MirTy.
pub fn lower_resolved_ty(ctx: &mut LowerCtx, ty: &ResolvedTy) -> MirTy {
    match ty {
        ResolvedTy::Named { entity, args } => {
            let mir_args: Vec<MirTy> = args.iter().map(|a| lower_resolved_ty(ctx, a)).collect();
            lower_named_type_from_entity(ctx, *entity, &mir_args)
        },
        ResolvedTy::Param { entity } => {
            ctx.register_name(*entity);
            MirTy::TypeParam(*entity)
        },
        ResolvedTy::SelfType { .. } => {
            // Abstract `Self` of a protocol. Monomorphization substitutes via
            // `substitute_type_with_self` against the enclosing function's
            // concrete self_type.
            MirTy::SelfType
        },
        ResolvedTy::AssocProjection { base, assoc } => {
            let base_ty = lower_resolved_ty(ctx, base);
            let (protocol, name) = match (
                ctx.world.parent_of(*assoc),
                ctx.world.get::<Name>(*assoc).map(|n| n.0.clone()),
            ) {
                (Some(p), Some(n)) => (p, n),
                _ => return MirTy::Error,
            };
            ctx.register_name(protocol);
            MirTy::AssociatedProjection {
                base: Box::new(base_ty),
                protocol,
                name,
            }
        },
        ResolvedTy::Tuple(elems) => {
            let lowered: Vec<MirTy> = elems.iter().map(|t| lower_resolved_ty(ctx, t)).collect();
            MirTy::Tuple(lowered)
        },
        ResolvedTy::Function { params, ret } => {
            let lowered_params: Vec<MirTy> =
                params.iter().map(|t| lower_resolved_ty(ctx, t)).collect();
            let lowered_ret = lower_resolved_ty(ctx, ret);
            MirTy::FuncThick {
                params: lowered_params,
                ret: Box::new(lowered_ret),
            }
        },
        ResolvedTy::Opaque {
            origin,
            origin_args,
            ..
        } => {
            let body = ctx.query.query(InferBody {
                entity: *origin,
                root: ctx.root,
            });
            let concrete = body
                .as_ref()
                .and_then(|b| b.opaque_concrete_type.as_ref())
                .cloned()
                .unwrap_or(ResolvedTy::Error);

            let type_params = ctx
                .world
                .get::<TypeParams>(*origin)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();

            let substituted = substitute_resolved_ty(&concrete, &type_params, origin_args);
            lower_resolved_ty(ctx, &substituted)
        },
        ResolvedTy::Never => MirTy::Never,
        ResolvedTy::Error => MirTy::Error,
    }
}

/// Substitute type parameters in a ResolvedTy with concrete types.
/// `type_params` are the origin function's TypeParam entities;
/// `args` are the call-site type arguments (positionally matched).
fn substitute_resolved_ty(
    ty: &ResolvedTy,
    type_params: &[kestrel_hecs::Entity],
    args: &[ResolvedTy],
) -> ResolvedTy {
    match ty {
        ResolvedTy::Param { entity } => {
            for (i, tp) in type_params.iter().enumerate() {
                if tp == entity {
                    if let Some(arg) = args.get(i) {
                        return arg.clone();
                    }
                }
            }
            ty.clone()
        },
        ResolvedTy::Named { entity, args: ty_args } => ResolvedTy::Named {
            entity: *entity,
            args: ty_args
                .iter()
                .map(|a| substitute_resolved_ty(a, type_params, args))
                .collect(),
        },
        ResolvedTy::Tuple(elems) => ResolvedTy::Tuple(
            elems
                .iter()
                .map(|e| substitute_resolved_ty(e, type_params, args))
                .collect(),
        ),
        ResolvedTy::Function {
            params: fn_params,
            ret,
        } => ResolvedTy::Function {
            params: fn_params
                .iter()
                .map(|p| substitute_resolved_ty(p, type_params, args))
                .collect(),
            ret: Box::new(substitute_resolved_ty(ret, type_params, args)),
        },
        ResolvedTy::AssocProjection { base, assoc } => ResolvedTy::AssocProjection {
            base: Box::new(substitute_resolved_ty(base, type_params, args)),
            assoc: *assoc,
        },
        _ => ty.clone(),
    }
}
