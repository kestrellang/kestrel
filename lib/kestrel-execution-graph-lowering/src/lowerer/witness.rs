//! Witness generation - creates MIR witnesses from protocol conformances.
//!
//! Witnesses are generated from:
//! - Struct conformances: `struct Circle: Drawable { ... }`
//! - Extension conformances: `extend Int: Hashable { ... }`

use kestrel_execution_graph::Id;
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::behavior::conforms_to::ConformsToBehavior;
use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasTypedBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_alias::TypeAliasSymbol;
use kestrel_semantic_tree::ty::{Ty, TyKind};
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::name::qualified_name_for_symbol;
use crate::ty::lower_type;

/// Generate witnesses for all protocol conformances on a struct.
pub fn generate_witnesses_for_struct(
    ctx: &mut LoweringContext,
    struct_symbol: &Arc<StructSymbol>,
) {
    // Get the implementing type (the struct itself)
    let implementing_type = build_struct_type(ctx, struct_symbol);

    // Get conformances
    if let Some(conformances) = struct_symbol.metadata().get_behavior::<ConformancesBehavior>() {
        for protocol_ty in conformances.conformances() {
            generate_witness_for_protocol(
                ctx,
                &(struct_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
            );
        }
    }
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

    let implementing_type = lower_type(ctx, &target_ty);

    // Get conformances added by this extension
    if let Some(conformances) = extension_symbol.metadata().get_behavior::<ConformancesBehavior>() {
        for protocol_ty in conformances.conformances() {
            generate_witness_for_protocol(
                ctx,
                &(extension_symbol.clone() as Arc<dyn Symbol<KestrelLanguage>>),
                implementing_type,
                protocol_ty,
            );
        }
    }
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
            ctx.get_type_param(symbol_id)
                .map(|tp_id| ctx.mir.intern_type(kestrel_execution_graph::MirTy::TypeParam(tp_id)))
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
) {
    // Get the protocol symbol from the type
    let TyKind::Protocol { symbol: protocol_symbol, .. } = protocol_ty.kind() else {
        return;
    };

    // Get qualified name for the protocol
    let protocol_name = qualified_name_for_symbol(ctx, &(protocol_symbol.clone() as _));

    // Create the witness
    let witness_id = ctx.mir.add_witness(implementing_type, protocol_name);

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
            let method_name = if let Some(method_symbol) =
                ctx.model.query(SymbolFor { id: protocol_method_id })
            {
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
