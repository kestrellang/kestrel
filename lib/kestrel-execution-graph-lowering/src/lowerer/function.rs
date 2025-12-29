//! Function and initializer lowering.

use kestrel_execution_graph::TypeParamOwner;
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::executable::ResolvedExecutableBehavior;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;

use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::name::qualified_name_for_symbol;
use crate::stmt::lower_statement;
use crate::ty::lower_type;

/// Lower a function to MIR.
pub fn lower_function(ctx: &mut LoweringContext, func_symbol: &Arc<FunctionSymbol>) {
    // Skip functions without bodies (e.g., protocol methods)
    if !func_symbol.has_body() {
        return;
    }

    // Get the resolved body
    let body = match func_symbol.metadata().get_behavior::<ResolvedExecutableBehavior>() {
        Some(behavior) => behavior.body().clone(),
        None => {
            ctx.emit_error(LoweringError::missing_body(
                func_symbol.metadata().name().value.clone(),
                func_symbol.metadata().span().clone(),
            ));
            return;
        }
    };

    // Get callable behavior for parameter info
    let callable = func_symbol.metadata().get_behavior::<CallableBehavior>();

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(func_symbol.clone() as _));

    // Get return type
    let return_ty = func_symbol.return_type();
    let mir_ret_ty = lower_type(ctx, &return_ty);

    // Prepare self parameter type before creating function (to avoid borrow issues)
    let self_param_ty = if let Some(ref callable) = callable {
        if let Some(receiver) = callable.receiver() {
            compute_self_param_type(ctx, receiver, func_symbol)
        } else {
            None
        }
    } else {
        None
    };

    // Prepare other parameter types
    let param_types: Vec<(String, _)> = if let Some(ref callable) = callable {
        callable
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.internal_name().to_string();
                let mir_ty = lower_type(ctx, &param.ty);
                (param_name, mir_ty)
            })
            .collect()
    } else {
        vec![]
    };

    // Create the function
    let func_id = {
        let mut func = ctx.mir.add_function(name, mir_ret_ty);

        // Add self parameter if this is a method
        if let Some(self_ty) = self_param_ty {
            func.param("self", self_ty);
        }

        // Add parameters
        for (param_name, mir_ty) in param_types {
            func.param(param_name, mir_ty);
        }

        func.id()
    };

    // Lower type parameters for generic functions
    for tp in func_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def = kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);

        // Register the type param mapping for lowering types within this function
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Enter the function context
    ctx.enter_function(func_id);

    // Map semantic locals to MIR locals
    // Parameters are already created, map them first
    // Copy the locals vector to avoid borrow issues
    let (param_count, mir_locals) = {
        let func_def = ctx.mir.function(func_id);
        (func_def.params.len(), func_def.locals.clone())
    };

    // Get all locals from the semantic function
    let locals = func_symbol.locals();

    // First, map parameter locals (they were already created)
    for (i, local) in locals.iter().take(param_count).enumerate() {
        let mir_local_id = mir_locals[i];
        ctx.map_local(local.id(), mir_local_id);
    }

    // Then create and map non-parameter locals
    for local in locals.iter().skip(param_count) {
        let mir_ty = lower_type(ctx, local.ty());
        let mir_local_id = ctx.create_local(local.name().to_string(), mir_ty);
        ctx.map_local(local.id(), mir_local_id);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Lower statements
    for stmt in &body.statements {
        lower_statement(ctx, stmt);

        // If the block is terminated, we can't add more statements
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Lower yield expression if present
    if !ctx.is_block_terminated() {
        if let Some(yield_expr) = body.yield_expr.as_ref() {
            let value = lower_expression(ctx, yield_expr);
            if !ctx.is_block_terminated() {
                ctx.emit_return(value);
            }
        } else {
            // No yield expression - return unit
            ctx.emit_return_unit();
        }
    }

    ctx.exit_function();

    // Clear type param mappings after exiting function
    ctx.clear_type_params();
}

/// Lower an initializer to MIR.
///
/// Initializers are lowered as functions with signature:
/// `func Type.init(self: &var Type, params...) -> ()`
pub fn lower_initializer(ctx: &mut LoweringContext, init_symbol: &Arc<InitializerSymbol>) {
    // Get the resolved body
    let body = match init_symbol.metadata().get_behavior::<ResolvedExecutableBehavior>() {
        Some(behavior) => behavior.body().clone(),
        None => {
            ctx.emit_error(LoweringError::missing_body(
                "init",
                init_symbol.metadata().span().clone(),
            ));
            return;
        }
    };

    // Get callable behavior for parameter info
    let callable = init_symbol.metadata().get_behavior::<CallableBehavior>();

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(init_symbol.clone() as _));

    // Initializers always return unit
    let mir_ret_ty = ctx.mir.ty_unit();

    // Prepare self parameter type before creating function (to avoid borrow issues)
    let self_param_ty = if let Some(parent) = init_symbol.metadata().parent() {
        let parent_name = qualified_name_for_symbol(ctx, &parent);
        let parent_ty = ctx.mir.ty_named(parent_name, vec![]);
        Some(ctx.mir.ty_ref_mut(parent_ty))
    } else {
        None
    };

    // Prepare other parameter types
    let param_types: Vec<(String, _)> = if let Some(ref callable) = callable {
        callable
            .parameters()
            .iter()
            .map(|param| {
                let param_name = param.internal_name().to_string();
                let mir_ty = lower_type(ctx, &param.ty);
                (param_name, mir_ty)
            })
            .collect()
    } else {
        vec![]
    };

    // Create the function
    let func_id = {
        let mut func = ctx.mir.add_function(name, mir_ret_ty);

        // Add self parameter
        if let Some(self_ty) = self_param_ty {
            func.param("self", self_ty);
        }

        // Add other parameters
        for (param_name, mir_ty) in param_types {
            func.param(param_name, mir_ty);
        }

        func.id()
    };

    // Enter the function context
    ctx.enter_function(func_id);

    // Map semantic locals to MIR locals
    // Copy the locals vector to avoid borrow issues
    let (param_count, mir_locals) = {
        let func_def = ctx.mir.function(func_id);
        (func_def.params.len(), func_def.locals.clone())
    };

    let locals = init_symbol.locals();

    // Map parameter locals
    for (i, local) in locals.iter().take(param_count).enumerate() {
        let mir_local_id = mir_locals[i];
        ctx.map_local(local.id(), mir_local_id);
    }

    // Create and map non-parameter locals
    for local in locals.iter().skip(param_count) {
        let mir_ty = lower_type(ctx, local.ty());
        let mir_local_id = ctx.create_local(local.name().to_string(), mir_ty);
        ctx.map_local(local.id(), mir_local_id);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Lower statements
    for stmt in &body.statements {
        lower_statement(ctx, stmt);

        if ctx.is_block_terminated() {
            break;
        }
    }

    // Initializers implicitly return unit
    if !ctx.is_block_terminated() {
        ctx.emit_return_unit();
    }

    ctx.exit_function();
}

/// Compute the self parameter type for a method.
fn compute_self_param_type(
    ctx: &mut LoweringContext,
    receiver: ReceiverKind,
    func_symbol: &Arc<FunctionSymbol>,
) -> Option<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> {
    // Get the parent type for self
    let parent = func_symbol.metadata().parent()?;

    let parent_name = qualified_name_for_symbol(ctx, &parent);
    let parent_ty = ctx.mir.ty_named(parent_name, vec![]);

    // Create the self parameter type based on receiver kind
    let self_ty = match receiver {
        ReceiverKind::Borrowing => ctx.mir.ty_ref(parent_ty),
        ReceiverKind::Mutating => ctx.mir.ty_ref_mut(parent_ty),
        ReceiverKind::Consuming => parent_ty,
        ReceiverKind::Initializing => ctx.mir.ty_ref_mut(parent_ty),
    };

    Some(self_ty)
}
