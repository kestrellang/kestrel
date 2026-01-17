//! Witness generation - creates MIR witnesses from protocol conformances.
//!
//! Witnesses are generated from:
//! - Struct conformances: `struct Circle: Drawable { ... }`
//! - Extension conformances: `extend Int: Hashable { ... }`

use kestrel_execution_graph::{Id, TypeParam};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::conforms_to::ConformsToBehavior;
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::name::qualified_name_for_symbol;
use crate::ty::lower_type;

/// Generate witnesses for all protocol conformances on a struct.
pub fn generate_witnesses_for_struct(ctx: &mut LoweringContext, struct_symbol: &Arc<StructSymbol>) {
    // Re-register type parameters before building the struct type.
    // The type params were cleared after lower_struct completed, but we need them
    // to properly build the implementing_type with type arguments.
    let type_param_ids = register_struct_type_params(ctx, struct_symbol);

    // Get the implementing type (the struct itself)
    let implementing_type = build_struct_type(ctx, struct_symbol);

    // Get conformances
    if let Some(conformances) = struct_symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for protocol_ty in conformances.conformances() {
            generate_witness_for_protocol(
                ctx,
                &(struct_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
                &type_param_ids,
            );
        }
    }

    // Clear type params after generating witnesses
    ctx.clear_type_params();
}

/// Generate witnesses for all protocol conformances on an enum.
pub fn generate_witnesses_for_enum(ctx: &mut LoweringContext, enum_symbol: &Arc<EnumSymbol>) {
    // Re-register type parameters before building the enum type.
    // The type params were cleared after lower_enum completed, but we need them
    // to properly build the implementing_type with type arguments.
    let type_param_ids = register_enum_type_params(ctx, enum_symbol);

    // Get the implementing type (the enum itself)
    let implementing_type = build_enum_type(ctx, enum_symbol);

    // Get conformances
    if let Some(conformances) = enum_symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for protocol_ty in conformances.conformances() {
            generate_witness_for_protocol(
                ctx,
                &(enum_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
                &type_param_ids,
            );
        }
    }

    // Clear type params after generating witnesses
    ctx.clear_type_params();
}

/// Generate witnesses for all protocol conformances added by an extension.
pub fn generate_witnesses_for_extension(
    ctx: &mut LoweringContext,
    extension_symbol: &Arc<ExtensionSymbol>,
) {
    // Get the target type being extended
    let Some(target_ty) = extension_symbol.target_type() else {
        return;
    };

    // Register type parameters from the extension before lowering the target type.
    // Extensions that extend generic types (e.g., `extend Array[T, A]`) have
    // referenced_type_parameters() which need to be in scope.
    let type_param_ids = register_extension_type_params(ctx, extension_symbol);

    let implementing_type = lower_type(ctx, &target_ty);

    // Get conformances added by this extension
    if let Some(conformances) = extension_symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for protocol_ty in conformances.conformances() {
            generate_witness_for_protocol(
                ctx,
                &(extension_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
                &type_param_ids,
            );
        }
    }

    // Clear type params after generating witnesses
    ctx.clear_type_params();
}

/// Register type parameters for a struct symbol.
///
/// This looks up the MIR struct definition (which was already created by lower_struct)
/// and re-registers the mapping from semantic type param IDs to MIR type param IDs.
/// Returns the list of MIR type param IDs for the struct.
fn register_struct_type_params(
    ctx: &mut LoweringContext,
    struct_symbol: &Arc<StructSymbol>,
) -> Vec<Id<TypeParam>> {
    let name = qualified_name_for_symbol(ctx, &(struct_symbol.clone() as _));

    // Find the MIR struct by name
    let struct_id = ctx
        .mir
        .structs
        .iter()
        .find(|(_, def)| def.name == name)
        .map(|(id, _)| id);

    let Some(struct_id) = struct_id else {
        return Vec::new();
    };

    // Get the MIR struct's type params
    let type_param_ids = ctx.mir.structs[struct_id].type_params.clone();

    // Re-register the mapping: semantic type param -> MIR type param
    // The order of type_parameters() matches the order they were registered in lower_struct
    let semantic_type_params: Vec<_> = struct_symbol.type_parameters();
    for (semantic_tp, &mir_tp_id) in semantic_type_params.iter().zip(type_param_ids.iter()) {
        ctx.map_type_param(semantic_tp.metadata().id(), mir_tp_id);
    }

    type_param_ids
}

