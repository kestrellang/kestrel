//! Closure lowering - transforms closure expressions to MIR.
//!
//! ## Strategy
//!
//! All closures are lowered to thick function values (`FuncThick`):
//!
//! - **Non-capturing closures**: Generate a synthetic function, use `ApplyPartial`
//!   with empty captures to create the thick callable.
//!
//! - **Capturing closures**: Generate an environment struct with captured variables,
//!   generate a call function that takes `&env` as first parameter, use `ApplyPartial`
//!   with captured values to create the thick callable.
//!
//! ## Naming Convention
//!
//! - Call function: `module::containing_func.closure.N`
//! - Environment struct: `module::containing_func.closure.N.env`

use kestrel_execution_graph::{
    Id, MirTy, Origin, Place, QualifiedName, QualifiedNameData, Rvalue, Struct, Ty, Value,
};
use kestrel_semantic_tree::expr::{Capture, ClosureParam, Expression};
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::symbol::local::LocalId;
use kestrel_semantic_tree::ty::{Ty as SemanticTy, TyKind};
use kestrel_span::Span;

use crate::context::LoweringContext;
use crate::expr::lower_expression;
use crate::stmt::lower_statement;
use crate::ty::lower_type;

/// An effective parameter for a closure (either explicit or implicit `it`).
struct EffectiveParam {
    name: String,
    ty: SemanticTy,
    local_id: LocalId,
}

/// Lower a closure expression to MIR.
///
/// Returns a Value representing the thick callable.
pub fn lower_closure(
    ctx: &mut LoweringContext,
    params: &Option<Vec<ClosureParam>>,
    body: &[Statement],
    tail_expr: &Option<Box<Expression>>,
    captures: &[Capture],
    implicit_param: &Option<(LocalId, SemanticTy, Span)>,
    closure_type: &SemanticTy,
    span: &Span,
) -> Value {
    // Extract return type from the function type
    let return_ty = extract_return_type(closure_type);

    // Generate unique closure index and names
    let closure_idx = ctx.next_closure_index();
    let (closure_name, env_struct_name) = generate_closure_names(ctx, closure_idx);

    // Build list of effective parameters
    let effective_params = build_effective_params(params, implicit_param);

    if captures.is_empty() {
        lower_non_capturing_closure(
            ctx,
            closure_name,
            &effective_params,
            body,
            tail_expr,
            &return_ty,
            span,
        )
    } else {
        lower_capturing_closure(
            ctx,
            closure_name,
            env_struct_name,
            &effective_params,
            body,
            tail_expr,
            captures,
            &return_ty,
            span,
        )
    }
}

/// Extract the return type from a function type.
fn extract_return_type(ty: &SemanticTy) -> SemanticTy {
    match ty.kind() {
        TyKind::Function { return_type, .. } => (**return_type).clone(),
        _ => {
            // Shouldn't happen - closure should have function type
            // Return unit as fallback
            SemanticTy::unit(ty.span().clone())
        }
    }
}

/// Generate the qualified names for closure function and env struct.
fn generate_closure_names(
    ctx: &mut LoweringContext,
    idx: u32,
) -> (Id<QualifiedName>, Id<QualifiedName>) {
    let current_func = ctx.current_function_unwrap();
    let func_name = ctx.mir.function(current_func).name;
    let func_name_data = ctx.mir.name(func_name);

    // Build closure name: func_name.closure.idx
    let mut closure_segments = func_name_data.segments.clone();
    // Modify last segment to add .closure.idx
    if let Some(last) = closure_segments.last_mut() {
        *last = format!("{}.closure.{}", last, idx);
    }
    let closure_name = ctx.mir.intern_name(QualifiedNameData {
        segments: closure_segments.clone(),
    });

    // Build env struct name: closure_name.env
    let mut env_segments = closure_segments;
    if let Some(last) = env_segments.last_mut() {
        *last = format!("{}.env", last);
    }
    let env_name = ctx.mir.intern_name(QualifiedNameData {
        segments: env_segments,
    });

    (closure_name, env_name)
}

