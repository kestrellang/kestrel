//! HirTy / ResolvedTy → TyId lowering.
//!
//! All type lowering returns `TyId` (interned). No `MirTy` clones flow
//! through the pipeline — the arena deduplicates automatically.

use std::cell::RefCell;
use std::collections::HashSet;

use kestrel_ast_builder::{Name, NodeKind, TypeParams};
use kestrel_hecs::Entity;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::{LowerCallableReturnType, LowerCallableTypes, LowerTypeAnnotation};
use kestrel_mir_3::{MirTy, ParamConvention, TyId};
use kestrel_type_infer::InferBody;
use kestrel_type_infer::result::ResolvedTy;

use crate::context::LowerCtx;

// === Public query wrappers ===

/// Resolve an entity's TypeAnnotation to TyId via the query system.
pub fn resolve_type_annotation(ctx: &mut LowerCtx, entity: Entity) -> TyId {
    let hir_ty = ctx.query.query(LowerTypeAnnotation {
        entity,
        root: ctx.root,
    });
    match hir_ty {
        Some(ty) => lower_type(ctx, &ty),
        None => ctx.module.ty_arena.unit(),
    }
}

/// Resolve a callable entity's return type, handling opaque return types.
pub fn resolve_callable_return_type(ctx: &mut LowerCtx, entity: Entity) -> TyId {
    let hir_ty = ctx.query.query(LowerCallableReturnType {
        entity,
        root: ctx.root,
    });
    if contains_opaque(&hir_ty) {
        let body = ctx.query.query(InferBody {
            entity,
            root: ctx.root,
        });
        if let Some(concrete) = body.as_ref().and_then(|b| b.opaque_concrete_type.as_ref()) {
            return lower_type_replacing_opaque(ctx, &hir_ty, concrete);
        }
    }
    lower_type(ctx, &hir_ty)
}

/// Resolve a callable entity's parameter types. Returns None per param
/// without a type annotation.
pub fn resolve_callable_types(ctx: &mut LowerCtx, entity: Entity) -> Vec<Option<TyId>> {
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

// === Self type resolution ===

/// Build the concrete type for `Self` given a protocol (or struct/enum) entity.
///
/// - **Struct/Enum**: `Named(entity, [TypeParam(tp)...])` — a concrete type.
/// - **Protocol**: `TypeParam(protocol_entity)` — Self in a protocol is genuinely
///   a type parameter (there are no instances of protocols). The monomorphizer
///   substitutes it with the concrete conforming type via `InstantiationKey.self_type`.
pub fn build_self_type(ctx: &mut LowerCtx, entity: Entity) -> TyId {
    if ctx.world.get::<NodeKind>(entity) == Some(&NodeKind::Protocol) {
        ctx.register_name(entity);
        return ctx.intern(MirTy::TypeParam(entity));
    }
    let type_args: Vec<TyId> = ctx
        .world
        .get::<TypeParams>(entity)
        .map(|tp| {
            tp.0.iter()
                .map(|&tp_entity| {
                    ctx.register_name(tp_entity);
                    ctx.intern(MirTy::TypeParam(tp_entity))
                })
                .collect()
        })
        .unwrap_or_default();
    ctx.register_name(entity);
    ctx.module.ty_arena.named(entity, type_args)
}

// === HirTy → TyId ===

/// Lower a HirTy to an interned TyId.
pub fn lower_type(ctx: &mut LowerCtx, ty: &HirTy) -> TyId {
    match ty {
        HirTy::SelfType(entity, _) => build_self_type(ctx, *entity),
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            let type_args: Vec<TyId> = args.iter().map(|a| lower_type(ctx, a)).collect();
            lower_named_type(ctx, *entity, type_args)
        }
        HirTy::Tuple(elems, _) => {
            let elems: Vec<TyId> = elems.iter().map(|e| lower_type(ctx, e)).collect();
            ctx.module.ty_arena.tuple(elems)
        }
        HirTy::Function { params, ret, .. } => {
            let lowered_params: Vec<(TyId, ParamConvention)> = params
                .iter()
                .map(|t| (lower_type(ctx, t), ParamConvention::Consuming))
                .collect();
            let lowered_ret = lower_type(ctx, ret);
            ctx.intern(MirTy::FuncThick {
                params: lowered_params,
                ret: lowered_ret,
            })
        }
        HirTy::Param(entity, _) => {
            ctx.register_name(*entity);
            ctx.intern(MirTy::TypeParam(*entity))
        }
        HirTy::AliasUse { .. } => ctx.module.ty_arena.error(),
        HirTy::AssocProjection { base, assoc, .. } => {
            lower_assoc_projection(ctx, base, *assoc)
        }
        HirTy::Opaque { .. } => ctx.module.ty_arena.error(),
        HirTy::Never(_) => ctx.module.ty_arena.never(),
        HirTy::Infer(_) => ctx.module.ty_arena.error(),
        HirTy::Error(_) => ctx.module.ty_arena.error(),
    }
}

