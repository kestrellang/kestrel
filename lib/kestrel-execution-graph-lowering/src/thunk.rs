//! Thunk generation for function references used as thick callables.
//!
//! When a regular function (which has no env parameter) is used as a function
//! value (which uses thick calling convention with env as first param), we need
//! to generate a wrapper "thunk" function that bridges the calling convention gap.
//!
//! The thunk:
//! 1. Accepts `(env: *i8, ...args...)` (thick calling convention)
//! 2. Ignores the `env` parameter
//! 3. Calls the original function with just `...args...`
//! 4. Returns the result
//!
//! ## Example
//!
//! For a function `func double(x: Int64) -> Int64`:
//!
//! ```text
//! func double.thunk(env: *i8, x: Int64) -> Int64 {
//!     return double(x)
//! }
//! ```

use std::collections::HashMap;

use kestrel_execution_graph::{
    CallArg, Callee, Id, MirTy, Origin, PassingMode, Place, QualifiedName, QualifiedNameData,
    Rvalue, Ty, Value,
};

use crate::context::LoweringContext;

/// Cache key for thunks: (original function name, type args)
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ThunkKey {
    pub func_name: Id<QualifiedName>,
    pub type_args: Vec<Id<Ty>>,
}

/// Thunk cache stored in LoweringContext
pub type ThunkCache = HashMap<ThunkKey, Id<QualifiedName>>;

/// Generate a thunk for a function reference.
///
/// The thunk wraps the original function with thick calling convention,
/// accepting an env pointer as the first parameter (which is ignored).
///
/// Returns the qualified name of the generated thunk function.
pub fn generate_function_thunk(
    ctx: &mut LoweringContext,
    original_func_name: Id<QualifiedName>,
    param_types: &[Id<Ty>],
    return_type: Id<Ty>,
    type_args: &[Id<Ty>],
) -> Id<QualifiedName> {
    // Build the thunk name: original_name.thunk
    let original_name_data = ctx.mir.name(original_func_name);
    let mut thunk_segments = original_name_data.segments.clone();
    if let Some(last) = thunk_segments.last_mut() {
        *last = format!("{}.thunk", last);
    }
    let thunk_name = ctx.mir.intern_name(QualifiedNameData {
        segments: thunk_segments,
    });

    // Create the thunk function
    let func_id = ctx.mir.add_function(thunk_name, return_type).id();

    // Add env parameter (unused, but required for thick calling convention)
    // Use *i8 as a generic pointer type
    let i8_ty = ctx.mir.intern_type(MirTy::I8);
    let env_ty = ctx.mir.ty_ptr(i8_ty);
    ctx.mir.function_builder(func_id).param("env", env_ty);

    // Add the original function's parameters
    for (i, &param_ty) in param_types.iter().enumerate() {
        ctx.mir
            .function_builder(func_id)
            .param(format!("arg{}", i), param_ty);
    }

    // Set origin metadata
    ctx.mir.function_mut(func_id).meta.origin = Some(Origin::FunctionThunk {
        original_function: original_func_name,
    });

    // Now generate the function body
    // Save current function context
    let saved_func = ctx.current_function();
    let saved_block = ctx.current_block();

    // Enter the thunk function context
    ctx.enter_function(func_id);

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // Get the parameter locals (skip env at index 0)
    let mir_locals: Vec<_> = ctx.mir.function(func_id).locals.clone();

    // Build argument list for calling the original function
    // Skip the env parameter (local 0), pass the rest
    let mut call_args = Vec::with_capacity(param_types.len());
    for (i, _) in param_types.iter().enumerate() {
        let local_id = mir_locals[i + 1]; // +1 to skip env
        let arg_value = Value::Place(Place::local(local_id));
        // Use Copy mode for all args - the original function's calling convention
        // will handle borrowing/moving as needed
        call_args.push(CallArg::new(arg_value, PassingMode::Copy));
    }

    // Create the callee for the original function
    let callee = if type_args.is_empty() {
        Callee::direct(original_func_name)
    } else {
        Callee::direct_generic(original_func_name, type_args.to_vec())
    };

    // Check if return type is unit
    let is_unit_return = matches!(ctx.mir.ty(return_type), MirTy::Unit);

    if is_unit_return {
        // Call and return unit
        ctx.emit_call_unit_with_modes(callee, call_args);
        ctx.emit_return_unit();
    } else {
        // Create temp for result, call, and return
        let result_local = ctx.create_temp("result", return_type);
        let result_place = Place::local(result_local);
        ctx.emit_assign(result_place.clone(), Rvalue::Call { callee, args: call_args });
        ctx.emit_return(Value::Place(result_place));
    }

    // Restore context
    ctx.exit_function();
    if let Some(func) = saved_func {
        ctx.set_current_function(Some(func));
    }
    if let Some(block) = saved_block {
        ctx.set_current_block(block);
    }

    thunk_name
}

