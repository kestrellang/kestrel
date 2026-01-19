//! Protocol lowering - converts semantic protocol symbols to MIR protocol definitions.

use kestrel_execution_graph::function::TypeParamOwner;
use kestrel_execution_graph::{Id, Protocol, ProtocolMethod};
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::symbol::associated_type::AssociatedTypeSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::protocol::ProtocolSymbol;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::name::qualified_name_for_symbol;
use crate::ty::lower_type;

/// Lower a protocol definition to MIR.
///
/// This creates a MIR protocol with:
/// - Type parameters
/// - Parent protocol references (for inheritance)
/// - Associated types
/// - Method signatures (not bodies)
pub fn lower_protocol(ctx: &mut LoweringContext, protocol_symbol: &Arc<ProtocolSymbol>) {
    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(protocol_symbol.clone() as _));

    // Create the protocol
    let protocol_id = ctx.mir.add_protocol(name);

    // Lower type parameters
    for tp in protocol_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def = kestrel_execution_graph::function::TypeParamDef::new(
            tp_name,
            TypeParamOwner::Protocol(protocol_id),
        );
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.protocols[protocol_id].type_params.push(tp_id);

        // Register the type param mapping for lowering types within this protocol
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Lower parent protocols from ConformancesBehavior
    if let Some(conformances) = protocol_symbol
        .metadata()
        .get_behavior::<ConformancesBehavior>()
    {
        for parent_ty in conformances.conformances() {
            if let TyKind::Protocol { symbol, .. } = parent_ty.kind() {
                let parent_name = qualified_name_for_symbol(ctx, &(symbol.clone() as _));
                ctx.mir.protocols[protocol_id].add_parent(parent_name);
            }
        }
    }

    // Lower associated types (children with kind AssociatedType)
    for child in protocol_symbol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::AssociatedType {
            if let Ok(assoc_symbol) = child.downcast_arc::<AssociatedTypeSymbol>() {
                lower_protocol_associated_type(ctx, protocol_id, &assoc_symbol);
            }
        }
    }

    // Lower method signatures (children with kind Function)
    for child in protocol_symbol.metadata().children() {
        if child.metadata().kind() == KestrelSymbolKind::Function {
            if let Ok(func_symbol) = child.downcast_arc::<FunctionSymbol>() {
                lower_protocol_method(ctx, protocol_id, &func_symbol);
            }
        }
    }

    // Clear type params after we're done with this protocol
    // Note: This is safe because protocols don't contain nested items that need the type params
    ctx.clear_type_params();
}

/// Lower an associated type in a protocol.
fn lower_protocol_associated_type(
    ctx: &mut LoweringContext,
    protocol_id: Id<Protocol>,
    assoc_symbol: &Arc<AssociatedTypeSymbol>,
) {
    let name = assoc_symbol.metadata().name().value.clone();
    let mut assoc_def = kestrel_execution_graph::item::AssociatedTypeDef::new(name.clone());

    // Handle default type if present
    if let Some(default_ty) = assoc_symbol.default_type() {
        let mir_default = lower_type(ctx, &default_ty);
        assoc_def.default = Some(mir_default);
    }

    let assoc_id = ctx.mir.associated_types.alloc(assoc_def);
    ctx.mir.protocols[protocol_id].add_associated_type(name, assoc_id);
}

/// Lower a protocol method signature.
fn lower_protocol_method(
    ctx: &mut LoweringContext,
    protocol_id: Id<Protocol>,
    func_symbol: &Arc<FunctionSymbol>,
) {
    let name = func_symbol.metadata().name().value.clone();

    // We need to allocate the method ID first so we can use it for type param ownership.
    // We'll create a placeholder return type initially and update it after lowering.
    let placeholder_ret = ctx.mir.ty_unit();
    let method_def =
        kestrel_execution_graph::item::ProtocolMethodDef::new(name.clone(), placeholder_ret);
    let method_id: Id<ProtocolMethod> = ctx.mir.protocol_methods.alloc(method_def);

    // Register the method's own type parameters (e.g., H in `func hash[H](...) where H: Hasher`)
    // This must happen BEFORE we lower any types in the signature.
    for tp in func_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def = kestrel_execution_graph::function::TypeParamDef::new(
            tp_name,
            TypeParamOwner::ProtocolMethod(method_id),
        );
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.protocol_methods[method_id].type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // NOW we can lower types with the method's type parameters in scope

    // Get the return type
    let return_ty = func_symbol.return_type();
    let mir_ret = lower_type(ctx, &return_ty);
    ctx.mir.protocol_methods[method_id].ret = mir_ret;

    // Get callable behavior for parameter info
    if let Some(callable) = func_symbol.metadata().get_behavior::<CallableBehavior>() {
        // Add self parameter based on receiver kind (if not static)
        if let Some(receiver) = callable.receiver() {
            let self_ty = build_self_type_for_receiver(ctx, receiver);
            ctx.mir.protocol_methods[method_id].add_param("self", self_ty);
        }

        // Add regular parameters
        for param in callable.parameters() {
            let param_name = param.internal_name().to_string();
            let param_ty = lower_type(ctx, &param.ty);
            ctx.mir.protocol_methods[method_id].add_param(param_name, param_ty);
        }
    }

    // Note: We skip default method bodies as per design decision
    // has_default is set based on whether the method has a body
    ctx.mir.protocol_methods[method_id].has_default = func_symbol.has_body();

    ctx.mir.protocols[protocol_id].add_method(name, method_id);
}

/// Build the MIR type for the self parameter based on receiver kind.
fn build_self_type_for_receiver(
    ctx: &mut LoweringContext,
    receiver: ReceiverKind,
) -> kestrel_execution_graph::Id<kestrel_execution_graph::Ty> {
    let self_ty = ctx.mir.ty_self();

    match receiver {
        ReceiverKind::Consuming => {
            // consuming: takes ownership of Self
            self_ty
        }
        ReceiverKind::Borrowing => {
            // regular method: &Self
            ctx.mir.ty_ref(self_ty)
        }
        ReceiverKind::Mutating => {
            // mutating method: &var Self
            ctx.mir.ty_ref_mut(self_ty)
        }
        ReceiverKind::Initializing => {
            // initializer: &var Self
            ctx.mir.ty_ref_mut(self_ty)
        }
    }
}
