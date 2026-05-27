//! Function signature lowering.
//!
//! Lowers function entities into MIR FunctionDefs with params and return types.
//! Function bodies are lowered in a separate phase.

use kestrel_ast::AstType;
use kestrel_ast_builder::{
    Attributes, Callable, Intrinsic, NodeKind, Static, TypeParams,
    WhereClause as AstWhereClause, WhereConstraint as AstWhereConstraint,
};
use kestrel_hecs::Entity;
use kestrel_mir_3::item::function::{
    CallingConvention, ExternInfo, FunctionDef, FunctionKind, ParamDef, ReceiverConvention,
    WhereClause, WhereConstraint,
};
use kestrel_mir_3::{ParamConvention, TyId, TypeParamDef, ValueId};
use kestrel_name_res::resolve_type::{ResolveTypePath, TypeResolution};

use crate::context::LowerCtx;
use crate::ty::{resolve_callable_return_type, resolve_callable_types};

/// Lower a function entity into a MIR FunctionDef (signature only, no body).
pub fn lower_function_sig(ctx: &mut LowerCtx, entity: Entity) {
    let name = ctx.register_name(entity);
    let ret_ty = resolve_callable_return_type(ctx, entity);

    let mut def = FunctionDef::new(entity, &name, ret_ty);
    def.kind = determine_function_kind(ctx, entity);

    collect_inherited_type_params(ctx, entity, &mut def);

    let is_extern = ctx
        .world
        .get::<Attributes>(entity)
        .is_some_and(|attrs| attrs.0.iter().any(|a| a.name == "extern"));

    // Function's own type params
    if let Some(type_params) = ctx.world.get::<TypeParams>(entity) {
        for &tp_entity in &type_params.0 {
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    populate_where_clause(ctx, entity, &mut def);

    // Parameters
    if let Some(callable) = ctx.world.get::<Callable>(entity) {
        // Self parameter for methods
        if let Some(receiver) = &callable.receiver {
            let is_user_deinit =
                matches!(ctx.world.get::<NodeKind>(entity), Some(NodeKind::Deinit));
            let convention = if is_user_deinit {
                ParamConvention::MutBorrow
            } else {
                match receiver {
                    kestrel_ast_builder::ReceiverKind::Borrowing => ParamConvention::Borrow,
                    kestrel_ast_builder::ReceiverKind::Mutating => ParamConvention::MutBorrow,
                    kestrel_ast_builder::ReceiverKind::Consuming => ParamConvention::Consuming,
                }
            };
            let self_ty = resolve_self_type_for_function(ctx, entity);
            let value_id = ValueId::new(0); // placeholder
            let param = ParamDef::new("self", value_id, self_ty, convention);
            def.params.push(param);
        }

        let resolved_types = resolve_callable_types(ctx, entity);
        for (i, ast_param) in callable.params.iter().enumerate() {
            let inner_ty = resolved_types
                .get(i)
                .and_then(|t| *t)
                .unwrap_or_else(|| ctx.module.ty_arena.error());
            let value_id = ValueId::new(def.params.len());
            let convention = if is_extern || ast_param.is_consuming {
                ParamConvention::Consuming
            } else if ast_param.is_mut {
                ParamConvention::MutBorrow
            } else {
                ParamConvention::Borrow
            };
            let param = ParamDef::with_label(
                &ast_param.name,
                value_id,
                inner_ty,
                convention,
                ast_param.label.clone(),
            );
            def.params.push(param);
        }
    }

    // @extern
    if is_extern {
        if let Some(attrs) = ctx.world.get::<Attributes>(entity) {
            for attr in &attrs.0 {
                if attr.name == "extern" {
                    let symbol_name = attr
                        .args
                        .iter()
                        .find(|a| a.label.as_deref() == Some("mangleName"))
                        .map(|a| a.value.trim_matches('"').to_string())
                        .unwrap_or_else(|| {
                            def.name.rsplit('.').next().unwrap_or(&def.name).to_string()
                        });
                    def.extern_info = Some(ExternInfo {
                        calling_convention: CallingConvention::C,
                        symbol_name,
                    });
                    break;
                }
            }
        }
    }

    // Intrinsic functions have no body
    if ctx.world.get::<Intrinsic>(entity).is_some() {
        ctx.module.add_function(def);
        return;
    }

    let func_id = ctx.module.add_function(def);

    // Lower function body if present
    if ctx.world.get::<kestrel_ast_builder::Body>(entity).is_some() {
        crate::body::lower_function_body(ctx, entity, func_id.index());
    }
}

fn determine_function_kind(ctx: &LowerCtx, entity: Entity) -> FunctionKind {
    let kind = ctx
        .world
        .get::<NodeKind>(entity)
        .cloned()
        .unwrap_or(NodeKind::Function);

    match kind {
        NodeKind::Initializer => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            FunctionKind::Initializer { parent }
        }
        NodeKind::Deinit => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            FunctionKind::Deinit { parent }
        }
        NodeKind::Function => {
            let Some(parent) = ctx.world.parent_of(entity) else {
                return FunctionKind::Free;
            };
            let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
            match parent_kind {
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension) => {
                    if ctx.world.get::<Static>(entity).is_some() {
                        FunctionKind::StaticMethod { parent }
                    } else if let Some(callable) = ctx.world.get::<Callable>(entity) {
                        if callable.receiver.is_some() {
                            let receiver = match callable.receiver.as_ref().unwrap() {
                                kestrel_ast_builder::ReceiverKind::Borrowing => {
                                    ReceiverConvention::Borrow
                                }
                                kestrel_ast_builder::ReceiverKind::Mutating => {
                                    ReceiverConvention::MutBorrow
                                }
                                kestrel_ast_builder::ReceiverKind::Consuming => {
                                    ReceiverConvention::Consuming
                                }
                            };
                            FunctionKind::Method { parent, receiver }
                        } else {
                            FunctionKind::StaticMethod { parent }
                        }
                    } else {
                        FunctionKind::StaticMethod { parent }
                    }
                }
                Some(NodeKind::Protocol) => FunctionKind::Free,
                _ => FunctionKind::Free,
            }
        }
        NodeKind::Field | NodeKind::Subscript | NodeKind::Setter => {
            let parent = accessor_enclosing_container(ctx, entity).unwrap_or(ctx.root);
            if let Some(callable) = ctx.world.get::<Callable>(entity) {
                if let Some(receiver) = &callable.receiver {
                    let conv = match receiver {
                        kestrel_ast_builder::ReceiverKind::Borrowing => ReceiverConvention::Borrow,
                        kestrel_ast_builder::ReceiverKind::Mutating => {
                            ReceiverConvention::MutBorrow
                        }
                        kestrel_ast_builder::ReceiverKind::Consuming => {
                            ReceiverConvention::Consuming
                        }
                    };
                    FunctionKind::Method {
                        parent,
                        receiver: conv,
                    }
                } else {
                    FunctionKind::StaticMethod { parent }
                }
            } else {
                FunctionKind::Free
            }
        }
        _ => FunctionKind::Free,
    }
}