/// Register type parameters for an enum symbol.
///
/// This looks up the MIR enum definition (which was already created by lower_enum)
/// and re-registers the mapping from semantic type param IDs to MIR type param IDs.
/// Returns the list of MIR type param IDs for the enum.
fn register_enum_type_params(
    ctx: &mut LoweringContext,
    enum_symbol: &Arc<EnumSymbol>,
) -> Vec<Id<TypeParam>> {
    let name = qualified_name_for_symbol(ctx, &(enum_symbol.clone() as _));

    // Find the MIR enum by name
    let enum_id = ctx
        .mir
        .enums
        .iter()
        .find(|(_, def)| def.name == name)
        .map(|(id, _)| id);

    let Some(enum_id) = enum_id else {
        return Vec::new();
    };

    // Get the MIR enum's type params
    let type_param_ids = ctx.mir.enums[enum_id].type_params.clone();

    // Re-register the mapping: semantic type param -> MIR type param
    // The order of type_parameters() matches the order they were registered in lower_enum
    let semantic_type_params: Vec<_> = enum_symbol.type_parameters();
    for (semantic_tp, &mir_tp_id) in semantic_type_params.iter().zip(type_param_ids.iter()) {
        ctx.map_type_param(semantic_tp.metadata().id(), mir_tp_id);
    }

    type_param_ids
}

/// Register type parameters for an extension symbol.
///
/// Extensions reference type parameters from their target type. We look up the MIR
/// struct/enum definition and re-register those type param mappings.
/// Returns the list of MIR type param IDs.
fn register_extension_type_params(
    ctx: &mut LoweringContext,
    extension_symbol: &Arc<ExtensionSymbol>,
) -> Vec<Id<TypeParam>> {
    use kestrel_execution_graph::TypeParamOwner;

    // Get referenced type parameters from the extension
    let referenced_params = extension_symbol.referenced_type_parameters();
    if referenced_params.is_empty() {
        return Vec::new();
    }

    // Get the target type to find its MIR definition
    let Some(target_ty) = extension_symbol.target_type() else {
        return Vec::new();
    };

    // Try to get type params from the target struct/enum's MIR definition
    match target_ty.kind() {
        TyKind::Struct { symbol, .. } => {
            let name = qualified_name_for_symbol(ctx, &(symbol.clone() as _));
            // Find struct and clone type params to avoid borrow issues
            let mir_type_params = ctx
                .mir
                .structs
                .iter()
                .find(|(_, def)| def.name == name)
                .map(|(struct_id, _)| ctx.mir.structs[struct_id].type_params.clone());

            if let Some(mir_type_params) = mir_type_params {
                // Map semantic type param IDs to MIR type param IDs
                for (semantic_tp, &mir_tp_id) in
                    referenced_params.iter().zip(mir_type_params.iter())
                {
                    ctx.map_type_param(semantic_tp.metadata().id(), mir_tp_id);
                }

                return mir_type_params;
            }
        }
        TyKind::Enum { symbol, .. } => {
            let name = qualified_name_for_symbol(ctx, &(symbol.clone() as _));
            // Find enum and clone type params to avoid borrow issues
            let mir_type_params = ctx
                .mir
                .enums
                .iter()
                .find(|(_, def)| def.name == name)
                .map(|(enum_id, _)| ctx.mir.enums[enum_id].type_params.clone());

            if let Some(mir_type_params) = mir_type_params {
                // Map semantic type param IDs to MIR type param IDs
                for (semantic_tp, &mir_tp_id) in
                    referenced_params.iter().zip(mir_type_params.iter())
                {
                    ctx.map_type_param(semantic_tp.metadata().id(), mir_tp_id);
                }

                return mir_type_params;
            }
        }
        _ => {}
    }

    // Fallback: Create new type params for the witness
    // This happens when the target type isn't a struct/enum we've lowered
    let mut type_param_ids = Vec::new();
    for tp in &referenced_params {
        let tp_name = tp.metadata().name().value.clone();
        // Use a special witness-owned type param (or function-owned as placeholder)
        let tp_def = kestrel_execution_graph::TypeParamDef::new(
            tp_name,
            TypeParamOwner::Function(Id::from_raw(0u32.into())), // placeholder
        );
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.map_type_param(tp.metadata().id(), tp_id);
        type_param_ids.push(tp_id);
    }

    type_param_ids
}

/// Build the MIR type for a struct.
fn build_struct_type(
    ctx: &mut LoweringContext,
    struct_symbol: &Arc<StructSymbol>,
) -> Id<kestrel_execution_graph::Ty> {
    let name = qualified_name_for_symbol(ctx, &(struct_symbol.clone() as _));

    // Get type parameters if any
    let type_args: Vec<_> = struct_symbol
        .type_parameters()
        .iter()
        .filter_map(|tp| {
            let symbol_id = tp.metadata().id();
            ctx.get_type_param(symbol_id).map(|tp_id| {
                ctx.mir
                    .intern_type(kestrel_execution_graph::MirTy::TypeParam(tp_id))
            })
        })
        .collect();

    ctx.mir.ty_named(name, type_args)
}