// === ResolvedTy → TyId ===

thread_local! {
    static OPAQUE_RESOLVE_STACK: RefCell<HashSet<Entity>> = RefCell::new(HashSet::new());
}

/// Lower a ResolvedTy (from type inference) to an interned TyId.
pub fn lower_resolved_ty(ctx: &mut LowerCtx, ty: &ResolvedTy) -> TyId {
    match ty {
        ResolvedTy::Named { entity, args } => {
            let mir_args: Vec<TyId> = args.iter().map(|a| lower_resolved_ty(ctx, a)).collect();
            lower_named_type(ctx, *entity, mir_args)
        }
        ResolvedTy::Param { entity } => {
            ctx.register_name(*entity);
            ctx.intern(MirTy::TypeParam(*entity))
        }
        ResolvedTy::SelfType { entity } => build_self_type(ctx, *entity),
        ResolvedTy::AssocProjection { base, assoc } => {
            let base_ty = lower_resolved_ty(ctx, base);
            let Some(protocol) = ctx.world.parent_of(*assoc) else {
                return ctx.module.ty_arena.error();
            };
            ctx.register_name(protocol);
            ctx.intern(MirTy::AssociatedProjection {
                base: base_ty,
                protocol,
                assoc_type: *assoc,
            })
        }
        ResolvedTy::Tuple(elems) => {
            let lowered: Vec<TyId> = elems.iter().map(|t| lower_resolved_ty(ctx, t)).collect();
            ctx.module.ty_arena.tuple(lowered)
        }
        ResolvedTy::Function { params, ret } => {
            let lowered_params: Vec<(TyId, ParamConvention)> = params
                .iter()
                .map(|t| (lower_resolved_ty(ctx, t), ParamConvention::Consuming))
                .collect();
            let lowered_ret = lower_resolved_ty(ctx, ret);
            ctx.intern(MirTy::FuncThick {
                params: lowered_params,
                ret: lowered_ret,
            })
        }
        ResolvedTy::Opaque {
            origin,
            origin_args,
            ..
        } => {
            let is_cycle =
                OPAQUE_RESOLVE_STACK.with(|stack| !stack.borrow_mut().insert(*origin));
            if is_cycle {
                return ctx.module.ty_arena.error();
            }

            let body = ctx.query.query(InferBody {
                entity: *origin,
                root: ctx.root,
            });
            let concrete = body
                .as_ref()
                .and_then(|b| b.opaque_concrete_type.as_ref())
                .cloned()
                .unwrap_or_else(|| {
                    panic!("ICE: opaque type origin {:?} has no concrete type", origin)
                });

            let type_params = ctx
                .world
                .get::<TypeParams>(*origin)
                .map(|tp| tp.0.clone())
                .unwrap_or_default();

            let substituted = substitute_resolved_ty(&concrete, &type_params, origin_args);
            let result = lower_resolved_ty(ctx, &substituted);

            OPAQUE_RESOLVE_STACK.with(|stack| stack.borrow_mut().remove(origin));
            result
        }
        ResolvedTy::Never => ctx.module.ty_arena.never(),
        ResolvedTy::Error => ctx.module.ty_arena.error(),
    }
}

// === Shared helpers ===