/// Build the list of effective parameters from explicit params or implicit `it`.
fn build_effective_params(
    params: &Option<Vec<ClosureParam>>,
    implicit_param: &Option<(LocalId, SemanticTy, Span)>,
) -> Vec<EffectiveParam> {
    if let Some(explicit_params) = params {
        // Explicit parameters - use the LocalId from the ClosureParam
        explicit_params
            .iter()
            .map(|p| EffectiveParam {
                name: p.name.clone(),
                ty: p.ty.clone(),
                local_id: p.local_id,
            })
            .collect()
    } else if let Some((local_id, ty, _span)) = implicit_param {
        // Implicit `it` parameter
        vec![EffectiveParam {
            name: "it".to_string(),
            ty: ty.clone(),
            local_id: *local_id,
        }]
    } else {
        // No parameters
        vec![]
    }
}

/// Lower a non-capturing closure.
fn lower_non_capturing_closure(
    ctx: &mut LoweringContext,
    closure_name: Id<QualifiedName>,
    params: &[EffectiveParam],
    body: &[Statement],
    tail_expr: &Option<Box<Expression>>,
    return_ty: &SemanticTy,
    span: &Span,
) -> Value {
    // 1. Create the closure function
    create_closure_function(
        ctx,
        closure_name,
        None,
        &[],
        params,
        body,
        tail_expr,
        return_ty,
        span,
    );

    // 2. Create thick callable via ApplyPartial with empty captures
    let thick_ty = create_thick_func_type(ctx, params, return_ty);
    let result_local = ctx.create_temp("closure", thick_ty);
    let result_place = Place::local(result_local);

    ctx.emit_assign(
        result_place.clone(),
        Rvalue::ApplyPartial {
            func: closure_name,
            captures: vec![],
        },
    );

    Value::Place(result_place)
}

/// Lower a capturing closure.
fn lower_capturing_closure(
    ctx: &mut LoweringContext,
    closure_name: Id<QualifiedName>,
    env_struct_name: Id<QualifiedName>,
    params: &[EffectiveParam],
    body: &[Statement],
    tail_expr: &Option<Box<Expression>>,
    captures: &[Capture],
    return_ty: &SemanticTy,
    span: &Span,
) -> Value {
    // 1. Generate environment struct with captured variables
    let env_struct_id = generate_env_struct(ctx, env_struct_name, captures, span);

    // 2. Collect the captured values from current context BEFORE switching context
    let capture_values: Vec<Value> = captures
        .iter()
        .map(|cap| {
            let local_id = ctx.get_local_unwrap(cap.local_id);
            Value::Place(Place::local(local_id))
        })
        .collect();

    // 3. Create the closure function (takes &env as first param)
    create_closure_function(
        ctx,
        closure_name,
        Some((env_struct_id, env_struct_name)),
        captures,
        params,
        body,
        tail_expr,
        return_ty,
        span,
    );

    // 4. Use ApplyPartial to create thick callable
    let thick_ty = create_thick_func_type(ctx, params, return_ty);
    let result_local = ctx.create_temp("closure", thick_ty);
    let result_place = Place::local(result_local);

    ctx.emit_assign(
        result_place.clone(),
        Rvalue::ApplyPartial {
            func: closure_name,
            captures: capture_values,
        },
    );

    Value::Place(result_place)
}

/// Generate the environment struct for captured variables.
fn generate_env_struct(
    ctx: &mut LoweringContext,
    name: Id<QualifiedName>,
    captures: &[Capture],
    span: &Span,
) -> Id<Struct> {
    let struct_id = ctx.mir.add_struct(name);

    // Add fields for each capture
    for capture in captures {
        let field_ty = lower_type(ctx, &capture.ty);
        ctx.mir.add_field(struct_id, &capture.name, field_ty);
    }

    // Set Origin metadata
    let current_func = ctx.current_function_unwrap();
    let func_name = ctx.mir.function(current_func).name;
    ctx.mir.structs[struct_id].meta.origin = Some(Origin::ClosureEnv {
        containing_function: func_name,
        closure_span: span.clone(),
    });

    struct_id
}

