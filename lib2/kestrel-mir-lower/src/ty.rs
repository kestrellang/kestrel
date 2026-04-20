//! HirTy → MirTy lowering.
//!
//! Converts resolved HIR types into MIR types. Recognizes lang primitives
//! by checking if the entity's parent is the `lang` module.

use kestrel_ast_builder::{Name, NodeKind};
use kestrel_hecs::{Entity, World};
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableTypes, LowerTypeAnnotation};
use kestrel_mir::MirTy;

use crate::context::LowerCtx;

/// Resolve an entity's TypeAnnotation to MirTy via the query system.
/// Uses name resolution to convert AstType → HirTy, then HirTy → MirTy.
pub fn resolve_type_annotation(ctx: &mut LowerCtx, entity: Entity) -> MirTy {
    let hir_ty = ctx.query.query(LowerTypeAnnotation {
        entity,
        root: ctx.root,
    });
    match hir_ty {
        Some(ty) => lower_type(ctx, &ty),
        None => MirTy::Unit,
    }
}

/// Resolve a Callable entity's parameter types via the query system.
/// Returns None for each param without a type annotation.
pub fn resolve_callable_types(ctx: &mut LowerCtx, entity: Entity) -> Vec<Option<MirTy>> {
    let hir_tys = ctx.query.query(LowerCallableTypes {
        entity,
        root: ctx.root,
    });
    match hir_tys {
        Some(tys) => tys
            .iter()
            .map(|opt_ty| opt_ty.as_ref().map(|ty| lower_type(ctx, ty)))
            .collect(),
        None => Vec::new(),
    }
}

/// Lower a HirTy to a MirTy.
pub fn lower_type(ctx: &mut LowerCtx, ty: &HirTy) -> MirTy {
    match ty {
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => lower_named_type(ctx, *entity, args),
        HirTy::Tuple(elems, _) => {
            let lowered: Vec<MirTy> = elems.iter().map(|t| lower_type(ctx, t)).collect();
            MirTy::Tuple(lowered)
        },
        HirTy::Function { params, ret, .. } => {
            // All function types are thick (can capture environment)
            let lowered_params: Vec<MirTy> = params.iter().map(|t| lower_type(ctx, t)).collect();
            let lowered_ret = lower_type(ctx, ret);
            MirTy::FuncThick {
                params: lowered_params,
                ret: Box::new(lowered_ret),
            }
        },
        HirTy::Param(entity, _) => {
            // Register the type param name for display
            ctx.register_name(*entity);
            MirTy::TypeParam(*entity)
        },
        // Alias uses and associated-type projections should be reduced by
        // inference before MIR. If they reach here something upstream is broken.
        HirTy::AliasUse { .. } | HirTy::AssocProjection { .. } => MirTy::Error,
        HirTy::Never(_) => MirTy::Never,
        HirTy::Infer(_) => MirTy::Error, // shouldn't happen after inference
        HirTy::Error(_) => MirTy::Error,
    }
}

/// Lower a named type — check for lang primitives first, then treat as nominal.
fn lower_named_type(ctx: &mut LowerCtx, entity: Entity, args: &[HirTy]) -> MirTy {
    // Lower type arguments first
    let type_args: Vec<MirTy> = args.iter().map(|t| lower_type(ctx, t)).collect();
    lower_named_type_from_entity(ctx, entity, &type_args)
}

/// Lower a named type from entity + already-lowered MirTy args.
/// Shared between the HirTy and ResolvedTy lowering paths.
pub fn lower_named_type_from_entity(ctx: &mut LowerCtx, entity: Entity, type_args: &[MirTy]) -> MirTy {
    // Check if this is a lang primitive
    if let Some(prim) = try_lang_primitive(ctx.world, entity, type_args) {
        return prim;
    }

    // TypeParameter entities → MirTy::TypeParam
    if let Some(kind) = ctx.world.get::<NodeKind>(entity) {
        if *kind == NodeKind::TypeParameter {
            ctx.register_name(entity);
            return MirTy::TypeParam(entity);
        }
    }

    // Register the entity name for display
    ctx.register_name(entity);

    MirTy::Named {
        entity,
        type_args: type_args.to_vec(),
    }
}

/// Try to recognize a lang primitive type by checking if the entity's parent
/// is the `lang` module and matching on the entity's name.
fn try_lang_primitive(world: &World, entity: Entity, type_args: &[MirTy]) -> Option<MirTy> {
    let parent = world.parent_of(entity)?;
    let parent_kind = world.get::<NodeKind>(parent)?;
    if *parent_kind != NodeKind::Module {
        return None;
    }
    let parent_name = world.get::<Name>(parent)?;
    if parent_name.0 != "lang" {
        return None;
    }

    let name = world.get::<Name>(entity)?;
    match name.0.as_str() {
        "i1" => Some(MirTy::Bool),
        "i8" => Some(MirTy::I8),
        "i16" => Some(MirTy::I16),
        "i32" => Some(MirTy::I32),
        "i64" => Some(MirTy::I64),
        "f16" => Some(MirTy::F16),
        "f32" => Some(MirTy::F32),
        "f64" => Some(MirTy::F64),
        "str" => Some(MirTy::Str),
        "ptr" => {
            if let Some(inner) = type_args.first() {
                Some(MirTy::Pointer(Box::new(inner.clone())))
            } else {
                Some(MirTy::Error)
            }
        },
        _ => None,
    }
}