fn accessor_enclosing_container(ctx: &LowerCtx, entity: Entity) -> Option<Entity> {
    let direct = ctx.world.parent_of(entity)?;
    match ctx.world.get::<NodeKind>(direct).cloned() {
        Some(NodeKind::Field) | Some(NodeKind::Subscript) => ctx.world.parent_of(direct),
        _ => Some(direct),
    }
}

fn collect_inherited_type_params(ctx: &mut LowerCtx, entity: Entity, def: &mut FunctionDef) {
    let Some(parent) =
        accessor_enclosing_container(ctx, entity).or_else(|| ctx.world.parent_of(entity))
    else {
        return;
    };
    let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
    if !matches!(
        parent_kind,
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension)
    ) {
        return;
    }

    // For extensions, type params come from the target type
    let type_params_source = if matches!(parent_kind, Some(NodeKind::Extension)) {
        ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
            extension: parent,
            root: ctx.root,
        })
    } else {
        Some(parent)
    };

    if let Some(source) = type_params_source {
        if let Some(type_params) = ctx.world.get::<TypeParams>(source) {
            for &tp_entity in &type_params.0 {
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
            }
        }
    }

    // Extension's own free type params (e.g. `extend Int64: ArrayIndex[T]`)
    if matches!(parent_kind, Some(NodeKind::Extension))
        && let Some(ext_params) = ctx.world.get::<TypeParams>(parent)
    {
        let already_added: std::collections::HashSet<Entity> =
            def.type_params.iter().map(|tp| tp.entity).collect();
        for &tp_entity in &ext_params.0 {
            if already_added.contains(&tp_entity) {
                continue;
            }
            ctx.register_name(tp_entity);
            let tp_name = ctx
                .world
                .get::<kestrel_ast_builder::Name>(tp_entity)
                .map(|n| n.0.clone())
                .unwrap_or_default();
            def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
        }
    }

    // Setter under generic subscript inherits subscript's type params
    if matches!(ctx.world.get::<NodeKind>(entity), Some(NodeKind::Setter)) {
        let parent_subscript = ctx
            .world
            .parent_of(entity)
            .filter(|p| matches!(ctx.world.get::<NodeKind>(*p), Some(NodeKind::Subscript)));
        if let Some(parent_subscript) = parent_subscript
            && let Some(type_params) = ctx.world.get::<TypeParams>(parent_subscript)
        {
            for &tp_entity in &type_params.0 {
                ctx.register_name(tp_entity);
                let tp_name = ctx
                    .world
                    .get::<kestrel_ast_builder::Name>(tp_entity)
                    .map(|n| n.0.clone())
                    .unwrap_or_default();
                def.type_params.push(TypeParamDef::new(tp_entity, tp_name));
            }
        }
    }
}