/// Create the thick function type for the closure.
fn create_thick_func_type(
    ctx: &mut LoweringContext,
    params: &[EffectiveParam],
    return_ty: &SemanticTy,
) -> Id<Ty> {
    let param_tys: Vec<Id<Ty>> = params.iter().map(|p| lower_type(ctx, &p.ty)).collect();
    let ret_ty = lower_type(ctx, return_ty);

    ctx.mir.intern_type(MirTy::FuncThick {
        params: param_tys,
        ret: ret_ty,
    })
}

/// Create the closure's call function.
///
/// For capturing closures, the function takes `env: &EnvStruct` as its first parameter.
/// Captured variables are accessed via field reads from the env struct.
fn create_closure_function(
    ctx: &mut LoweringContext,
    name: Id<QualifiedName>,
    env_info: Option<(Id<Struct>, Id<QualifiedName>)>,
    captures: &[Capture],
    params: &[EffectiveParam],
    body: &[Statement],
    tail_expr: &Option<Box<Expression>>,
    return_ty: &SemanticTy,
    span: &Span,
) {
    let mir_ret_ty = lower_type(ctx, return_ty);

    // Save current context
    let saved_func = ctx.current_function();
    let saved_local_map = ctx.save_local_map();
    let saved_block = ctx.current_block();
    let saved_closure_counter = ctx.get_closure_counter();
    let saved_temp_counter = ctx.get_temp_counter();

    // Pre-compute types for env parameter and regular parameters to avoid borrow issues
    let env_param_ty = env_info.as_ref().map(|(_, env_struct_name)| {
        let env_struct_ty = ctx.mir.ty_named(*env_struct_name, vec![]);
        ctx.mir.ty_ref(env_struct_ty)
    });

    let param_types: Vec<_> = params
        .iter()
        .map(|p| (p.name.clone(), lower_type(ctx, &p.ty)))
        .collect();

    // Create the function
    let func_id = {
        let mut func = ctx.mir.add_function(name, mir_ret_ty);

        // Add env parameter if there are captures
        if let Some(env_ty) = env_param_ty {
            func.param("env", env_ty);
        }

        // Add regular parameters
        for (param_name, param_ty) in &param_types {
            func.param(param_name, *param_ty);
        }

        func.id()
    };

    // Set Origin metadata for closure call function
    if let Some((env_struct_id, _)) = env_info {
        ctx.mir.functions[func_id].meta.origin = Some(Origin::ClosureCall {
            env_struct: env_struct_id,
            closure_span: span.clone(),
        });
    }

    // Enter the new function context
    ctx.enter_function(func_id);

    // Get the locals that were created for the function parameters
    let mir_locals: Vec<_> = ctx.mir.function(func_id).locals.clone();

    // Calculate the offset for regular params (skip env param if present)
    let param_offset = if env_info.is_some() { 1 } else { 0 };

    // Map regular parameters to their MIR locals
    for (i, param) in params.iter().enumerate() {
        let mir_local_id = mir_locals[param_offset + i];
        ctx.map_local(param.local_id, mir_local_id);
    }

    // Create entry block
    let entry_block = ctx.create_block();
    ctx.set_current_block(entry_block);
    ctx.mir.function_mut(func_id).entry_block = Some(entry_block);

    // If we have captures, set up access by loading from the env struct
    if env_info.is_some() {
        // The env parameter is the first local (index 0)
        let env_local_id = mir_locals[0];

        // For each capture, create a local and load from the env struct field
        for capture in captures {
            let capture_ty = lower_type(ctx, &capture.ty);
            let capture_local = ctx.create_local(&capture.name, capture_ty);

            // Create field access: (deref env).field_name
            let env_place = Place::local(env_local_id);
            let deref_env = Place::deref(env_place);
            let field_place = Place::field(deref_env, capture.name.clone());

            // Copy the field value into the local
            ctx.emit_assign(Place::local(capture_local), Rvalue::Copy(field_place));

            // Map the capture's LocalId to this new local
            ctx.map_local(capture.local_id, capture_local);
        }
    }

    // Lower body statements
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Lower tail expression
    if !ctx.is_block_terminated() {
        if let Some(expr) = tail_expr {
            let value = lower_expression(ctx, expr);
            if !ctx.is_block_terminated() {
                ctx.emit_return(value);
            }
        } else {
            ctx.emit_return_unit();
        }
    }

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
