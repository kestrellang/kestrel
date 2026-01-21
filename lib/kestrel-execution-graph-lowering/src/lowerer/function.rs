//! Function and initializer lowering.

use std::collections::HashSet;
use std::sync::Arc;

use kestrel_execution_graph::CallingConvention as MirCallingConvention;
use kestrel_execution_graph::TypeParamOwner;
use kestrel_semantic_tree::behavior::callable::{
    CallableBehavior, ParameterAccessMode, ReceiverKind,
};
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ResolvedExecutableBehavior};
use kestrel_semantic_tree::behavior::extern_fn::{CallingConvention, ExternBehavior};
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::getter::GetterSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::symbol::setter::SetterSymbol;
use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
use kestrel_semantic_tree::symbol::type_parameter::TypeParameterSymbol;
use semantic_tree::symbol::Symbol;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::name::qualified_name_for_symbol;
use crate::stmt::lower_statement;
use crate::ty::lower_type;

/// Lower a function to MIR.
pub fn lower_function(ctx: &mut LoweringContext, func_symbol: &Arc<FunctionSymbol>) {
    // Check if this is an extern function
    let extern_behavior = func_symbol.metadata().get_behavior::<ExternBehavior>();

    // For non-extern functions without bodies (e.g., protocol methods), skip
    if extern_behavior.is_none() && !func_symbol.has_body() {
        return;
    }

    // Get the resolved body (not required for extern functions)
    let body = if extern_behavior.is_some() {
        // Extern functions have no body
        None
    } else {
        match func_symbol
            .metadata()
            .get_behavior::<ResolvedExecutableBehavior>()
        {
            Some(behavior) => Some(behavior.body().clone()),
            None => {
                ctx.emit_error(LoweringError::missing_body(
                    func_symbol.metadata().name().value.clone(),
                    func_symbol.metadata().span().clone(),
                ));
                return;
            },
        }
    };

    // Get callable behavior for parameter info
    let callable = func_symbol.metadata().get_behavior::<CallableBehavior>();

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(func_symbol.clone() as _));

    // Create the function with a placeholder return type.
    // We'll set the real return type after registering type parameters.
    let placeholder_ret = ctx.mir.ty_unit();
    let func_id = ctx.mir.add_function(name, placeholder_ret).id();

    // IMPORTANT: Register type parameters BEFORE lowering any types.
    // This ensures that type parameters like T, A, B are in scope when
    // we lower return types and parameter types.

    // First, register parent type parameters (for methods inside generic structs/enums)
    let parent_type_params = get_parent_type_parameters(func_symbol);
    for tp in &parent_type_params {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Then register the function's own type parameters
    for tp in func_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // NOW we can lower types with type parameters in scope

    // Lower return type
    let return_ty = func_symbol.return_type();
    let mir_ret_ty = lower_type(ctx, &return_ty);
    ctx.mir.function_mut(func_id).ret = mir_ret_ty;

    // Prepare and add self parameter if this is a method
    if let Some(ref callable) = callable
        && let Some(receiver) = callable.receiver()
        && let Some(self_ty) =
            compute_self_param_type(ctx, receiver, func_symbol, &parent_type_params)
    {
        ctx.mir.function_builder(func_id).param("self", self_ty);
    }

    // Add other parameters
    // Parameters are wrapped in reference types based on their access mode:
    // - Borrow: parameter has type &T (caller passes Rvalue::Ref)
    // - Mutating: parameter has type &var T (caller passes Rvalue::RefMut)
    // - Consuming: parameter has type T (caller passes value)
    // When accessing a reference-typed parameter, the LocalRef handler in expr.rs
    // automatically dereferences it.
    if let Some(ref callable) = callable {
        for param in callable.parameters() {
            let param_name = param.internal_name().to_string();
            let base_mir_ty = lower_type(ctx, &param.ty);
            let mir_ty = match param.access_mode() {
                ParameterAccessMode::Borrow => ctx.mir.ty_ref(base_mir_ty),
                ParameterAccessMode::Mutating => ctx.mir.ty_ref_mut(base_mir_ty),
                ParameterAccessMode::Consuming => base_mir_ty,
            };
            ctx.mir.function_builder(func_id).param(param_name, mir_ty);
        }
    }

    // For extern functions, set the extern_info and skip body lowering
    if let Some(extern_behavior) = extern_behavior {
        let func_name = func_symbol.metadata().name().value.clone();
        let symbol_name = extern_behavior.symbol_name(&func_name).to_string();
        let calling_convention = match extern_behavior.calling_convention() {
            CallingConvention::C => MirCallingConvention::C,
        };
        ctx.mir.function_mut(func_id).extern_info = Some(kestrel_execution_graph::ExternInfo {
            calling_convention,
            symbol_name,
        });
        // Clear type param mappings and return - no body to lower
        ctx.clear_type_params();
        return;
    }

    // Get the body (we know it's Some because we checked above)
    let body = body.unwrap();

    // Enter the function context
    ctx.enter_function(func_id);

    // Collect LocalIds that belong to closures (their parameters and implicit `it`).
    // These should NOT be created as locals in the parent function - they'll be
    // created when the closure function is lowered.
    let closure_local_ids = collect_closure_local_ids(&body);

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

    // Then create and map non-parameter locals, excluding closure parameters
    for local in locals.iter().skip(param_count) {
        // Skip locals that belong to closures
        if closure_local_ids.contains(&local.id()) {
            continue;
        }
        let mir_ty = lower_type(ctx, local.ty());
        let mir_local_id = ctx.create_local(local.name().to_string(), mir_ty);
        ctx.map_local(local.id(), mir_local_id);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Enter the function body scope for deinit tracking
    ctx.enter_scope();

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
                // Mark the return value's local as moved so it doesn't get deinited.
                // The caller takes ownership of the return value.
                if let Some(local) = crate::expr::try_get_local_from_value(&value) {
                    ctx.mark_moved(local);
                }
                // Emit deinits for all scopes before returning
                ctx.emit_all_scope_deinits();
                ctx.emit_return(value);
            }
        } else {
            // Emit deinits for all scopes before returning
            ctx.emit_all_scope_deinits();
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
    let body = match init_symbol
        .metadata()
        .get_behavior::<ResolvedExecutableBehavior>()
    {
        Some(behavior) => behavior.body().clone(),
        None => {
            ctx.emit_error(LoweringError::missing_body(
                "init",
                init_symbol.metadata().span().clone(),
            ));
            return;
        },
    };

    // Get callable behavior for parameter info
    let callable = init_symbol.metadata().get_behavior::<CallableBehavior>();

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(init_symbol.clone() as _));

    // Initializers always return unit
    let mir_ret_ty = ctx.mir.ty_unit();

    // Create the function
    let func_id = ctx.mir.add_function(name, mir_ret_ty).id();

    // IMPORTANT: Register type parameters BEFORE lowering any types.
    // Get parent type parameters (for initializers inside generic structs/enums)
    let parent_type_params = get_initializer_parent_type_parameters(init_symbol);
    for tp in &parent_type_params {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // NOW we can lower types with type parameters in scope

    // Prepare self parameter type
    let self_param_ty = if let Some(parent) = init_symbol.metadata().parent() {
        let parent_name = qualified_name_for_symbol(ctx, &parent);
        // Build type arguments from parent's type parameters
        let type_args: Vec<_> = parent_type_params
            .iter()
            .filter_map(|tp| {
                ctx.get_type_param(tp.metadata().id()).map(|mir_tp| {
                    ctx.mir
                        .intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp))
                })
            })
            .collect();
        let parent_ty = ctx.mir.ty_named(parent_name, type_args);
        Some(ctx.mir.ty_ref_mut(parent_ty))
    } else {
        None
    };

    // Add self parameter
    if let Some(self_ty) = self_param_ty {
        ctx.mir.function_builder(func_id).param("self", self_ty);
    }

    // Add other parameters
    // Parameters are wrapped in reference types based on their access mode
    if let Some(ref callable) = callable {
        for param in callable.parameters() {
            let param_name = param.internal_name().to_string();
            let base_mir_ty = lower_type(ctx, &param.ty);
            let mir_ty = match param.access_mode() {
                ParameterAccessMode::Borrow => ctx.mir.ty_ref(base_mir_ty),
                ParameterAccessMode::Mutating => ctx.mir.ty_ref_mut(base_mir_ty),
                ParameterAccessMode::Consuming => base_mir_ty,
            };
            ctx.mir.function_builder(func_id).param(param_name, mir_ty);
        }
    }

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

    // Lower yield expression (if any) for side effects, then return unit.
    if !ctx.is_block_terminated() {
        if let Some(yield_expr) = body.yield_expr.as_ref() {
            let _ = lower_expression(ctx, yield_expr);
        }
        if !ctx.is_block_terminated() {
            ctx.emit_return_unit();
        }
    }

    ctx.exit_function();
}