/// Lower a named type: check for lang primitives, type parameters, and
/// protocol associated types before falling through to Named.
/// Shared by both the HirTy and ResolvedTy paths.
pub fn lower_named_type(ctx: &mut LowerCtx, entity: Entity, type_args: Vec<TyId>) -> TyId {
    if let Some(prim) = try_lang_primitive(ctx, entity, &type_args) {
        return prim;
    }

    if let Some(kind) = ctx.world.get::<NodeKind>(entity).cloned() {
        if kind == NodeKind::TypeParameter {
            ctx.register_name(entity);
            return ctx.intern(MirTy::TypeParam(entity));
        }
        // Abstract associated type (TypeAlias child of a Protocol) leaking
        // through inference — wrap as AssociatedProjection.
        if kind == NodeKind::TypeAlias
            && let Some(parent) = ctx.world.parent_of(entity)
            && ctx.world.get::<NodeKind>(parent) == Some(&NodeKind::Protocol)
        {
            ctx.register_name(parent);
            ctx.register_name(entity);
            let self_ty = build_self_type(ctx, parent);
            return ctx.intern(MirTy::AssociatedProjection {
                base: self_ty,
                protocol: parent,
                assoc_type: entity,
            });
        }
    }

    ctx.register_name(entity);
    ctx.module.ty_arena.named(entity, type_args)
}

/// Lower an AssocProjection from HirTy.
fn lower_assoc_projection(ctx: &mut LowerCtx, base: &HirTy, assoc: Entity) -> TyId {
    let Some(protocol) = ctx.world.parent_of(assoc) else {
        return ctx.module.ty_arena.error();
    };
    let base_ty = lower_type(ctx, base);
    ctx.register_name(protocol);
    ctx.intern(MirTy::AssociatedProjection {
        base: base_ty,
        protocol,
        assoc_type: assoc,
    })
}

/// Recognize lang primitive types by checking the entity's parent is the
/// `lang` module.
fn try_lang_primitive(ctx: &mut LowerCtx, entity: Entity, type_args: &[TyId]) -> Option<TyId> {
    let parent = ctx.world.parent_of(entity)?;
    let parent_kind = ctx.world.get::<NodeKind>(parent)?;
    if *parent_kind != NodeKind::Module {
        return None;
    }
    let parent_name = ctx.world.get::<Name>(parent)?;
    if parent_name.0 != "lang" {
        return None;
    }

    let name = ctx.world.get::<Name>(entity)?;
    match name.0.as_str() {
        "i1" => Some(ctx.module.ty_arena.bool()),
        "i8" => Some(ctx.module.ty_arena.i8()),
        "i16" => Some(ctx.module.ty_arena.i16()),
        "i32" => Some(ctx.module.ty_arena.i32()),
        "i64" => Some(ctx.module.ty_arena.i64()),
        "f16" => Some(ctx.module.ty_arena.f16()),
        "f32" => Some(ctx.module.ty_arena.f32()),
        "f64" => Some(ctx.module.ty_arena.f64()),
        "str" => Some(ctx.module.ty_arena.str_ty()),
        "ptr" => {
            let inner = type_args.first().copied()?;
            Some(ctx.module.ty_arena.pointer(inner))
        }
        _ => None,
    }
}

/// Walk HirTy, replacing Opaque nodes with the concrete type from inference.
fn lower_type_replacing_opaque(
    ctx: &mut LowerCtx,
    ty: &HirTy,
    concrete: &ResolvedTy,
) -> TyId {
    match ty {
        HirTy::Opaque { .. } => lower_resolved_ty(ctx, concrete),
        HirTy::Struct { entity, args, .. }
        | HirTy::Enum { entity, args, .. }
        | HirTy::Protocol { entity, args, .. } => {
            let type_args: Vec<TyId> = args
                .iter()
                .map(|a| {
                    if contains_opaque(a) {
                        lower_type_replacing_opaque(ctx, a, concrete)
                    } else {
                        lower_type(ctx, a)
                    }
                })
                .collect();
            lower_named_type(ctx, *entity, type_args)
        }
        HirTy::Tuple(elems, _) => {
            let lowered: Vec<TyId> = elems
                .iter()
                .map(|e| {
                    if contains_opaque(e) {
                        lower_type_replacing_opaque(ctx, e, concrete)
                    } else {
                        lower_type(ctx, e)
                    }
                })
                .collect();
            ctx.module.ty_arena.tuple(lowered)
        }
        HirTy::Function { params, ret, .. } => {
            let lowered_params: Vec<(TyId, ParamConvention)> = params
                .iter()
                .map(|p| {
                    let ty = if contains_opaque(p) {
                        lower_type_replacing_opaque(ctx, p, concrete)
                    } else {
                        lower_type(ctx, p)
                    };
                    (ty, ParamConvention::Consuming)
                })
                .collect();
            let lowered_ret = if contains_opaque(ret) {
                lower_type_replacing_opaque(ctx, ret, concrete)
            } else {
                lower_type(ctx, ret)
            };
            ctx.intern(MirTy::FuncThick {
                params: lowered_params,
                ret: lowered_ret,
            })
        }
        HirTy::AssocProjection { base, assoc, .. } if contains_opaque(base) => {
            let base_ty = lower_type_replacing_opaque(ctx, base, concrete);
            let Some(protocol) = ctx.world.parent_of(*assoc) else {
                return ctx.module.ty_arena.error();
            };
            ctx.register_name(protocol);
            ctx.intern(MirTy::AssociatedProjection {
                base: base_ty,
                protocol,
                assoc_type: *assoc,
            })
        }
        _ => lower_type(ctx, ty),
    }
}