/// Build the MIR type for an enum.
fn build_enum_type(
    ctx: &mut LoweringContext,
    enum_symbol: &Arc<EnumSymbol>,
) -> Id<kestrel_execution_graph::Ty> {
    let name = qualified_name_for_symbol(ctx, &(enum_symbol.clone() as _));

    // Get type parameters if any
    let type_args: Vec<_> = enum_symbol
        .type_parameters()
        .iter()
        .filter_map(|tp| {
            let symbol_id = tp.metadata().id();
            ctx.get_type_param(symbol_id).map(|tp_id| {
                ctx.mir
                    .intern_type(kestrel_execution_graph::MirTy::TypeParam(tp_id))
            })
        })
        .collect();

    ctx.mir.ty_named(name, type_args)
}

/// Generate a witness for a single protocol conformance.
fn generate_witness_for_protocol(
    ctx: &mut LoweringContext,
    implementing_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    implementing_type: Id<kestrel_execution_graph::Ty>,
    protocol_ty: &Ty,
    type_param_ids: &[Id<TypeParam>],
) {
    // Get the protocol symbol from the type
    let TyKind::Protocol {
        symbol: protocol_symbol,
        ..
    } = protocol_ty.kind()
    else {
        return;
    };

    // Get qualified name for the protocol
    let protocol_name = qualified_name_for_symbol(ctx, &(protocol_symbol.clone() as _));

    // Create the witness
    let witness_id = ctx.mir.add_witness(implementing_type, protocol_name);

    // Store the type parameters on the witness so monomorphization can use them
    ctx.mir.witnesses[witness_id].type_params = type_param_ids.to_vec();

    // Bind associated types
    bind_associated_types(ctx, witness_id, implementing_symbol, protocol_symbol);

    // Bind methods
    bind_methods(ctx, witness_id, implementing_symbol, protocol_symbol);
}

/// Bind associated types in the witness from type alias children with ConformsToBehavior.
fn bind_associated_types(
    ctx: &mut LoweringContext,
    witness_id: Id<kestrel_execution_graph::Witness>,
    implementing_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol_symbol: &Arc<kestrel_semantic_tree::symbol::protocol::ProtocolSymbol>,
) {
    let protocol_id = protocol_symbol.metadata().id();

    for child in implementing_symbol.metadata().children() {
        if child.metadata().kind() != KestrelSymbolKind::TypeAlias {
            continue;
        }

        // Check if this type alias provides a binding for our protocol
        if let Some(conforms_to) = child.metadata().get_behavior::<ConformsToBehavior>() {
            if conforms_to.protocol().metadata().id() != protocol_id {
                continue;
            }

            // Get the aliased type
            if let Ok(alias_symbol) = child.clone().downcast_arc::<TypeAliasSymbol>() {
                // Get the resolved type from TypeAliasTypedBehavior
                if let Some(typed_behavior) = alias_symbol
                    .metadata()
                    .get_behavior::<TypeAliasTypedBehavior>()
                {
                    let mir_ty = lower_type(ctx, typed_behavior.resolved_ty());

                    ctx.mir.witnesses[witness_id]
                        .bind_type(conforms_to.associated_type_name(), mir_ty);
                }
            }
        }
    }
}

/// Bind methods in the witness from function children with ImplementsBehavior.
fn bind_methods(
    ctx: &mut LoweringContext,
    witness_id: Id<kestrel_execution_graph::Witness>,
    implementing_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol_symbol: &Arc<kestrel_semantic_tree::symbol::protocol::ProtocolSymbol>,
) {
    let protocol_id = protocol_symbol.metadata().id();

    for child in implementing_symbol.metadata().children() {
        if child.metadata().kind() != KestrelSymbolKind::Function {
            continue;
        }

        // Check if this method implements a protocol method
        if let Some(implements) = child.metadata().get_behavior::<ImplementsBehavior>() {
            if implements.protocol() != protocol_id {
                continue;
            }

            // Get the protocol method name by looking up the symbol
            let protocol_method_id = implements.protocol_method();
            let method_name = if let Some(method_symbol) = ctx.model.query(SymbolFor {
                id: protocol_method_id,
            }) {
                method_symbol.metadata().name().value.clone()
            } else {
                // Fallback to the implementing method's name
                child.metadata().name().value.clone()
            };

            // Get the implementation function's qualified name
            let impl_name = qualified_name_for_symbol(ctx, &child);

            ctx.mir.witnesses[witness_id].bind_method(method_name, impl_name);
        }
    }
}