/// Lower a getter to MIR.
///
/// Getters are lowered as functions with signature:
/// `func Type.get:fieldName(self: &Type) -> FieldType`
/// or for static getters:
/// `func Type.get:fieldName() -> FieldType`
pub fn lower_getter(ctx: &mut LoweringContext, getter_symbol: &Arc<GetterSymbol>) {
    // Get the resolved body
    let body = match getter_symbol
        .metadata()
        .get_behavior::<ResolvedExecutableBehavior>()
    {
        Some(behavior) => behavior.body().clone(),
        None => {
            // No resolved body - skip (might be a computed property without a body yet)
            return;
        },
    };

    // Get callable behavior for parameter/return info
    let callable = getter_symbol.metadata().get_behavior::<CallableBehavior>();
    let Some(callable) = callable else {
        return;
    };

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(getter_symbol.clone() as _));

    // Lower return type
    let return_ty = callable.return_type();
    let placeholder_ret = ctx.mir.ty_unit(); // Will be updated after type params registered

    // Create the function
    let func_id = ctx.mir.add_function(name, placeholder_ret).id();

    // Register parent type parameters (getter is nested: Type -> Field -> Getter)
    let parent_type_params = get_getter_parent_type_parameters(getter_symbol);
    for tp in &parent_type_params {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Now lower return type with type params in scope
    let mir_ret_ty = lower_type(ctx, return_ty);
    ctx.mir.function_mut(func_id).ret = mir_ret_ty;

    // Add self parameter if this is an instance getter
    if let Some(receiver) = callable.receiver()
        && let Some(self_ty) =
            compute_getter_self_param_type(ctx, receiver, getter_symbol, &parent_type_params)
    {
        ctx.mir.function_builder(func_id).param("self", self_ty);
    }

    // Enter the function context
    ctx.enter_function(func_id);

    // Map the self local if it exists (parameter 0)
    let param_count = ctx.mir.function(func_id).params.len();
    let mir_locals = ctx.mir.function(func_id).locals.clone();

    // Map parameter locals (just self for getters)
    // The getter body references `self` via LocalId(0) from the temporary local scope
    // created during binding. We map it to the MIR self parameter.
    if param_count > 0 {
        // Map LocalId(0) (self) to the first MIR local (the self parameter)
        let mir_self_local = mir_locals[0];
        ctx.map_local(LocalId(0), mir_self_local);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Enter scope for deinit tracking
    ctx.enter_scope();

    // Lower statements
    for stmt in &body.statements {
        lower_statement(ctx, stmt);

        if ctx.is_block_terminated() {
            break;
        }
    }

    // Lower yield expression if present
    if !ctx.is_block_terminated() {
        if let Some(yield_expr) = body.yield_expr.as_ref() {
            let value = lower_expression(ctx, yield_expr);
            if !ctx.is_block_terminated() {
                // Mark the return value's local as moved so it doesn't get deinited.
                // The caller takes ownership of the return value.
                if let Some(local) = crate::expr::try_get_local_from_value(&value) {
                    ctx.mark_moved(local);
                }
                ctx.emit_all_scope_deinits();
                ctx.emit_return(value);
            }
        } else {
            // Getters should always have a yield expression
            ctx.emit_all_scope_deinits();
            ctx.emit_return_unit();
        }
    }

    ctx.exit_scope();
    ctx.exit_function();

    // Clear type param mappings
    ctx.clear_type_params();
}

/// Lower a setter to MIR.
///
/// Setters are lowered as functions with signature:
/// `func Type.set:fieldName(self: &var Type, newValue: FieldType) -> ()`
/// or for static setters:
/// `func Type.set:fieldName(newValue: FieldType) -> ()`
pub fn lower_setter(ctx: &mut LoweringContext, setter_symbol: &Arc<SetterSymbol>) {
    // Get the resolved body
    let body = match setter_symbol
        .metadata()
        .get_behavior::<ResolvedExecutableBehavior>()
    {
        Some(behavior) => behavior.body().clone(),
        None => {
            // No resolved body - skip
            return;
        },
    };

    // Get callable behavior for parameter info
    let callable = setter_symbol.metadata().get_behavior::<CallableBehavior>();
    let Some(callable) = callable else {
        return;
    };

    // Generate qualified name
    let name = qualified_name_for_symbol(ctx, &(setter_symbol.clone() as _));

    // Setters return unit
    let mir_ret_ty = ctx.mir.ty_unit();

    // Create the function
    let func_id = ctx.mir.add_function(name, mir_ret_ty).id();

    // Register parent type parameters (setter is nested: Type -> Field -> Setter)
    let parent_type_params = get_setter_parent_type_parameters(setter_symbol);
    for tp in &parent_type_params {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def =
            kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Add self parameter if this is an instance setter
    if let Some(receiver) = callable.receiver()
        && let Some(self_ty) =
            compute_setter_self_param_type(ctx, receiver, setter_symbol, &parent_type_params)
    {
        ctx.mir.function_builder(func_id).param("self", self_ty);
    }

    // Add newValue parameter
    for param in callable.parameters() {
        let param_name = param.internal_name().to_string();
        let base_mir_ty = lower_type(ctx, &param.ty);
        let mir_ty = match param.access_mode() {
            ParameterAccessMode::Borrow => ctx.mir.ty_ref(base_mir_ty),
            ParameterAccessMode::Mutating => ctx.mir.ty_ref_mut(base_mir_ty),
            ParameterAccessMode::Consuming => base_mir_ty,
        };
        ctx.mir.function_builder(func_id).param(param_name, mir_ty);
    }

    // Enter the function context
    ctx.enter_function(func_id);

    // Map parameter locals
    // Setters have:
    // - LocalId(0) = self (if instance setter)
    // - LocalId(N) = newValue parameter
    let param_count = ctx.mir.function(func_id).params.len();
    let mir_locals = ctx.mir.function(func_id).locals.clone();

    // Map all parameter locals
    for (i, mir_local) in mir_locals.iter().take(param_count).enumerate() {
        ctx.map_local(LocalId(i), *mir_local);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Enter scope for deinit tracking
    ctx.enter_scope();

    // Lower statements
    for stmt in &body.statements {
        lower_statement(ctx, stmt);

        if ctx.is_block_terminated() {
            break;
        }
    }

    // Setters return unit
    if !ctx.is_block_terminated() {
        ctx.emit_all_scope_deinits();
        ctx.emit_return_unit();
    }

    ctx.exit_scope();
    ctx.exit_function();

    // Clear type param mappings
    ctx.clear_type_params();
}

/// Get type parameters from the grandparent struct/enum/extension (for getters).
/// Hierarchy: Struct/Enum/Extension -> Field -> Getter
fn get_getter_parent_type_parameters(
    getter_symbol: &Arc<GetterSymbol>,
) -> Vec<Arc<TypeParameterSymbol>> {
    use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

    // Getter's parent is Field, Field's parent is the type
    let Some(field) = getter_symbol.metadata().parent() else {
        return vec![];
    };
    let Some(type_parent) = field.metadata().parent() else {
        return vec![];
    };

    // Try to downcast to get type parameters
    if let Ok(struct_symbol) = type_parent.clone().downcast_arc::<StructSymbol>() {
        return struct_symbol.type_parameters();
    }
    if let Ok(enum_symbol) = type_parent.clone().downcast_arc::<EnumSymbol>() {
        return enum_symbol.type_parameters();
    }
    if let Ok(extension_symbol) = type_parent.downcast_arc::<ExtensionSymbol>() {
        return extension_symbol.referenced_type_parameters();
    }

    vec![]
}

/// Get type parameters from the grandparent struct/enum/extension (for setters).
/// Hierarchy: Struct/Enum/Extension -> Field -> Setter
fn get_setter_parent_type_parameters(
    setter_symbol: &Arc<SetterSymbol>,
) -> Vec<Arc<TypeParameterSymbol>> {
    use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

    // Setter's parent is Field, Field's parent is the type
    let Some(field) = setter_symbol.metadata().parent() else {
        return vec![];
    };
    let Some(type_parent) = field.metadata().parent() else {
        return vec![];
    };

    // Try to downcast to get type parameters
    if let Ok(struct_symbol) = type_parent.clone().downcast_arc::<StructSymbol>() {
        return struct_symbol.type_parameters();
    }
    if let Ok(enum_symbol) = type_parent.clone().downcast_arc::<EnumSymbol>() {
        return enum_symbol.type_parameters();
    }
    if let Ok(extension_symbol) = type_parent.downcast_arc::<ExtensionSymbol>() {
        return extension_symbol.referenced_type_parameters();
    }

    vec![]
}

/// Compute the self parameter type for a getter.
fn compute_getter_self_param_type(
    ctx: &mut LoweringContext,
    receiver: ReceiverKind,
    getter_symbol: &Arc<GetterSymbol>,
    parent_type_params: &[Arc<TypeParameterSymbol>],
) -> Option<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> {
    // Getter's grandparent is the type (Getter -> Field -> Type)
    let field = getter_symbol.metadata().parent()?;
    let type_parent = field.metadata().parent()?;

    let parent_name = qualified_name_for_symbol(ctx, &type_parent);

    // Build type arguments from parent's type parameters
    let type_args: Vec<_> = parent_type_params
        .iter()
        .filter_map(|tp| {
            ctx.get_type_param(tp.metadata().id()).map(|mir_tp| {
                ctx.mir
                    .intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp))
            })
        })
        .collect();

    let parent_ty = ctx.mir.ty_named(parent_name, type_args);

    // Create the self parameter type based on receiver kind
    let self_ty = match receiver {
        ReceiverKind::Borrowing => ctx.mir.ty_ref(parent_ty),
        ReceiverKind::Mutating => ctx.mir.ty_ref_mut(parent_ty),
        ReceiverKind::Consuming => parent_ty,
        ReceiverKind::Initializing => ctx.mir.ty_ref_mut(parent_ty),
    };

    Some(self_ty)
}

/// Compute the self parameter type for a setter.
fn compute_setter_self_param_type(
    ctx: &mut LoweringContext,
    receiver: ReceiverKind,
    setter_symbol: &Arc<SetterSymbol>,
    parent_type_params: &[Arc<TypeParameterSymbol>],
) -> Option<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> {
    // Setter's grandparent is the type (Setter -> Field -> Type)
    let field = setter_symbol.metadata().parent()?;
    let type_parent = field.metadata().parent()?;

    let parent_name = qualified_name_for_symbol(ctx, &type_parent);

    // Build type arguments from parent's type parameters
    let type_args: Vec<_> = parent_type_params
        .iter()
        .filter_map(|tp| {
            ctx.get_type_param(tp.metadata().id()).map(|mir_tp| {
                ctx.mir
                    .intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp))
            })
        })
        .collect();

    let parent_ty = ctx.mir.ty_named(parent_name, type_args);

    // Create the self parameter type based on receiver kind
    let self_ty = match receiver {
        ReceiverKind::Borrowing => ctx.mir.ty_ref(parent_ty),
        ReceiverKind::Mutating => ctx.mir.ty_ref_mut(parent_ty),
        ReceiverKind::Consuming => parent_ty,
        ReceiverKind::Initializing => ctx.mir.ty_ref_mut(parent_ty),
    };

    Some(self_ty)
}

