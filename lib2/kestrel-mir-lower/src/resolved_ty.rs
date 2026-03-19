//! ResolvedTy → MirTy conversion.
//!
//! ResolvedTy comes from type inference results. It has the same shape as HirTy
//! but without spans and without Infer.

use kestrel_type_infer::result::ResolvedTy;
use kestrel_mir::MirTy;

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
        ResolvedTy::Tuple(elems) => {
            let lowered: Vec<MirTy> = elems.iter().map(|t| lower_resolved_ty(ctx, t)).collect();
            MirTy::Tuple(lowered)
        },
        ResolvedTy::Function { params, ret } => {
            let lowered_params: Vec<MirTy> = params.iter().map(|t| lower_resolved_ty(ctx, t)).collect();
            let lowered_ret = lower_resolved_ty(ctx, ret);
            MirTy::FuncThick {
                params: lowered_params,
                ret: Box::new(lowered_ret),
            }
        },
        ResolvedTy::Never => MirTy::Never,
        ResolvedTy::Error => MirTy::Error,
    }
}
