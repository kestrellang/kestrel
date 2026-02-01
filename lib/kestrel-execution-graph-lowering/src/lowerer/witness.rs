//! Witness generation - creates MIR witnesses from protocol conformances.
//!
//! Witnesses are generated from:
//! - Struct conformances: `struct Circle: Drawable { ... }`
//! - Extension conformances: `extend Int: Hashable { ... }`

use kestrel_execution_graph::{Id, TypeParam};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_model::queries::ExtensionsFor;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::conforms_to::ConformsToBehavior;
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::field::FieldSymbol;
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
            // Generate witness for the direct conformance
            generate_witness_for_protocol(
                ctx,
                &(struct_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
                &type_param_ids,
            );

            // Generate derived witnesses from protocol extensions
            // (e.g., if X: Comparable and extend Comparable: Less[Self], generate X: Less)
            generate_derived_witnesses_for_protocol_extensions(
                ctx,
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
            // Generate witness for the direct conformance
            generate_witness_for_protocol(
                ctx,
                &(enum_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
                &type_param_ids,
            );

            // Generate derived witnesses from protocol extensions
            // (e.g., if X: Comparable and extend Comparable: Less[Self], generate X: Less)
            generate_derived_witnesses_for_protocol_extensions(
                ctx,
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

    // Protocol extensions don't generate witnesses directly.
    // When `extend Comparable: Less[Self]` is defined, no witness is created here.
    // Instead, when a concrete type `struct X: Comparable` is processed,
    // the derived witness `X: Less` is generated through
    // `generate_derived_witnesses_for_protocol_extensions()`.
    if matches!(target_ty.kind(), TyKind::Protocol { .. }) {
        return;
    }

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
        },
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
        },
        _ => {},
    }

    // Fallback: Create new type params for the witness
    // This happens when the target type isn't a struct/enum we've lowered
    let mut type_param_ids = Vec::new();
    for tp in &referenced_params {
        let tp_name = tp.metadata().name().value.clone();
        // Use a special witness-owned type param (or function-owned as placeholder)
        let tp_def = kestrel_execution_graph::TypeParamDef::new(
            tp_name,
            TypeParamOwner::Function(Id::from_raw(0u32)), // placeholder
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
    // Get the protocol symbol and substitutions from the type
    let TyKind::Protocol {
        symbol: protocol_symbol,
        substitutions,
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

    // Store protocol type arguments (e.g., Rhs -> Bool for And[Bool])
    // This captures the type arguments used when conforming to a parameterized protocol.
    let protocol_type_params = protocol_symbol.type_parameters();
    for type_param in &protocol_type_params {
        let param_name = type_param.metadata().name().value.clone();
        let param_id = type_param.metadata().id();
        if let Some(sub_ty) = substitutions.get(param_id) {
            let mir_ty = lower_type(ctx, sub_ty);
            ctx.mir.witnesses[witness_id]
                .protocol_type_args
                .insert(param_name, mir_ty);
        }
    }

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

    // Collect protocol method names for fallback matching (including initializers)
    let protocol_method_names: std::collections::HashSet<String> = protocol_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| {
            let kind = c.metadata().kind();
            kind == KestrelSymbolKind::Function || kind == KestrelSymbolKind::Initializer
        })
        .map(|c| c.metadata().name().value.clone())
        .collect();

    for child in implementing_symbol.metadata().children() {
        let child_kind = child.metadata().kind();
        // Handle both functions and initializers (for protocols like Defaultable with init())
        if child_kind != KestrelSymbolKind::Function && child_kind != KestrelSymbolKind::Initializer
        {
            continue;
        }

        let func_name = child.metadata().name().value.clone();

        // First try: Check for ImplementsBehavior pointing to this protocol
        if let Some(implements) = child.metadata().get_behavior::<ImplementsBehavior>()
            && implements.protocol() == protocol_id
        {
            // Get the protocol method name by looking up the symbol
            let protocol_method_id = implements.protocol_method();
            let method_name = if let Some(method_symbol) = ctx.model.query(SymbolFor {
                id: protocol_method_id,
            }) {
                method_symbol.metadata().name().value.clone()
            } else {
                func_name.clone()
            };

            let impl_name = qualified_name_for_symbol(ctx, &child);
            ctx.mir.witnesses[witness_id].bind_method(method_name, impl_name, vec![]);
            continue;
        }

        // Fallback: For extension methods implementing protocol requirements from the same
        // extension's conformance, ImplementsBehavior may not be set (due to signature matching
        // complexities with Self). If the method name matches a protocol method name, bind it.
        if protocol_method_names.contains(&func_name) {
            let impl_name = qualified_name_for_symbol(ctx, &child);
            ctx.mir.witnesses[witness_id].bind_method(func_name, impl_name, vec![]);
        }
    }

    // Bind property getters and setters
    // Protocol property requirements need their getters/setters in the witness table
    // so that T.property can be resolved through witness lookup.
    bind_property_accessors(ctx, witness_id, implementing_symbol, protocol_symbol);
}

/// Bind computed property getters and setters to the witness table.
///
/// For a protocol property requirement like `var value: Int { get set }`,
/// we need witness entries for "get:value" and "set:value" pointing to
/// the implementing type's getter/setter functions.
fn bind_property_accessors(
    ctx: &mut LoweringContext,
    witness_id: Id<kestrel_execution_graph::Witness>,
    implementing_symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    protocol_symbol: &Arc<kestrel_semantic_tree::symbol::protocol::ProtocolSymbol>,
) {
    // Collect protocol property names for fallback matching
    let protocol_property_names: std::collections::HashSet<String> = protocol_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Field)
        .filter_map(|c| {
            let field = c.downcast_arc::<FieldSymbol>().ok()?;
            if field.is_computed() {
                Some(field.metadata().name().value.clone())
            } else {
                None
            }
        })
        .collect();

    // Find implementing fields (computed properties)
    for child in implementing_symbol.metadata().children() {
        if child.metadata().kind() != KestrelSymbolKind::Field {
            continue;
        }

        let Ok(field) = child.clone().downcast_arc::<FieldSymbol>() else {
            continue;
        };

        // Only process computed properties
        if !field.is_computed() {
            continue;
        }

        let field_name = field.metadata().name().value.clone();

        // Check if this field implements a protocol property requirement
        if !protocol_property_names.contains(&field_name) {
            continue;
        }

        // Bind getter if present
        if let Some(getter_id) = field.getter()
            && let Some(getter_sym) = ctx.model.query(SymbolFor { id: getter_id })
        {
            let getter_method_name = format!("get:{}", field_name);
            let impl_name = qualified_name_for_symbol(ctx, &getter_sym);
            ctx.mir.witnesses[witness_id].bind_method(getter_method_name, impl_name, vec![]);
        }

        // Bind setter if present
        if let Some(setter_id) = field.setter()
            && let Some(setter_sym) = ctx.model.query(SymbolFor { id: setter_id })
        {
            let setter_method_name = format!("set:{}", field_name);
            let impl_name = qualified_name_for_symbol(ctx, &setter_sym);
            ctx.mir.witnesses[witness_id].bind_method(setter_method_name, impl_name, vec![]);
        }
    }
}

/// Generate derived witnesses from protocol extensions.
///
/// When a struct/enum conforms to a protocol (e.g., `struct X: Comparable`), and there
/// are extensions on that protocol (e.g., `extend Comparable: Less[Self]`), this function
/// generates witnesses for the derived conformances (e.g., `witness X: Less`).
///
/// The method bindings in the derived witness point to the extension methods with
/// `Self` bound to the implementing type.
pub fn generate_derived_witnesses_for_protocol_extensions(
    ctx: &mut LoweringContext,
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

    let protocol_id = protocol_symbol.metadata().id();

    // Find all extensions on this protocol
    let extensions = ctx.model.query(ExtensionsFor {
        target_id: protocol_id,
    });

    for extension in &extensions {
        // Get conformances added by this extension (e.g., Less[Self], Greater[Self])
        let Some(extension_conformances) =
            extension.metadata().get_behavior::<ConformancesBehavior>()
        else {
            continue;
        };

        for added_protocol_ty in extension_conformances.conformances() {
            // Get the added protocol symbol
            let TyKind::Protocol {
                symbol: added_protocol_symbol,
                ..
            } = added_protocol_ty.kind()
            else {
                continue;
            };

            let added_protocol_name =
                qualified_name_for_symbol(ctx, &(added_protocol_symbol.clone() as _));

            // Create the derived witness
            let witness_id = ctx.mir.add_witness(implementing_type, added_protocol_name);

            // Store the type parameters on the witness
            ctx.mir.witnesses[witness_id].type_params = type_param_ids.to_vec();

            // Bind associated types from the extension
            bind_associated_types_from_extension(ctx, witness_id, extension, added_protocol_symbol);

            // Bind methods from the extension with Self=implementing_type
            bind_methods_from_extension(
                ctx,
                witness_id,
                extension,
                added_protocol_symbol,
                implementing_type,
            );
        }
    }
}

/// Bind associated types in a derived witness from an extension.
fn bind_associated_types_from_extension(
    ctx: &mut LoweringContext,
    witness_id: Id<kestrel_execution_graph::Witness>,
    extension: &Arc<ExtensionSymbol>,
    added_protocol_symbol: &Arc<kestrel_semantic_tree::symbol::protocol::ProtocolSymbol>,
) {
    let added_protocol_id = added_protocol_symbol.metadata().id();

    for child in extension.metadata().children() {
        if child.metadata().kind() != KestrelSymbolKind::TypeAlias {
            continue;
        }

        // Check if this type alias provides a binding for the added protocol
        if let Some(conforms_to) = child.metadata().get_behavior::<ConformsToBehavior>() {
            if conforms_to.protocol().metadata().id() != added_protocol_id {
                continue;
            }

            // Get the aliased type
            if let Ok(alias_symbol) = child.clone().downcast_arc::<TypeAliasSymbol>()
                && let Some(typed_behavior) = alias_symbol
                    .metadata()
                    .get_behavior::<TypeAliasTypedBehavior>()
            {
                let mir_ty = lower_type(ctx, typed_behavior.resolved_ty());
                ctx.mir.witnesses[witness_id].bind_type(conforms_to.associated_type_name(), mir_ty);
            }
        }
    }
}

/// Bind methods in a derived witness from an extension.
///
/// The method bindings point to the extension methods with `Self=implementing_type`.
///
/// Protocol extension methods don't have `ImplementsBehavior` attached because they ARE
/// the default implementations, not struct implementations. So we match by name:
/// for each method required by the protocol, look for a method with that name in the extension.
fn bind_methods_from_extension(
    ctx: &mut LoweringContext,
    witness_id: Id<kestrel_execution_graph::Witness>,
    extension: &Arc<ExtensionSymbol>,
    added_protocol_symbol: &Arc<kestrel_semantic_tree::symbol::protocol::ProtocolSymbol>,
    _implementing_type: Id<kestrel_execution_graph::Ty>,
) {
    // Get the required methods from the protocol
    let protocol_methods: Vec<_> = added_protocol_symbol
        .metadata()
        .children()
        .into_iter()
        .filter(|c| c.metadata().kind() == KestrelSymbolKind::Function)
        .collect();

    // For each required protocol method, look for an implementation in the extension
    for protocol_method in &protocol_methods {
        let method_name = protocol_method.metadata().name().value.clone();

        // Look for a method with this name in the extension
        let extension_method = extension.metadata().children().into_iter().find(|c| {
            c.metadata().kind() == KestrelSymbolKind::Function
                && c.metadata().name().value == method_name
        });

        if let Some(impl_method) = extension_method {
            // Get the extension method's qualified name
            let impl_name = qualified_name_for_symbol(ctx, &impl_method);

            // Protocol extension methods use MirTy::SelfType which gets substituted
            // during monomorphization via FunctionInstantiation.self_type.
            // We don't need to pass type args here - the collector will set self_type
            // when creating the FunctionInstantiation.
            ctx.mir.witnesses[witness_id].bind_method(method_name, impl_name, vec![]);
        }
    }
}