/// Compute the self parameter type for a method.
fn compute_self_param_type(
    ctx: &mut LoweringContext,
    receiver: ReceiverKind,
    func_symbol: &Arc<FunctionSymbol>,
    parent_type_params: &[Arc<TypeParameterSymbol>],
) -> Option<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> {
    // Get the parent type for self
    let parent = func_symbol.metadata().parent()?;

    let parent_name = qualified_name_for_symbol(ctx, &parent);

    // Build type arguments from parent's type parameters
    // These are now registered in ctx, so we can look them up
    let type_args: Vec<_> = parent_type_params
        .iter()
        .filter_map(|tp| {
            ctx.get_type_param(tp.metadata().id()).map(|mir_tp| {
                ctx.mir
                    .intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp))
            })
        })
        .collect();

    let parent_ty = ctx.mir.ty_named(parent_name, type_args);

    // Create the self parameter type based on receiver kind
    let self_ty = match receiver {
        ReceiverKind::Borrowing => ctx.mir.ty_ref(parent_ty),
        ReceiverKind::Mutating => ctx.mir.ty_ref_mut(parent_ty),
        ReceiverKind::Consuming => parent_ty,
        ReceiverKind::Initializing => ctx.mir.ty_ref_mut(parent_ty),
    };

    Some(self_ty)
}

