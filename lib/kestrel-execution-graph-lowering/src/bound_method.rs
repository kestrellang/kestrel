//! Bound method lowering - transforms method references to thick callables.
//!
//! When a method is referenced without being called (e.g., `obj.method`),
//! we create a thick callable (closure) that captures the receiver and
//! forwards calls to the method.
//!
//! ## Strategy
//!
//! Given `receiver.method`:
//! 1. Evaluate and capture the receiver value
//! 2. Generate an environment struct containing the captured receiver
//! 3. Generate a call function that extracts the receiver and calls the method
//! 4. Use `ApplyPartial` to create the thick callable
//!
//! ## Naming Convention
//!
//! - Call function: `module::containing_func.bound.N`
//! - Environment struct: `module::containing_func.bound.N.env`

use kestrel_execution_graph::{
    Callee, Id, MirTy, Place, QualifiedName, QualifiedNameData, Rvalue, Struct, Ty, Value,
};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::expr::Expression;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty as SemanticTy;
use kestrel_span::Span;
use semantic_tree::symbol::SymbolId;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::expr::lower_expression;
use crate::name::qualified_name_for_symbol;
use crate::ty::lower_type;

/// Lower a method reference (bound method) to a thick callable.
///
/// Returns a Value representing the thick callable that, when called,
/// invokes the method on the captured receiver.
pub fn lower_bound_method(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    candidates: &[SymbolId],
    method_name: &str,
    expr_ty: &SemanticTy,
    span: &Span,
) -> Value {
    // Get the first candidate (overload resolution should have selected one)
    let method_id = match candidates.first() {
        Some(id) => *id,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("no method candidates for bound method '{}'", method_name),
                Some(span.clone()),
            ));
            return Value::Immediate(kestrel_execution_graph::Immediate::error());
        },
    };

    // Look up the method symbol to get its signature
    let method_symbol = match ctx.model.query(SymbolFor { id: method_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("method symbol not found for '{}'", method_name),
                Some(span.clone()),
            ));
            return Value::Immediate(kestrel_execution_graph::Immediate::error());
        },
    };

    // Get the callable behavior to extract parameter info
    let callable = match method_symbol.metadata().get_behavior::<CallableBehavior>() {
        Some(c) => c,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("method '{}' has no callable behavior", method_name),
                Some(span.clone()),
            ));
            return Value::Immediate(kestrel_execution_graph::Immediate::error());
        },
    };

    // For static methods, we don't need to capture anything - just return a function reference
    if !callable.is_instance_method() {
        // Static method reference - return a thin function reference
        let func_name = qualified_name_for_symbol(ctx, &method_symbol);
        return create_static_method_reference(ctx, func_name, &callable, expr_ty, span);
    }

    // Lower the receiver expression to get the value to capture
    let receiver_value = lower_expression(ctx, receiver);

    // Generate unique bound method index and names
    let bound_idx = ctx.next_closure_index(); // Reuse closure counter
    let (bound_name, env_struct_name) = generate_bound_method_names(ctx, bound_idx);

    // Create the environment struct with the captured receiver
    let receiver_ty = lower_type(ctx, &receiver.ty);
    let env_struct_id = generate_env_struct(ctx, env_struct_name, receiver_ty, span);

    // Get method's qualified name for the call inside the generated function
    let method_qualified_name = qualified_name_for_symbol(ctx, &method_symbol);

    // Check if this is a protocol method (needs witness call)
    let is_protocol_method = method_symbol
        .metadata()
        .parent()
        .map(|p| p.metadata().kind() == KestrelSymbolKind::Protocol)
        .unwrap_or(false);

    // Create the bound method function
    create_bound_method_function(
        ctx,
        bound_name,
        env_struct_id,
        env_struct_name,
        &callable,
        method_qualified_name,
        method_name,
        &receiver.ty,
        is_protocol_method,
        span,
    );

    // Create the thick callable using ApplyPartial
    let thick_ty = create_thick_func_type(ctx, &callable);
    let result_local = ctx.create_temp("bound_method", thick_ty);
    let result_place = Place::local(result_local);

    ctx.emit_assign(
        result_place.clone(),
        Rvalue::ApplyPartial {
            func: bound_name,
            captures: vec![receiver_value],
        },
    );

    Value::Place(result_place)
}

/// Create a reference to a static method (no receiver capture needed).
fn create_static_method_reference(
    ctx: &mut LoweringContext,
    func_name: Id<QualifiedName>,
    callable: &CallableBehavior,
    _expr_ty: &SemanticTy,
    _span: &Span,
) -> Value {
    // For static methods, we can create a thick callable with no captures
    let thick_ty = create_thick_func_type(ctx, callable);
    let result_local = ctx.create_temp("static_method_ref", thick_ty);
    let result_place = Place::local(result_local);

    ctx.emit_assign(
        result_place.clone(),
        Rvalue::ApplyPartial {
            func: func_name,
            captures: vec![],
        },
    );

    Value::Place(result_place)
}