fn populate_where_clause(ctx: &mut LowerCtx, entity: Entity, def: &mut FunctionDef) {
    let mut wc = WhereClause::new();
    let mut has_type_params = false;
    let mut current = Some(entity);
    while let Some(e) = current {
        if let Some(ast_wc) = ctx.world.get::<AstWhereClause>(e) {
            for ast_constraint in &ast_wc.0 {
                lower_where_constraint(ctx, ast_constraint, e, &mut wc);
            }
        }
        if ctx
            .world
            .get::<TypeParams>(e)
            .is_some_and(|tp| !tp.0.is_empty())
        {
            has_type_params = true;
        }
        current = ctx.world.parent_of(e);
    }
    if !wc.constraints.is_empty() || has_type_params {
        def.where_clause = Some(wc);
    }
}

fn lower_where_constraint(
    ctx: &mut LowerCtx,
    constraint: &AstWhereConstraint,
    context: Entity,
    out: &mut WhereClause,
) {
    match constraint {
        AstWhereConstraint::Bound {
            subject, protocols, ..
        } => {
            let Some(subject_entity) = resolve_ast_type_to_entity(ctx, subject, context) else {
                return;
            };
            for protocol_ty in protocols {
                let Some(protocol_entity) = resolve_ast_type_to_entity(ctx, protocol_ty, context)
                else {
                    continue;
                };
                // Extract protocol type arguments as entities (e.g., T from SeqIndex[T])
                let proto_type_arg_entities = extract_type_arg_entities(ctx, protocol_ty, context);
                out.add_constraint(WhereConstraint::implements_with_args(
                    subject_entity,
                    protocol_entity,
                    proto_type_arg_entities,
                ));
            }
        }
        AstWhereConstraint::NegativeBound {
            subject, protocol, ..
        } => {
            let Some(subject_entity) = resolve_ast_type_to_entity(ctx, subject, context) else {
                return;
            };
            let Some(protocol_entity) = resolve_ast_type_to_entity(ctx, protocol, context) else {
                return;
            };
            out.add_constraint(WhereConstraint::not_implements(
                subject_entity,
                protocol_entity,
            ));
        }
        AstWhereConstraint::Equality { .. } => {
            // Deferred — not consulted by copy-behavior check
        }
    }
}

/// Extract protocol type argument entities from an AST type like `SeqIndex[T]`.
/// Returns entity IDs for each type arg that resolves to a type parameter.
fn extract_type_arg_entities(
    ctx: &LowerCtx,
    ast_ty: &AstType,
    context: Entity,
) -> Vec<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return Vec::new();
    };
    let Some(last_seg) = segments.last() else {
        return Vec::new();
    };
    last_seg
        .type_args
        .iter()
        .filter_map(|arg| resolve_ast_type_to_entity(ctx, arg, context))
        .collect()
}

fn resolve_ast_type_to_entity(
    ctx: &LowerCtx,
    ast_ty: &AstType,
    context: Entity,
) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_ty else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    match ctx.query.query(ResolveTypePath {
        segments: seg_names,
        context,
        root: ctx.root,
    }) {
        TypeResolution::Found(e) | TypeResolution::NotAType(e) => Some(e),
        TypeResolution::SelfType | TypeResolution::NotFound(_) => None,
    }
}

/// Walk the parent chain from a function entity to find the enclosing
/// struct/enum/protocol and build its Self type as `Named(entity, [TypeParam...])`.
fn resolve_self_type_for_function(ctx: &mut LowerCtx, func_entity: Entity) -> TyId {
    let mut current = ctx.world.parent_of(func_entity);
    while let Some(entity) = current {
        match ctx.world.get::<NodeKind>(entity).cloned() {
            Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol) => {
                return crate::ty::build_self_type(ctx, entity);
            }
            Some(NodeKind::Extension) => {
                // Extension target: resolve the target entity
                if let Some(target) = ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
                    extension: entity,
                    root: ctx.root,
                }) {
                    return crate::ty::build_self_type(ctx, target);
                }
                return ctx.module.ty_arena.error();
            }
            _ => current = ctx.world.parent_of(entity),
        }
    }
    ctx.module.ty_arena.error()
}