/// Get type parameters from the parent struct, enum, or extension (for methods).
fn get_parent_type_parameters(func_symbol: &Arc<FunctionSymbol>) -> Vec<Arc<TypeParameterSymbol>> {
    use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

    if let Some(parent) = func_symbol.metadata().parent() {
        // Try to downcast to StructSymbol
        if let Ok(struct_symbol) = parent.clone().downcast_arc::<StructSymbol>() {
            return struct_symbol.type_parameters();
        }
        // Try to downcast to EnumSymbol
        if let Ok(enum_symbol) = parent.clone().downcast_arc::<EnumSymbol>() {
            return enum_symbol.type_parameters();
        }
        // Try to downcast to ExtensionSymbol
        if let Ok(extension_symbol) = parent.downcast_arc::<ExtensionSymbol>() {
            return extension_symbol.referenced_type_parameters();
        }
    }
    vec![]
}

/// Get type parameters from the parent struct, enum, or extension (for initializers).
fn get_initializer_parent_type_parameters(
    init_symbol: &Arc<InitializerSymbol>,
) -> Vec<Arc<TypeParameterSymbol>> {
    use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;

    if let Some(parent) = init_symbol.metadata().parent() {
        // Try to downcast to StructSymbol
        if let Ok(struct_symbol) = parent.clone().downcast_arc::<StructSymbol>() {
            return struct_symbol.type_parameters();
        }
        // Try to downcast to EnumSymbol
        if let Ok(enum_symbol) = parent.clone().downcast_arc::<EnumSymbol>() {
            return enum_symbol.type_parameters();
        }
        // Try to downcast to ExtensionSymbol
        if let Ok(extension_symbol) = parent.downcast_arc::<ExtensionSymbol>() {
            return extension_symbol.referenced_type_parameters();
        }
    }
    vec![]
}