/// Generate qualified names for the bound method function and env struct.
fn generate_bound_method_names(
    ctx: &mut LoweringContext,
    idx: u32,
) -> (Id<QualifiedName>, Id<QualifiedName>) {
    let current_func = ctx.current_function_unwrap();
    let func_name = ctx.mir.function(current_func).name;
    let func_name_data = ctx.mir.name(func_name);

    // Build bound method name: func_name.bound.idx
    let mut bound_segments = func_name_data.segments.clone();
    if let Some(last) = bound_segments.last_mut() {
        *last = format!("{}.bound.{}", last, idx);
    }
    let bound_name = ctx.mir.intern_name(QualifiedNameData {
        segments: bound_segments.clone(),
    });

    // Build env struct name: bound_name.env
    let mut env_segments = bound_segments;
    if let Some(last) = env_segments.last_mut() {
        *last = format!("{}.env", last);
    }
    let env_name = ctx.mir.intern_name(QualifiedNameData {
        segments: env_segments,
    });

    (bound_name, env_name)
}

/// Generate the environment struct for the captured receiver.
fn generate_env_struct(
    ctx: &mut LoweringContext,
    name: Id<QualifiedName>,
    receiver_ty: Id<Ty>,
    _span: &Span,
) -> Id<Struct> {
    let struct_id = ctx.mir.add_struct(name);

    // Add field for the captured receiver
    ctx.mir.add_field(struct_id, "receiver", receiver_ty);

    struct_id
}

/// Create the thick function type for the bound method.
fn create_thick_func_type(ctx: &mut LoweringContext, callable: &CallableBehavior) -> Id<Ty> {
    // Parameters are the method's parameters (NOT including self/receiver)
    let param_tys: Vec<Id<Ty>> = callable
        .parameters()
        .iter()
        .map(|p| lower_type(ctx, &p.ty))
        .collect();

    let ret_ty = lower_type(ctx, callable.return_type());

    ctx.mir.intern_type(MirTy::FuncThick {
        params: param_tys,
        ret: ret_ty,
    })
}

/// Create the bound method's call function.
///
/// The function takes `env: &EnvStruct` as its first parameter, followed by
/// the method's parameters. It extracts the receiver from the env and calls
/// the original method.
fn create_bound_method_function(
    ctx: &mut LoweringContext,
    name: Id<QualifiedName>,
    env_struct_id: Id<Struct>,
    env_struct_name: Id<QualifiedName>,
    callable: &CallableBehavior,
    method_name: Id<QualifiedName>,
    _method_name_str: &str,
    _receiver_ty: &SemanticTy,
    is_protocol_method: bool,
    _span: &Span,
) {
    let mir_ret_ty = lower_type(ctx, callable.return_type());

    // Save current context
    let saved_func = ctx.current_function();
    let saved_local_map = ctx.save_local_map();
    let saved_block = ctx.current_block();
    let saved_closure_counter = ctx.get_closure_counter();
    let saved_temp_counter = ctx.get_temp_counter();

    // Pre-compute types
    let env_struct_ty = ctx.mir.ty_named(env_struct_name, vec![]);
    let env_param_ty = ctx.mir.ty_ref(env_struct_ty);

    let param_types: Vec<_> = callable
        .parameters()
        .iter()
        .map(|p| (p.internal_name().to_string(), lower_type(ctx, &p.ty)))
        .collect();

    // Create the function
    let func_id = {
        let mut func = ctx.mir.add_function(name, mir_ret_ty);

        // Add env parameter
        func.param("env", env_param_ty);

        // Add regular parameters
        for (param_name, param_ty) in &param_types {
            func.param(param_name, *param_ty);
        }

        func.id()
    };

    // Enter the new function context
    ctx.enter_function(func_id);

    // Get the locals that were created for the function parameters
    let mir_locals: Vec<_> = ctx.mir.function(func_id).locals.clone();

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Extract receiver from env struct: receiver = (deref env).receiver
    let env_local_id = mir_locals[0];
    let receiver_field_id = ctx.mir.structs[env_struct_id].fields[0];
    let receiver_mir_ty = ctx.mir.fields[receiver_field_id].ty;
    let receiver_local = ctx.create_local("receiver", receiver_mir_ty);

    let env_place = Place::local(env_local_id);
    let deref_env = Place::deref(env_place);
    let receiver_field = Place::field(deref_env, "receiver".to_string());

    ctx.emit_assign(Place::local(receiver_local), Rvalue::Copy(receiver_field));

    // Build arguments for the method call: receiver first, then the passed parameters
    let mut all_args = vec![Value::Place(Place::local(receiver_local))];
    for (i, _) in param_types.iter().enumerate() {
        let param_local = mir_locals[1 + i]; // Skip env parameter
        all_args.push(Value::Place(Place::local(param_local)));
    }

    // Create result place for the method call
    let result_local = ctx.create_temp("result", mir_ret_ty);
    let result_place = Place::local(result_local);

    // Emit the method call
    if is_protocol_method {
        // Protocol method - need witness call
        // Get protocol name from method's parent
        // For now, we'll use a direct call since we've already resolved the method
        // TODO: Handle protocol methods properly with witness lookup
        let callee = Callee::direct(method_name);
        ctx.emit_call(result_place.clone(), callee, all_args);
    } else {
        // Regular direct method call
        let callee = Callee::direct(method_name);
        ctx.emit_call(result_place.clone(), callee, all_args);
    }

    // Return the result
    ctx.emit_return(Value::Place(result_place));

    // Restore original context
    ctx.exit_function();
    ctx.set_current_function(saved_func);
    ctx.restore_local_map(saved_local_map);
    ctx.set_closure_counter(saved_closure_counter);
    ctx.set_temp_counter(saved_temp_counter);
    if let Some(block) = saved_block {
        ctx.set_current_block(block);
    }
}