fn contains_opaque(ty: &HirTy) -> bool {
    match ty {
        HirTy::Opaque { .. } => true,
        HirTy::Struct { args, .. }
        | HirTy::Enum { args, .. }
        | HirTy::Protocol { args, .. }
        | HirTy::AliasUse { args, .. } => args.iter().any(contains_opaque),
        HirTy::Tuple(elems, _) => elems.iter().any(contains_opaque),
        HirTy::Function { params, ret, .. } => {
            params.iter().any(contains_opaque) || contains_opaque(ret)
        }
        HirTy::AssocProjection { base, .. } => contains_opaque(base),
        _ => false,
    }
}

/// Substitute type params in a ResolvedTy (for opaque return type resolution).
fn substitute_resolved_ty(
    ty: &ResolvedTy,
    type_params: &[Entity],
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
        }
        ResolvedTy::Named {
            entity,
            args: ty_args,
        } => ResolvedTy::Named {
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

#[cfg(test)]
mod tests {
    use super::*;
    use kestrel_compiler::Compiler;
    use kestrel_hecs::World;
    use kestrel_mir_3::MirTy;

    fn stdlib_path() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../lang/std")
            .canonicalize()
            .expect("stdlib path should exist at lang/std")
    }

    #[test]
    fn lower_type_primitives() {
        let mut ctx = setup_stdlib_ctx();
        let hir_ty_i64 = HirTy::Struct {
            entity: find_lang_entity(ctx.world, "i64"),
            args: vec![],
            span: kestrel_span::Span::synthetic(0),
        };
        let ty = lower_type(&mut ctx, &hir_ty_i64);
        assert_eq!(ctx.module.ty_arena.get(ty), &MirTy::I64);
    }

    #[test]
    fn lower_type_tuple() {
        let mut ctx = setup_stdlib_ctx();
        let i64_entity = find_lang_entity(ctx.world, "i64");
        let bool_entity = find_lang_entity(ctx.world, "i1");
        let hir = HirTy::Tuple(
            vec![
                HirTy::Struct {
                    entity: i64_entity,
                    args: vec![],
                    span: kestrel_span::Span::synthetic(0),
                },
                HirTy::Struct {
                    entity: bool_entity,
                    args: vec![],
                    span: kestrel_span::Span::synthetic(0),
                },
            ],
            kestrel_span::Span::synthetic(0),
        );
        let ty = lower_type(&mut ctx, &hir);
        match ctx.module.ty_arena.get(ty) {
            MirTy::Tuple(elems) => {
                assert_eq!(elems.len(), 2);
                assert_eq!(ctx.module.ty_arena.get(elems[0]), &MirTy::I64);
                assert_eq!(ctx.module.ty_arena.get(elems[1]), &MirTy::Bool);
            }
            other => panic!("expected Tuple, got {other:?}"),
        }
    }

    #[test]
    fn lower_type_self() {
        let mut ctx = setup_stdlib_ctx();
        let entity = Entity::from_raw(1);
        // Entity 1 has no NodeKind, so build_self_type treats it as non-protocol
        // and produces Named(entity, [])
        let hir = HirTy::SelfType(entity, kestrel_span::Span::synthetic(0));
        let ty = lower_type(&mut ctx, &hir);
        assert_eq!(
            ctx.module.ty_arena.get(ty),
            &MirTy::Named { entity, type_args: vec![] }
        );
    }

    #[test]
    fn lower_type_never() {
        let mut ctx = setup_stdlib_ctx();
        let hir = HirTy::Never(kestrel_span::Span::synthetic(0));
        let ty = lower_type(&mut ctx, &hir);
        assert_eq!(ctx.module.ty_arena.get(ty), &MirTy::Never);
    }

    #[test]
    fn lower_type_error() {
        let mut ctx = setup_stdlib_ctx();
        let hir = HirTy::Error(kestrel_span::Span::synthetic(0));
        let ty = lower_type(&mut ctx, &hir);
        assert_eq!(ctx.module.ty_arena.get(ty), &MirTy::Error);
    }

    #[test]
    fn lower_type_type_param() {
        let mut ctx = setup_stdlib_ctx();
        let param_entity = Entity::from_raw(999);
        let hir = HirTy::Param(param_entity, kestrel_span::Span::synthetic(0));
        let ty = lower_type(&mut ctx, &hir);
        assert_eq!(ctx.module.ty_arena.get(ty), &MirTy::TypeParam(param_entity));
    }

    #[test]
    fn lower_resolved_ty_primitives() {
        let mut ctx = setup_stdlib_ctx();
        let i64_entity = find_lang_entity(ctx.world, "i64");
        let resolved = ResolvedTy::Named {
            entity: i64_entity,
            args: vec![],
        };
        let ty = lower_resolved_ty(&mut ctx, &resolved);
        assert_eq!(ctx.module.ty_arena.get(ty), &MirTy::I64);
    }

    #[test]
    fn stdlib_field_types_resolve() {
        let c = setup_stdlib_compiler();
        let mut ctx = LowerCtx::new(c.world(), c.root(), "test");

        // Lower all struct field types via type annotations
        let children: Vec<Entity> = ctx.world.children_of(ctx.root).to_vec();
        let mut error_count = 0;
        let mut total_count = 0;
        visit_fields(ctx.world, &children, &mut |field_entity| {
            total_count += 1;
            let ty = resolve_type_annotation(&mut ctx, field_entity);
            if ctx.module.ty_arena.get(ty) == &MirTy::Error {
                error_count += 1;
            }
        });

        assert!(
            total_count > 0,
            "should have found some fields to resolve"
        );
        assert!(
            error_count < total_count / 2,
            "too many unresolved field types: {error_count}/{total_count}"
        );
    }

    // --- Test helpers ---

    fn setup_stdlib_compiler() -> Compiler {
        let mut c = Compiler::new();
        let path = stdlib_path();
        c.load_dir(&path);
        c
    }

    fn setup_stdlib_ctx() -> LowerCtx<'static> {
        // Leak the compiler to get a 'static World ref for simpler test code.
        // Only used in tests — the leak is bounded by test process lifetime.
        let c = Box::leak(Box::new(setup_stdlib_compiler()));
        LowerCtx::new(c.world(), c.root(), "test")
    }

    fn find_lang_entity(world: &World, name: &str) -> Entity {
        for &child in world.children_of(Entity::from_raw(0)) {
            if world.get::<Name>(child).is_some_and(|n| n.0 == "lang") {
                for &grandchild in world.children_of(child) {
                    if world.get::<Name>(grandchild).is_some_and(|n| n.0 == name) {
                        return grandchild;
                    }
                }
            }
        }
        // Walk modules under root
        fn walk(world: &World, entity: Entity, target: &str) -> Option<Entity> {
            for &child in world.children_of(entity) {
                if let Some(kind) = world.get::<NodeKind>(child) {
                    if *kind == NodeKind::Module
                        && world.get::<Name>(child).is_some_and(|n| n.0 == "lang")
                    {
                        for &gc in world.children_of(child) {
                            if world.get::<Name>(gc).is_some_and(|n| n.0 == target) {
                                return Some(gc);
                            }
                        }
                    }
                    if *kind == NodeKind::Module {
                        if let Some(found) = walk(world, child, target) {
                            return Some(found);
                        }
                    }
                }
            }
            None
        }
        let root = world
            .children_of(Entity::from_raw(0))
            .first()
            .copied()
            .unwrap_or(Entity::from_raw(0));
        walk(world, root, name).unwrap_or_else(|| panic!("lang.{name} not found"))
    }

    fn visit_fields(world: &World, entities: &[Entity], visitor: &mut dyn FnMut(Entity)) {
        for &entity in entities {
            let kind = world.get::<NodeKind>(entity);
            if kind == Some(&NodeKind::Field)
                && world
                    .get::<kestrel_ast_builder::Callable>(entity)
                    .is_none()
                && world.get::<kestrel_ast_builder::Static>(entity).is_none()
            {
                visitor(entity);
            }
            let children: Vec<Entity> = world.children_of(entity).to_vec();
            visit_fields(world, &children, visitor);
        }
    }
}