/// Collect all LocalIds that belong to closures in a code block.
///
/// This includes:
/// - Explicit closure parameters (`{ x in ... }`)
/// - Implicit `it` parameters (`{ $0 + 1 }`)
///
/// These locals should NOT be created in the parent function - they will be
/// created when the closure function is lowered.
fn collect_closure_local_ids(body: &CodeBlock) -> HashSet<LocalId> {
    let mut ids = HashSet::new();

    for stmt in &body.statements {
        collect_closure_local_ids_from_stmt(stmt, &mut ids);
    }

    if let Some(expr) = &body.yield_expr {
        collect_closure_local_ids_from_expr(expr, &mut ids);
    }

    ids
}

fn collect_closure_local_ids_from_stmt(stmt: &Statement, ids: &mut HashSet<LocalId>) {
    match &stmt.kind {
        StatementKind::Binding { value, .. } => {
            if let Some(expr) = value {
                collect_closure_local_ids_from_expr(expr, ids);
            }
        },
        StatementKind::Expr(expr) => {
            collect_closure_local_ids_from_expr(expr, ids);
        },
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            for cond in conditions {
                collect_closure_local_ids_from_condition(cond, ids);
            }
            for stmt in &else_block.statements {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
            if let Some(expr) = &else_block.yield_expr {
                collect_closure_local_ids_from_expr(expr, ids);
            }
        },
        StatementKind::Deinit { .. } => {
            // Deinit statement doesn't contain closures
        },
    }
}