/// Generate a thunk for a witness method reference.
///
/// The thunk wraps a witness method call with thick calling convention.
/// This is used when a protocol method is used as a function value.
pub fn generate_witness_thunk(
    ctx: &mut LoweringContext,
    protocol_name: Id<QualifiedName>,
    method_name: &str,
    for_type: Id<Ty>,
    param_types: &[Id<Ty>],
    return_type: Id<Ty>,
) -> Id<QualifiedName> {
    // Build the thunk name: Protocol.method.for_Type.thunk
    let protocol_name_data = ctx.mir.name(protocol_name);
    let type_str = format!("{:?}", for_type); // Simple representation
    let mut thunk_segments = protocol_name_data.segments.clone();
    thunk_segments.push(format!("{}.for_{}.thunk", method_name, type_str));
    let thunk_name = ctx.mir.intern_name(QualifiedNameData {
        segments: thunk_segments,
    });

    // Create the thunk function
    let func_id = ctx.mir.add_function(thunk_name, return_type).id();

    // Add env parameter (unused)
    let i8_ty = ctx.mir.intern_type(MirTy::I8);
    let env_ty = ctx.mir.ty_ptr(i8_ty);
    ctx.mir.function_builder(func_id).param("env", env_ty);

    // Add the method's parameters
    for (i, &param_ty) in param_types.iter().enumerate() {
        ctx.mir
            .function_builder(func_id)
            .param(format!("arg{}", i), param_ty);
    }

    // Set origin metadata
    ctx.mir.function_mut(func_id).meta.origin = Some(Origin::FunctionThunk {
        original_function: protocol_name, // Use protocol name as reference
    });

    // Generate the function body
    let saved_func = ctx.current_function();
    let saved_block = ctx.current_block();

    ctx.enter_function(func_id);

    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    let mir_locals: Vec<_> = ctx.mir.function(func_id).locals.clone();

    // Build argument list (skip env)
    let mut call_args = Vec::with_capacity(param_types.len());
    for (i, _) in param_types.iter().enumerate() {
        let local_id = mir_locals[i + 1];
        let arg_value = Value::Place(Place::local(local_id));
        call_args.push(CallArg::new(arg_value, PassingMode::Copy));
    }

    // Create witness method callee
    let callee = Callee::witness(protocol_name, method_name, for_type);

    let is_unit_return = matches!(ctx.mir.ty(return_type), MirTy::Unit);

    if is_unit_return {
        ctx.emit_call_unit_with_modes(callee, call_args);
        ctx.emit_return_unit();
    } else {
        let result_local = ctx.create_temp("result", return_type);
        let result_place = Place::local(result_local);
        ctx.emit_assign(result_place.clone(), Rvalue::Call { callee, args: call_args });
        ctx.emit_return(Value::Place(result_place));
    }

    ctx.exit_function();
    if let Some(func) = saved_func {
        ctx.set_current_function(Some(func));
    }
    if let Some(block) = saved_block {
        ctx.set_current_block(block);
    }

    thunk_name
}
