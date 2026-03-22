//! Function signature lowering.
//!
//! Lowers function entities into MIR FunctionDefs with params and return types.
//! Function bodies are not lowered in this phase.

use kestrel_ast_builder::{Attributes, Callable, Intrinsic, NodeKind, Static, TypeParams};
use kestrel_hecs::Entity;
use kestrel_mir::{
    FunctionDef, FunctionId, FunctionKind, ReceiverConvention, TypeParamDef,
};

use crate::context::LowerCtx;
use crate::ty::{resolve_callable_types, resolve_type_annotation};

/// Lower a function entity into a MIR FunctionDef (signature only, no body).
pub fn lower_function_sig(ctx: &mut LowerCtx, entity: Entity) -> FunctionId {
    let name = ctx.register_name(entity);

    // Resolve return type from TypeAnnotation
    let ret_ty = resolve_type_annotation(ctx, entity);

    let mut def = FunctionDef::new(entity, &name, ret_ty);

    // Determine FunctionKind based on NodeKind + parent context
    def.kind = determine_function_kind(ctx, entity);

    // Type parameters: parent's first (for methods), then function's own
    collect_inherited_type_params(ctx, entity, &mut def);

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

    // Parameters from Callable component
    if let Some(callable) = ctx.world.get::<Callable>(entity) {
        // Self parameter for methods
        if let Some(receiver) = &callable.receiver {
            let self_ty = match receiver {
                kestrel_ast_builder::ReceiverKind::Borrowing => {
                    kestrel_mir::MirTy::Ref(Box::new(kestrel_mir::MirTy::SelfType))
                },
                kestrel_ast_builder::ReceiverKind::Mutating => {
                    kestrel_mir::MirTy::RefMut(Box::new(kestrel_mir::MirTy::SelfType))
                },
                kestrel_ast_builder::ReceiverKind::Consuming => kestrel_mir::MirTy::SelfType,
            };

            let param = kestrel_mir::ParamDef::new(
                "self",
                kestrel_mir::LocalId::new(0), // placeholder
                self_ty,
            );
            def.params.push(param);
        }

        // Resolve param types via query
        let resolved_types = resolve_callable_types(ctx, entity);
        for (i, ast_param) in callable.params.iter().enumerate() {
            let param_ty = resolved_types
                .get(i)
                .and_then(|t| t.clone())
                .unwrap_or(kestrel_mir::MirTy::Error);
            let local_id = kestrel_mir::LocalId::new(def.params.len());

            let param = kestrel_mir::ParamDef::with_label(
                &ast_param.name,
                local_id,
                param_ty,
                ast_param.label.clone(),
            );
            def.params.push(param);
        }
    }

    // Check for @extern attribute → set extern_info
    if let Some(attrs) = ctx.world.get::<Attributes>(entity) {
        for attr in &attrs.0 {
            if attr.name == "extern" {
                let symbol_name = attr.args.iter()
                    .find(|a| a.label.as_deref() == Some("mangleName"))
                    .map(|a| a.value.trim_matches('"').to_string())
                    .unwrap_or_else(|| {
                        // Fall back to the last segment of the qualified name
                        def.name.rsplit('.').next().unwrap_or(&def.name).to_string()
                    });
                def.extern_info = Some(kestrel_mir::ExternInfo {
                    calling_convention: kestrel_mir::CallingConvention::C,
                    symbol_name,
                });
                break;
            }
        }
    }

    // Intrinsic functions have params but no body — register and stop
    if ctx.world.get::<Intrinsic>(entity).is_some() {
        return ctx.module.add_function(def);
    }

    let func_id = ctx.module.add_function(def);

    // Lower function body if it has one
    if ctx.world.get::<kestrel_ast_builder::Body>(entity).is_some() {
        crate::body_lower::lower_function_body(ctx, entity, func_id);
    }

    func_id
}

/// Determine the FunctionKind based on entity's NodeKind and parent context.
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
        },
        NodeKind::Deinit => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            FunctionKind::Deinit { parent }
        },
        NodeKind::Function => {
            let Some(parent) = ctx.world.parent_of(entity) else {
                return FunctionKind::Free;
            };
            let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
            match parent_kind {
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension) => {
                    // Is it a static method or instance method?
                    if ctx.world.get::<Static>(entity).is_some() {
                        FunctionKind::StaticMethod { parent }
                    } else if let Some(callable) = ctx.world.get::<Callable>(entity) {
                        if callable.receiver.is_some() {
                            let receiver = match callable.receiver.as_ref().unwrap() {
                                kestrel_ast_builder::ReceiverKind::Borrowing => {
                                    ReceiverConvention::Ref
                                },
                                kestrel_ast_builder::ReceiverKind::Mutating => {
                                    ReceiverConvention::RefMut
                                },
                                kestrel_ast_builder::ReceiverKind::Consuming => {
                                    ReceiverConvention::Consuming
                                },
                            };
                            FunctionKind::Method { parent, receiver }
                        } else {
                            // No receiver, no Static marker → static method
                            FunctionKind::StaticMethod { parent }
                        }
                    } else {
                        FunctionKind::StaticMethod { parent }
                    }
                },
                Some(NodeKind::Protocol) => {
                    // Protocol methods — for MIR purposes, treat as free functions
                    // (they'll be referenced via witness tables)
                    FunctionKind::Free
                },
                _ => FunctionKind::Free,
            }
        },
        // Computed property getters — treated as methods
        NodeKind::Field => {
            let parent = ctx.world.parent_of(entity).unwrap_or(ctx.root);
            if let Some(callable) = ctx.world.get::<Callable>(entity) {
                if let Some(receiver) = &callable.receiver {
                    let conv = match receiver {
                        kestrel_ast_builder::ReceiverKind::Borrowing => ReceiverConvention::Ref,
                        kestrel_ast_builder::ReceiverKind::Mutating => ReceiverConvention::RefMut,
                        kestrel_ast_builder::ReceiverKind::Consuming => ReceiverConvention::Consuming,
                    };
                    FunctionKind::Method { parent, receiver: conv }
                } else {
                    FunctionKind::StaticMethod { parent }
                }
            } else {
                FunctionKind::Free
            }
        },
        _ => FunctionKind::Free,
    }
}

/// Collect type parameters inherited from parent types (for methods inside
/// generic structs/enums). These come before the function's own type params.
fn collect_inherited_type_params(ctx: &mut LowerCtx, entity: Entity, def: &mut FunctionDef) {
    let Some(parent) = ctx.world.parent_of(entity) else {
        return;
    };
    let parent_kind = ctx.world.get::<NodeKind>(parent).cloned();
    let needs_parent_params = matches!(
        parent_kind,
        Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Extension)
    );

    if !needs_parent_params {
        return;
    }

    // Determine which entity holds the TypeParams.
    // For structs/enums, it's the parent directly.
    // For extensions, the extension itself has no TypeParams — they're on the
    // target type (e.g., `extend Array[T]` → Array has TypeParams [T]).
    let type_params_source = if matches!(parent_kind, Some(NodeKind::Extension)) {
        // Resolve the extension's target entity and use its TypeParams
        ctx.query.query(kestrel_name_res::ExtensionTargetEntity {
            extension: parent,
            root: ctx.root,
        })
    } else {
        Some(parent)
    };

    let Some(source) = type_params_source else {
        return;
    };

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