fn collect_closure_local_ids_from_condition(cond: &IfCondition, ids: &mut HashSet<LocalId>) {
    match cond {
        IfCondition::Expr(expr) => collect_closure_local_ids_from_expr(expr, ids),
        IfCondition::Let { value, .. } => collect_closure_local_ids_from_expr(value, ids),
    }
}

fn collect_closure_local_ids_from_expr(expr: &Expression, ids: &mut HashSet<LocalId>) {
    match &expr.kind {
        ExprKind::Closure {
            params,
            body,
            tail_expr,
            implicit_param,
            ..
        } => {
            // Collect explicit parameter LocalIds
            if let Some(param_list) = params {
                for param in param_list {
                    ids.insert(param.local_id);
                }
            }
            // Collect implicit `it` parameter LocalId
            if let Some((local_id, _, _)) = implicit_param {
                ids.insert(*local_id);
            }
            // Recurse into closure body (for nested closures)
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
            if let Some(tail) = tail_expr {
                collect_closure_local_ids_from_expr(tail, ids);
            }
        },

        // Recurse into all expression kinds that can contain sub-expressions
        ExprKind::Call {
            callee, arguments, ..
        } => {
            collect_closure_local_ids_from_expr(callee, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_closure_local_ids_from_expr(receiver, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },
        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            collect_closure_local_ids_from_expr(receiver, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },
        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },
        ExprKind::Assignment { target, value } => {
            collect_closure_local_ids_from_expr(target, ids);
            collect_closure_local_ids_from_expr(value, ids);
        },
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            for cond in conditions {
                collect_closure_local_ids_from_condition(cond, ids);
            }
            for stmt in then_branch {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
            if let Some(val) = then_value {
                collect_closure_local_ids_from_expr(val, ids);
            }
            if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::ElseIf(else_if) => {
                        collect_closure_local_ids_from_expr(else_if, ids);
                    },
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            collect_closure_local_ids_from_stmt(stmt, ids);
                        }
                        if let Some(val) = value {
                            collect_closure_local_ids_from_expr(val, ids);
                        }
                    },
                }
            }
        },
        ExprKind::Match { scrutinee, arms } => {
            collect_closure_local_ids_from_expr(scrutinee, ids);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_closure_local_ids_from_expr(guard, ids);
                }
                collect_closure_local_ids_from_expr(&arm.body, ids);
            }
        },
        ExprKind::Block { statements, value } => {
            for stmt in statements {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
            if let Some(val) = value {
                collect_closure_local_ids_from_expr(val, ids);
            }
        },
        ExprKind::While {
            condition, body, ..
        } => {
            collect_closure_local_ids_from_expr(condition, ids);
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        },
        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for cond in conditions {
                collect_closure_local_ids_from_condition(cond, ids);
            }
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        },
        ExprKind::Loop { body, .. } => {
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        },
        ExprKind::Tuple(exprs) | ExprKind::Array(exprs) => {
            for e in exprs {
                collect_closure_local_ids_from_expr(e, ids);
            }
        },
        ExprKind::Grouping(inner) => {
            collect_closure_local_ids_from_expr(inner, ids);
        },
        ExprKind::FieldAccess { object, .. } => {
            collect_closure_local_ids_from_expr(object, ids);
        },
        ExprKind::TupleIndex { tuple, .. } => {
            collect_closure_local_ids_from_expr(tuple, ids);
        },
        ExprKind::MethodRef { receiver, .. } => {
            collect_closure_local_ids_from_expr(receiver, ids);
        },
        ExprKind::PrimitiveMethodRef { receiver, .. } => {
            collect_closure_local_ids_from_expr(receiver, ids);
        },
        ExprKind::Return { value } => {
            if let Some(e) = value {
                collect_closure_local_ids_from_expr(e, ids);
            }
        },
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    collect_closure_local_ids_from_expr(&arg.value, ids);
                }
            }
        },

        // Lang intrinsics - recurse into arguments
        ExprKind::LangIntrinsic { arguments, .. } => {
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },

        // Subscript call - recurse into receiver and arguments
        ExprKind::SubscriptCall {
            receiver,
            arguments,
            ..
        } => {
            collect_closure_local_ids_from_expr(receiver, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        },

        // Leaf expressions that don't contain sub-expressions
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::LangIntrinsicRef(_)
        | ExprKind::Error => {},
    }
}
