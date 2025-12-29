//! Function and initializer lowering.

use std::collections::HashSet;
use std::sync::Arc;

use kestrel_execution_graph::TypeParamOwner;
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ReceiverKind};
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ResolvedExecutableBehavior};
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::local::LocalId;
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
        let tp_def = kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
        let tp_id = ctx.mir.type_params.alloc(tp_def);
        ctx.mir.function_mut(func_id).type_params.push(tp_id);
        ctx.map_type_param(tp.metadata().id(), tp_id);
    }

    // Then register the function's own type parameters
    for tp in func_symbol.type_parameters() {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def = kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
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
    if let Some(ref callable) = callable {
        if let Some(receiver) = callable.receiver() {
            if let Some(self_ty) = compute_self_param_type(ctx, receiver, func_symbol, &parent_type_params) {
                ctx.mir.function_builder(func_id).param("self", self_ty);
            }
        }
    }

    // Add other parameters
    if let Some(ref callable) = callable {
        for param in callable.parameters() {
            let param_name = param.internal_name().to_string();
            let mir_ty = lower_type(ctx, &param.ty);
            ctx.mir.function_builder(func_id).param(param_name, mir_ty);
        }
    }

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

    // Create the function
    let func_id = ctx.mir.add_function(name, mir_ret_ty).id();

    // IMPORTANT: Register type parameters BEFORE lowering any types.
    // Get parent type parameters (for initializers inside generic structs/enums)
    let parent_type_params = get_initializer_parent_type_parameters(init_symbol);
    for tp in &parent_type_params {
        let tp_name = tp.metadata().name().value.clone();
        let tp_def = kestrel_execution_graph::TypeParamDef::new(tp_name, TypeParamOwner::Function(func_id));
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
                ctx.get_type_param(tp.metadata().id())
                    .map(|mir_tp| ctx.mir.intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp)))
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
    if let Some(ref callable) = callable {
        for param in callable.parameters() {
            let param_name = param.internal_name().to_string();
            let mir_ty = lower_type(ctx, &param.ty);
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
            ctx.get_type_param(tp.metadata().id())
                .map(|mir_tp| ctx.mir.intern_type(kestrel_execution_graph::MirTy::TypeParam(mir_tp)))
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

/// Get type parameters from the parent struct or enum (for methods).
fn get_parent_type_parameters(func_symbol: &Arc<FunctionSymbol>) -> Vec<Arc<TypeParameterSymbol>> {
    if let Some(parent) = func_symbol.metadata().parent() {
        // Try to downcast to StructSymbol
        if let Ok(struct_symbol) = parent.clone().downcast_arc::<StructSymbol>() {
            return struct_symbol.type_parameters();
        }
        // Try to downcast to EnumSymbol
        if let Ok(enum_symbol) = parent.downcast_arc::<EnumSymbol>() {
            return enum_symbol.type_parameters();
        }
    }
    vec![]
}

/// Get type parameters from the parent struct or enum (for initializers).
fn get_initializer_parent_type_parameters(init_symbol: &Arc<InitializerSymbol>) -> Vec<Arc<TypeParameterSymbol>> {
    if let Some(parent) = init_symbol.metadata().parent() {
        // Try to downcast to StructSymbol
        if let Ok(struct_symbol) = parent.clone().downcast_arc::<StructSymbol>() {
            return struct_symbol.type_parameters();
        }
        // Try to downcast to EnumSymbol
        if let Ok(enum_symbol) = parent.downcast_arc::<EnumSymbol>() {
            return enum_symbol.type_parameters();
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
        }
        StatementKind::Expr(expr) => {
            collect_closure_local_ids_from_expr(expr, ids);
        }
        StatementKind::GuardLet { conditions, else_block } => {
            for cond in conditions {
                collect_closure_local_ids_from_condition(cond, ids);
            }
            for stmt in &else_block.statements {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
            if let Some(expr) = &else_block.yield_expr {
                collect_closure_local_ids_from_expr(expr, ids);
            }
        }
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
        ExprKind::Closure { params, body, tail_expr, implicit_param, .. } => {
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
        }
        
        // Recurse into all expression kinds that can contain sub-expressions
        ExprKind::Call { callee, arguments, .. } => {
            collect_closure_local_ids_from_expr(callee, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        }
        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            collect_closure_local_ids_from_expr(receiver, ids);
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                collect_closure_local_ids_from_expr(&arg.value, ids);
            }
        }
        ExprKind::Assignment { target, value } => {
            collect_closure_local_ids_from_expr(target, ids);
            collect_closure_local_ids_from_expr(value, ids);
        }
        ExprKind::If { conditions, then_branch, then_value, else_branch } => {
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
                    }
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            collect_closure_local_ids_from_stmt(stmt, ids);
                        }
                        if let Some(val) = value {
                            collect_closure_local_ids_from_expr(val, ids);
                        }
                    }
                }
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_closure_local_ids_from_expr(scrutinee, ids);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    collect_closure_local_ids_from_expr(guard, ids);
                }
                collect_closure_local_ids_from_expr(&arm.body, ids);
            }
        }
        ExprKind::While { condition, body, .. } => {
            collect_closure_local_ids_from_expr(condition, ids);
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        }
        ExprKind::WhileLet { conditions, body, .. } => {
            for cond in conditions {
                collect_closure_local_ids_from_condition(cond, ids);
            }
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        }
        ExprKind::Loop { body, .. } => {
            for stmt in body {
                collect_closure_local_ids_from_stmt(stmt, ids);
            }
        }
        ExprKind::Tuple(exprs) | ExprKind::Array(exprs) => {
            for e in exprs {
                collect_closure_local_ids_from_expr(e, ids);
            }
        }
        ExprKind::Grouping(inner) => {
            collect_closure_local_ids_from_expr(inner, ids);
        }
        ExprKind::FieldAccess { object, .. } => {
            collect_closure_local_ids_from_expr(object, ids);
        }
        ExprKind::TupleIndex { tuple, .. } => {
            collect_closure_local_ids_from_expr(tuple, ids);
        }
        ExprKind::MethodRef { receiver, .. } => {
            collect_closure_local_ids_from_expr(receiver, ids);
        }
        ExprKind::Return { value } => {
            if let Some(e) = value {
                collect_closure_local_ids_from_expr(e, ids);
            }
        }
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    collect_closure_local_ids_from_expr(&arg.value, ids);
                }
            }
        }
        
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
        | ExprKind::Error => {}
    }
}
