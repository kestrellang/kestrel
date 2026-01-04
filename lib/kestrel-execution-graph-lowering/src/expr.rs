//! Expression lowering - converts semantic expressions to MIR values.
//!
//! This is the core of the lowering pass. Each expression is converted to
//! a MIR Value (either a Place or an Immediate), potentially generating
//! statements and new basic blocks along the way.

use kestrel_execution_graph::{
    BinOp, CallArg, Callee, Id, Immediate, Local, MirTy, PassingMode, Place, QualifiedNameData,
    Rvalue, UnOp, Value,
};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::behavior::callable::{CallableBehavior, ParameterAccessMode};
use kestrel_semantic_tree::expr::{
    CallArgument, ElseBranch, ExprKind, Expression, IfCondition, LiteralValue, PrimitiveMethod,
};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::{ParamInfo, Ty, TyKind};

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::name::qualified_name_for_symbol;
use crate::stmt::lower_statement;
use crate::ty::lower_type;

/// Convert a ParameterAccessMode to a PassingMode based on type copyability.
///
/// For `Consuming` parameters, we check whether the argument type is copyable:
/// - Copyable types use `PassingMode::Copy` (value is duplicated)
/// - Non-copyable types use `PassingMode::Move` (value is moved, original becomes invalid)
///
/// Note: This function does NOT handle Cloneable types - those require emitting
/// a witness call before the function call, which is done separately in
/// `build_call_args_with_cloning`.
fn access_mode_to_passing_mode(mode: ParameterAccessMode, arg_ty: &Ty) -> PassingMode {
    match mode {
        ParameterAccessMode::Borrow => PassingMode::Ref,
        ParameterAccessMode::Mutating => PassingMode::MutRef,
        ParameterAccessMode::Consuming => {
            if arg_ty.is_copyable() {
                PassingMode::Copy
            } else {
                PassingMode::Move
            }
        }
    }
}

/// Emit a clone call for a Cloneable type.
///
/// For Cloneable types being passed to `consuming` parameters:
/// 1. Create a temp for the cloned value
/// 2. Emit a witness call: `%cloned = call witness_method Cloneable.clone for T (ref %original)`
/// 3. Return the cloned value
///
/// The caller is responsible for checking `is_cloneable()` before calling this.
fn emit_clone_call(ctx: &mut LoweringContext, value: &Value, ty: &Ty) -> Value {
    // Get the MIR type for the argument
    let mir_ty = lower_type(ctx, ty);

    // Create a temp for the cloned value
    let cloned_local = ctx.create_temp("cloned", mir_ty);
    let cloned_place = Place::local(cloned_local);

    // Look up the Cloneable protocol via the builtin registry
    let protocol_name = match ctx.model.builtin_registry().cloneable_protocol() {
        Some(cloneable_id) => {
            // Query the symbol to get its qualified name
            match ctx.model.query(SymbolFor { id: cloneable_id }) {
                Some(symbol) => qualified_name_for_symbol(ctx, &symbol),
                None => {
                    // Cloneable protocol symbol not found - internal error
                    ctx.emit_error(LoweringError::internal(
                        "Cloneable protocol symbol not found in registry",
                        None,
                    ));
                    // Return a dummy value to allow compilation to continue
                    return Value::Immediate(Immediate::unit());
                }
            }
        }
        None => {
            // Cloneable builtin not registered - std library may not be loaded
            ctx.emit_error(LoweringError::internal(
                "Cloneable builtin protocol not registered",
                None,
            ));
            // Return a dummy value to allow compilation to continue
            return Value::Immediate(Immediate::unit());
        }
    };

    // Create the witness callee: witness_method Cloneable.clone for T
    let callee = Callee::witness(protocol_name, "clone", mir_ty);

    // The clone method takes `self` by borrow (ref), so pass with PassingMode::Ref
    let call_args = vec![CallArg::borrow(value.clone())];

    // Emit the call: %cloned = call witness_method Cloneable.clone for T (ref %original)
    ctx.emit_call_with_modes(cloned_place.clone(), callee, call_args);

    // Track the cloned temp for deinit if the type needs it
    // (The cloned value is a new value we own, so it might need deinit at statement end
    // if it's not consumed. However, since we're immediately passing it to a function,
    // we typically don't need to track it here - it will be moved to the callee.)

    Value::Place(cloned_place)
}

/// Build CallArgs from values, types, and parameter access modes from a CallableBehavior.
///
/// This version handles Cloneable types by emitting witness calls to `Cloneable.clone`
/// for `consuming` parameters where the argument type is Cloneable but not Copyable.
///
/// For `consuming` parameters:
/// - Copyable types → PassingMode::Copy (bitwise copy)
/// - Cloneable types → emit clone call, then PassingMode::Move the cloned value
/// - Non-copyable, non-cloneable types → PassingMode::Move (original moved)
///
/// The `arg_types` slice provides the type of each argument, used to determine
/// whether a `Consuming` parameter should use Copy, Clone, or Move semantics.
///
/// If behavior is None (e.g., for indirect calls), defaults to Ref for all args.
fn build_call_args(
    ctx: &mut LoweringContext,
    arg_values: Vec<Value>,
    arg_types: &[&Ty],
    behavior: Option<&CallableBehavior>,
    is_instance_method: bool,
) -> Vec<CallArg> {
    match behavior {
        Some(beh) => {
            let params = beh.parameters();

            // For instance methods, we need to skip the implicit self parameter
            // when matching argument indices to parameter indices.
            // Instance method arguments: [receiver, arg0, arg1, ...]
            // Parameter list: [param0, param1, ...] (receiver is not in params)
            let param_offset = if is_instance_method { 1 } else { 0 };

            arg_values
                .into_iter()
                .enumerate()
                .map(|(i, value)| {
                    // For instance methods, the first arg (receiver) uses ReceiverKind
                    // which we handle separately. Skip it in the parameter lookup.
                    if is_instance_method && i == 0 {
                        // Receiver - use Ref for now (borrowing receiver)
                        // TODO: Check ReceiverKind for mutating/consuming receivers
                        CallArg::borrow(value)
                    } else {
                        let param_idx = i - param_offset;
                        if let Some(param) = params.get(param_idx) {
                            // Get the argument type for this position
                            if let Some(arg_ty) = arg_types.get(i).copied() {
                                // Check if this is a consuming parameter with a cloneable type
                                if param.access_mode() == ParameterAccessMode::Consuming
                                    && arg_ty.is_cloneable()
                                {
                                    // For Cloneable types, emit a clone call and move the result
                                    let cloned_value = emit_clone_call(ctx, &value, arg_ty);
                                    // Clone was emitted - move the cloned value
                                    return CallArg::new(cloned_value, PassingMode::Move);
                                }

                                // Handle based on access mode
                                match param.access_mode() {
                                    ParameterAccessMode::Borrow => {
                                        // Create a reference to the argument
                                        let ref_value = create_ref(ctx, &value, arg_ty, false);
                                        CallArg::new(ref_value, PassingMode::Copy)
                                    }
                                    ParameterAccessMode::Mutating => {
                                        // Create a mutable reference to the argument
                                        let ref_value = create_ref(ctx, &value, arg_ty, true);
                                        CallArg::new(ref_value, PassingMode::Copy)
                                    }
                                    ParameterAccessMode::Consuming => {
                                        // Pass by value (copy or move)
                                        let mode = if arg_ty.is_copyable() {
                                            PassingMode::Copy
                                        } else {
                                            PassingMode::Move
                                        };
                                        CallArg::new(value, mode)
                                    }
                                }
                            } else {
                                // Fallback if type not found (shouldn't happen after type checking)
                                // Default to Copy for Consuming since most types are copyable
                                let mode = match param.access_mode() {
                                    ParameterAccessMode::Borrow => PassingMode::Ref,
                                    ParameterAccessMode::Mutating => PassingMode::MutRef,
                                    ParameterAccessMode::Consuming => PassingMode::Copy,
                                };
                                CallArg::new(value, mode)
                            }
                        } else {
                            // Fallback if parameter not found (shouldn't happen after type checking)
                            CallArg::borrow(value)
                        }
                    }
                })
                .collect()
        }
        None => {
            // No behavior available - default to Ref for all arguments
            arg_values.into_iter().map(CallArg::borrow).collect()
        }
    }
}

/// Create a reference (or mutable reference) to a value.
///
/// For values that are already places, emits `Rvalue::Ref` or `Rvalue::RefMut`.
/// For immediates, creates a temporary and takes a reference to that.
fn create_ref(ctx: &mut LoweringContext, value: &Value, ty: &Ty, is_mutable: bool) -> Value {
    match value {
        Value::Place(place) => {
            // Take a reference to the place
            let base_mir_ty = lower_type(ctx, ty);
            let ref_ty = if is_mutable {
                ctx.mir.ty_ref_mut(base_mir_ty)
            } else {
                ctx.mir.ty_ref(base_mir_ty)
            };
            let ref_local = ctx.create_temp("arg_ref", ref_ty);
            let ref_place = Place::local(ref_local);

            let rvalue = if is_mutable {
                Rvalue::RefMut(place.clone())
            } else {
                Rvalue::Ref(place.clone())
            };
            ctx.emit_assign(ref_place.clone(), rvalue);

            Value::Place(ref_place)
        }
        Value::Immediate(imm) => {
            // For immediates, we need to spill to a temp first, then take a reference
            let base_mir_ty = lower_type(ctx, ty);
            let temp_local = ctx.create_temp("arg_temp", base_mir_ty);
            let temp_place = Place::local(temp_local);

            // Store the immediate in the temp
            ctx.emit_assign(temp_place.clone(), Rvalue::Use(imm.clone()));

            // Take a reference to the temp
            let ref_ty = if is_mutable {
                ctx.mir.ty_ref_mut(base_mir_ty)
            } else {
                ctx.mir.ty_ref(base_mir_ty)
            };
            let ref_local = ctx.create_temp("arg_ref", ref_ty);
            let ref_place = Place::local(ref_local);

            let rvalue = if is_mutable {
                Rvalue::RefMut(temp_place)
            } else {
                Rvalue::Ref(temp_place)
            };
            ctx.emit_assign(ref_place.clone(), rvalue);

            Value::Place(ref_place)
        }
    }
}

/// Extract the local ID from a Value if it's a simple local reference.
/// Returns None for complex places (field access, etc.) or immediates.
fn try_get_local_from_value(value: &Value) -> Option<Id<Local>> {
    match value {
        Value::Place(place) => place.as_local(),
        _ => None,
    }
}

/// Mark locals as moved for any arguments passed with Move mode.
fn mark_moved_args(ctx: &mut LoweringContext, call_args: &[CallArg]) {
    for arg in call_args {
        if arg.mode == PassingMode::Move {
            if let Some(local) = try_get_local_from_value(&arg.value) {
                ctx.mark_moved(local);
            }
        }
    }
}

/// Build call arguments for indirect calls (function pointers/closures).
///
/// For indirect calls, we don't have CallableBehavior, so we determine passing
/// semantics from the function type itself. In Kestrel, function type parameters
/// are passed by reference (borrow) by default.
fn build_indirect_call_args(
    ctx: &mut LoweringContext,
    arg_values: Vec<Value>,
    arg_types: &[&Ty],
    callee_ty: &Ty,
) -> Vec<CallArg> {
    // Get the parameter types from the callee's function type
    let param_types: Vec<&Ty> = match callee_ty.kind() {
        TyKind::Function { params, .. } => params.iter().collect(),
        TyKind::UnresolvedFunction { param_info, .. } => {
            match param_info {
                ParamInfo::Explicit { param_types } => param_types.iter().collect(),
                ParamInfo::ImplicitIt { it_type } => vec![it_type.as_ref()],
                ParamInfo::Unconstrained => vec![], // No param info available
            }
        }
        _ => vec![], // Not a function type, shouldn't happen
    };

    arg_values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            // Get the argument type if available
            if let Some(&arg_ty) = arg_types.get(i) {
                // For function pointer/closure calls, parameters are passed by reference
                // (this matches Kestrel's default borrow semantics)
                let ref_value = create_ref(ctx, &value, arg_ty, false);
                CallArg::new(ref_value, PassingMode::Copy)
            } else {
                // Fallback: no type info, just borrow as-is
                // This path shouldn't be hit in practice after type checking
                CallArg::borrow(value)
            }
        })
        .collect()
}

/// Lower an expression to a MIR Value.
///
/// This may generate statements in the current block and potentially
/// create new basic blocks for control flow.
///
/// # Returns
///
/// Returns a `Value` representing the expression result. For simple expressions
/// like literals, this is an immediate. For expressions that reference memory
/// locations, this is a place. For complex expressions, a temporary local may
/// be created and its place returned.
pub fn lower_expression(ctx: &mut LoweringContext, expr: &Expression) -> Value {
    match &expr.kind {
        // === Literals ===
        ExprKind::Literal(lit) => lower_literal(lit, expr),

        // === Variable References ===
        ExprKind::LocalRef(local_id) => {
            let mir_local = ctx.get_local_unwrap(*local_id);
            let local_place = Place::local(mir_local);

            // Check if this local is a reference type (e.g., parameter with borrow/mutating mode).
            // If so, we need to dereference it to get the underlying value.
            let local_def = ctx.mir.local(mir_local);
            let local_mir_ty = ctx.mir.ty(local_def.ty);

            if matches!(local_mir_ty, MirTy::Ref(_) | MirTy::RefMut(_)) {
                // This is a reference-typed local (e.g., borrow or mutating parameter).
                // Dereference it to access the underlying value.
                Value::Place(local_place.deref())
            } else {
                Value::Place(local_place)
            }
        }

        ExprKind::SymbolRef(symbol_id) => {
            // SymbolRef represents a reference to a symbol as a first-class value.
            // This includes:
            // - Module-level functions (e.g., `let f = myFunction`)
            // - Enum cases with associated values (e.g., `let f = Option.Some`)
            // - Initializers (e.g., `let f = SomeStruct.init`)
            let symbol = ctx.model.query(SymbolFor { id: *symbol_id });
            match symbol {
                Some(sym) => {
                    let kind = sym.metadata().kind();
                    match kind {
                        KestrelSymbolKind::Function
                        | KestrelSymbolKind::Initializer
                        | KestrelSymbolKind::EnumCase => {
                            // Function/callable reference as first-class value
                            let func_name = qualified_name_for_symbol(ctx, &sym);
                            // For now, emit without type args - generic function references
                            // would need the substitutions from the expression context
                            Value::Immediate(Immediate::function_ref(func_name))
                        }
                        KestrelSymbolKind::Field => {
                            // Global variable access - not yet supported
                            ctx.emit_error(LoweringError::unsupported_expr(
                                "global variable access",
                                expr.span.clone(),
                            ));
                            Value::Immediate(Immediate::error())
                        }
                        _ => {
                            ctx.emit_error(LoweringError::unsupported_expr(
                                format!("SymbolRef to {:?}", kind),
                                expr.span.clone(),
                            ));
                            Value::Immediate(Immediate::error())
                        }
                    }
                }
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("symbol not found: {:?}", symbol_id),
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                }
            }
        }

        // === Field Access ===
        ExprKind::FieldAccess { object, field } => {
            let obj_value = lower_expression(ctx, object);
            match obj_value {
                Value::Place(p) => Value::Place(p.field(field)),
                Value::Immediate(_) => {
                    // Can't access field of an immediate - this shouldn't happen
                    // after type checking
                    ctx.emit_error(LoweringError::internal(
                        "field access on immediate value",
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                }
            }
        }

        ExprKind::TupleIndex { tuple, index } => {
            let tuple_value = lower_expression(ctx, tuple);
            match tuple_value {
                Value::Place(p) => Value::Place(p.index(*index)),
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::internal(
                        "tuple index on immediate value",
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                }
            }
        }

        // === Assignment ===
        ExprKind::Assignment { target, value } => {
            let target_place = match lower_expression(ctx, target) {
                Value::Place(p) => p,
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::internal(
                        "assignment to non-place",
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                }
            };

            let rhs_value = lower_expression(ctx, value);

            // Assignment uses copy semantics for now
            // TODO: Use move for non-Copy types
            ctx.emit_assign_value(target_place, rhs_value);

            // Assignment expression yields unit (actually Never in semantic tree)
            Value::Immediate(Immediate::unit())
        }

        // === Primitive Method Calls (operators) ===
        ExprKind::PrimitiveMethodCall {
            receiver,
            method,
            arguments,
        } => lower_primitive_method_call(ctx, receiver, *method, arguments, expr),

        // === Struct Construction ===
        ExprKind::ImplicitStructInit {
            struct_type,
            arguments,
        } => lower_struct_init(ctx, struct_type, arguments, expr),

        // === Function/Method Calls ===
        ExprKind::Call {
            callee,
            arguments,
            substitutions,
        } => lower_call(ctx, callee, arguments, substitutions, expr),

        // === Control Flow ===
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => lower_if(ctx, conditions, then_branch, then_value, else_branch, expr),

        ExprKind::While {
            loop_id,
            label: _,
            condition,
            body,
        } => lower_while(ctx, *loop_id, condition, body, expr),

        ExprKind::Loop {
            loop_id,
            label: _,
            body,
        } => lower_loop(ctx, *loop_id, body, expr),

        ExprKind::WhileLet {
            loop_id,
            label: _,
            conditions,
            body,
        } => lower_while_let(ctx, *loop_id, conditions, body),

        ExprKind::Break { loop_id, label: _ } => {
            // Find the target loop and jump to its exit block
            if let Some(loop_info) = ctx.find_loop(*loop_id) {
                let exit_block = loop_info.exit_block;
                // Emit deinits for scopes between current and target loop
                ctx.emit_deinits_to_loop(*loop_id);
                ctx.emit_jump(exit_block);
            } else {
                ctx.emit_error(LoweringError::internal(
                    "break: loop not found in loop stack",
                    Some(expr.span.clone()),
                ));
            }
            // Break never produces a value (it transfers control)
            Value::Immediate(Immediate::unit())
        }

        ExprKind::Continue { loop_id, label: _ } => {
            // Find the target loop and jump to its header block
            if let Some(loop_info) = ctx.find_loop(*loop_id) {
                let header_block = loop_info.header_block;
                // Emit deinits for scopes between current and target loop
                ctx.emit_deinits_to_loop(*loop_id);
                ctx.emit_jump(header_block);
            } else {
                ctx.emit_error(LoweringError::internal(
                    "continue: loop not found in loop stack",
                    Some(expr.span.clone()),
                ));
            }
            // Continue never produces a value (it transfers control)
            Value::Immediate(Immediate::unit())
        }

        ExprKind::Return { value } => {
            let ret_value = if let Some(v) = value {
                lower_expression(ctx, v)
            } else {
                Value::Immediate(Immediate::unit())
            };
            // Emit deinits for all scopes before returning
            ctx.emit_all_scope_deinits();
            ctx.emit_return(ret_value);
            // Return a unit value even though this is never used (block is terminated)
            Value::Immediate(Immediate::unit())
        }

        // === Match Expressions ===
        ExprKind::Match { scrutinee, arms } => {
            crate::match_lowering::lower_match_expr(ctx, scrutinee, arms, expr)
        }

        // === Closures ===
        ExprKind::Closure {
            params,
            body,
            tail_expr,
            captures,
            uses_it,
            implicit_param,
        } => crate::closure::lower_closure(
            ctx,
            params,
            body,
            tail_expr,
            captures,
            implicit_param,
            *uses_it,
            &expr.ty,
            &expr.span,
        ),

        // === Other ===
        ExprKind::Array(elements) => {
            // Lower each element
            let element_values: Vec<Value> =
                elements.iter().map(|e| lower_expression(ctx, e)).collect();

            // Get the array element type from the expression type
            let (element_ty, elem_sem_ty) = match expr.ty.kind() {
                kestrel_semantic_tree::ty::TyKind::Array(elem_ty) => {
                    (lower_type(ctx, elem_ty), Some(elem_ty))
                }
                _ => {
                    ctx.emit_error(LoweringError::internal(
                        "array literal with non-array type",
                        Some(expr.span.clone()),
                    ));
                    (ctx.mir.ty_error(), None)
                }
            };

            // Create result local and emit array construction
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("array", result_ty);
            let result_place = Place::local(result_local);

            // Track the temp for deinit if array element type needs deinit
            if let Some(elem_ty) = elem_sem_ty {
                if ctx.type_needs_deinit(elem_ty) {
                    ctx.track_statement_temp(result_local);
                }
            }

            ctx.emit_assign(
                result_place.clone(),
                Rvalue::Array {
                    element_ty,
                    elements: element_values,
                },
            );

            Value::Place(result_place)
        }

        ExprKind::Tuple(elements) => {
            // Lower each element
            let element_values: Vec<Value> =
                elements.iter().map(|e| lower_expression(ctx, e)).collect();

            // Create result local and emit tuple construction
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("tuple", result_ty);
            let result_place = Place::local(result_local);

            // Track the temp for deinit if any tuple element type needs deinit
            let needs_deinit = elements.iter().any(|e| ctx.type_needs_deinit(&e.ty));
            if needs_deinit {
                ctx.track_statement_temp(result_local);
            }

            ctx.emit_assign(result_place.clone(), Rvalue::Tuple(element_values));

            Value::Place(result_place)
        }

        ExprKind::Grouping(inner) => lower_expression(ctx, inner),

        ExprKind::OverloadedRef(_) => {
            // Should be resolved by now
            ctx.emit_error(LoweringError::internal(
                "unresolved overloaded reference",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        }

        ExprKind::TypeRef(_) => {
            // Type references shouldn't appear as values
            ctx.emit_error(LoweringError::internal(
                "type reference as value",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        }

        ExprKind::TypeParameterRef(_) => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "type parameter reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::error())
        }

        ExprKind::AssociatedTypeRef => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "associated type reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::error())
        }

        ExprKind::MethodRef {
            receiver,
            candidates,
            method_name,
        } => {
            // Method reference without a call - creates a bound method
            crate::bound_method::lower_bound_method(
                ctx,
                receiver,
                candidates,
                method_name,
                &expr.ty,
                &expr.span,
            )
        }

        ExprKind::EnumCase { case_id } => {
            // Simple enum case (no associated values)
            // Look up the case symbol to get its name
            let case_symbol = ctx.model.query(SymbolFor { id: *case_id });
            match case_symbol {
                Some(sym) => {
                    let variant_name = sym.metadata().name().value.clone();

                    // Get the enum type from the expression type
                    let enum_ty = lower_type(ctx, &expr.ty);

                    // Create result and emit enum variant construction
                    let result_local = ctx.create_temp("enum", enum_ty);
                    let result_place = Place::local(result_local);

                    // Track the temp for deinit if the enum type needs deinit
                    if ctx.type_needs_deinit(&expr.ty) {
                        ctx.track_statement_temp(result_local);
                    }

                    ctx.emit_assign(
                        result_place.clone(),
                        Rvalue::EnumVariant {
                            enum_ty,
                            variant: variant_name,
                            payload: vec![],
                        },
                    );

                    Value::Place(result_place)
                }
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("enum case symbol not found: {:?}", case_id),
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                }
            }
        }

        ExprKind::ImplicitMemberAccess {
            member_name,
            arguments: _,
        } => {
            // Should be resolved by type inference
            ctx.emit_error(LoweringError::internal(
                format!("unresolved implicit member '.{}'", member_name),
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        }

        ExprKind::Error => {
            // Error expression - return error value (error already reported)
            Value::Immediate(Immediate::error())
        }
    }
}

/// Lower a literal expression.
fn lower_literal(lit: &LiteralValue, _expr: &Expression) -> Value {
    match lit {
        LiteralValue::Unit => Value::Immediate(Immediate::unit()),
        LiteralValue::Integer(n) => Value::Immediate(Immediate::i64(*n)),
        LiteralValue::Float(f) => Value::Immediate(Immediate::f64(*f)),
        LiteralValue::Bool(b) => Value::Immediate(Immediate::bool(*b)),
        LiteralValue::String(s) => Value::Immediate(Immediate::string(s.clone())),
    }
}

/// Lower a primitive method call (operators).
fn lower_primitive_method_call(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    method: PrimitiveMethod,
    arguments: &[CallArgument],
    expr: &Expression,
) -> Value {
    let receiver_value = lower_expression(ctx, receiver);

    // Determine if this is a unary or binary operation
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("prim", result_ty);
    let result_place = Place::local(result_local);

    match method {
        // === Unary Operations ===
        PrimitiveMethod::IntNeg | PrimitiveMethod::IntIdentity => {
            let op = match method {
                PrimitiveMethod::IntNeg => UnOp::Neg,
                PrimitiveMethod::IntIdentity => {
                    // Identity just returns the value
                    return receiver_value;
                }
                _ => unreachable!(),
            };
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op,
                    operand: receiver_value,
                },
            );
        }

        PrimitiveMethod::FloatNeg | PrimitiveMethod::FloatIdentity => {
            let op = match method {
                PrimitiveMethod::FloatNeg => UnOp::FNeg,
                PrimitiveMethod::FloatIdentity => {
                    return receiver_value;
                }
                _ => unreachable!(),
            };
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op,
                    operand: receiver_value,
                },
            );
        }

        PrimitiveMethod::BoolNot => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::BoolNot,
                    operand: receiver_value,
                },
            );
        }

        PrimitiveMethod::IntBitNot => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::Not,
                    operand: receiver_value,
                },
            );
        }

        // === String methods (unary) ===
        PrimitiveMethod::StringLength => {
            // string.length() -> StrLen(string)
            ctx.emit_assign(result_place.clone(), Rvalue::StrLen(receiver_value));
        }

        PrimitiveMethod::StringIsEmpty => {
            // string.isEmpty() -> StrLen(string) == 0
            // First get the length
            let len_ty = ctx.mir.ty_i64();
            let len_local = ctx.create_temp("len", len_ty);
            let len_place = Place::local(len_local);
            ctx.emit_assign(len_place.clone(), Rvalue::StrLen(receiver_value));

            // Then compare to 0
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::Eq,
                    lhs: Value::Place(len_place),
                    rhs: Value::Immediate(Immediate::i64(0)),
                },
            );
        }

        // === Int methods (unary) ===
        PrimitiveMethod::IntAbs => {
            // int.abs() -> if int < 0 { -int } else { int }
            // We implement this with: (int ^ (int >> 63)) - (int >> 63)
            // This is a branchless abs for signed integers
            //
            // Alternative: just emit a conditional
            let int_ty = lower_type(ctx, &receiver.ty);

            // First, we need the receiver in a place if it's not already
            let receiver_place = match &receiver_value {
                Value::Place(p) => p.clone(),
                Value::Immediate(imm) => {
                    let temp = ctx.create_temp("abs_input", int_ty);
                    let temp_place = Place::local(temp);
                    ctx.emit_assign(temp_place.clone(), Rvalue::Use(imm.clone()));
                    temp_place
                }
            };

            // Create blocks for the conditional
            let neg_block = ctx.create_block();
            let pos_block = ctx.create_block();
            let join_block = ctx.create_block();

            // Check if value < 0
            let cmp_ty = ctx.mir.ty_bool();
            let cmp_local = ctx.create_temp("is_neg", cmp_ty);
            let cmp_place = Place::local(cmp_local);
            ctx.emit_assign(
                cmp_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::LtSigned,
                    lhs: Value::Place(receiver_place.clone()),
                    rhs: Value::Immediate(Immediate::i64(0)),
                },
            );

            ctx.emit_branch(Value::Place(cmp_place), neg_block, pos_block);

            // Negative case: result = -value
            ctx.set_current_block(neg_block);
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::Neg,
                    operand: Value::Place(receiver_place.clone()),
                },
            );
            ctx.emit_jump(join_block);

            // Positive case: result = value
            ctx.set_current_block(pos_block);
            ctx.emit_assign_value(result_place.clone(), Value::Place(receiver_place));
            ctx.emit_jump(join_block);

            // Continue from join block
            ctx.set_current_block(join_block);
        }

        PrimitiveMethod::IntToString => {
            // Convert integer to string using the IntToString operation
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("str", result_ty);
            let result_place = Place::local(result_local);

            ctx.emit_assign(result_place.clone(), Rvalue::IntToString(receiver_value));
            return Value::Place(result_place);
        }

        // === Binary Operations ===
        _ => {
            // Binary operations take one argument
            if arguments.is_empty() {
                ctx.emit_error(LoweringError::internal(
                    "binary primitive method with no arguments",
                    Some(expr.span.clone()),
                ));
                return Value::Immediate(Immediate::error());
            }

            let rhs_value = lower_expression(ctx, &arguments[0].value);

            let op = match method {
                // Integer arithmetic
                PrimitiveMethod::IntAdd => BinOp::AddSigned,
                PrimitiveMethod::IntSub => BinOp::SubSigned,
                PrimitiveMethod::IntMul => BinOp::MulSigned,
                PrimitiveMethod::IntDiv => BinOp::DivSigned,
                PrimitiveMethod::IntRem => BinOp::RemSigned,

                // Integer comparison
                PrimitiveMethod::IntEq => BinOp::Eq,
                PrimitiveMethod::IntNe => BinOp::Ne,
                PrimitiveMethod::IntLt => BinOp::LtSigned,
                PrimitiveMethod::IntLe => BinOp::LeSigned,
                PrimitiveMethod::IntGt => BinOp::GtSigned,
                PrimitiveMethod::IntGe => BinOp::GeSigned,

                // Integer bitwise
                PrimitiveMethod::IntBitAnd => BinOp::And,
                PrimitiveMethod::IntBitOr => BinOp::Or,
                PrimitiveMethod::IntBitXor => BinOp::Xor,
                PrimitiveMethod::IntShl => BinOp::Shl,
                PrimitiveMethod::IntShr => BinOp::ShrSigned,

                // Float arithmetic
                PrimitiveMethod::FloatAdd => BinOp::FAdd,
                PrimitiveMethod::FloatSub => BinOp::FSub,
                PrimitiveMethod::FloatMul => BinOp::FMul,
                PrimitiveMethod::FloatDiv => BinOp::FDiv,

                // Float comparison
                PrimitiveMethod::FloatEq => BinOp::FEq,
                PrimitiveMethod::FloatNe => BinOp::FNe,
                PrimitiveMethod::FloatLt => BinOp::FLt,
                PrimitiveMethod::FloatLe => BinOp::FLe,
                PrimitiveMethod::FloatGt => BinOp::FGt,
                PrimitiveMethod::FloatGe => BinOp::FGe,

                // Boolean
                PrimitiveMethod::BoolAnd => BinOp::BoolAnd,
                PrimitiveMethod::BoolOr => BinOp::BoolOr,
                PrimitiveMethod::BoolEq => BinOp::Eq,
                PrimitiveMethod::BoolNe => BinOp::Ne,

                // String comparison
                PrimitiveMethod::StringEq => BinOp::Eq,
                PrimitiveMethod::StringNe => BinOp::Ne,

                // Already handled above
                _ => unreachable!(),
            };

            ctx.emit_assign(
                result_place.clone(),
                Rvalue::BinaryOp {
                    op,
                    lhs: receiver_value,
                    rhs: rhs_value,
                },
            );
        }
    }

    Value::Place(result_place)
}

/// Lower a struct initialization.
fn lower_struct_init(
    ctx: &mut LoweringContext,
    struct_type: &kestrel_semantic_tree::ty::Ty,
    arguments: &[CallArgument],
    _expr: &Expression,
) -> Value {
    // Get the MIR type for the struct
    let mir_ty = lower_type(ctx, struct_type);

    // Create a temporary for the result
    let result_local = ctx.create_temp("struct", mir_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit at statement end if the struct type needs deinit
    if ctx.type_needs_deinit(struct_type) {
        ctx.track_statement_temp(result_local);
    }

    // Lower the field values
    let fields: Vec<(String, Value)> = arguments
        .iter()
        .map(|arg| {
            let field_name = arg.label.clone().unwrap_or_else(|| "<unnamed>".to_string());
            let value = lower_expression(ctx, &arg.value);
            (field_name, value)
        })
        .collect();

    // Emit construct statement
    ctx.emit_assign(
        result_place.clone(),
        Rvalue::Construct { ty: mir_ty, fields },
    );

    Value::Place(result_place)
}

/// Lower a function call.
fn lower_call(
    ctx: &mut LoweringContext,
    callee: &Expression,
    arguments: &[CallArgument],
    substitutions: &kestrel_semantic_tree::ty::Substitutions,
    expr: &Expression,
) -> Value {
    use kestrel_semantic_tree::symbol::function::FunctionSymbol;
    use semantic_tree::symbol::Symbol;

    // Lower arguments first
    let arg_values: Vec<Value> = arguments
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Extract argument types for determining Copy vs Move passing modes
    let arg_types: Vec<&Ty> = arguments.iter().map(|arg| &arg.value.ty).collect();

    // Helper to get ordered type args from a symbol's type parameters
    let get_ordered_type_args =
        |ctx: &mut LoweringContext,
         sym: &std::sync::Arc<dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>>|
         -> Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> {
            use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
            use kestrel_semantic_tree::symbol::EnumSymbol;
            use semantic_tree::symbol::SymbolId;

            // Try to get type parameters from different symbol types
            let param_ids: Option<Vec<SymbolId>> =
                if let Some(func_sym) = sym.as_ref().downcast_ref::<FunctionSymbol>() {
                    let type_params = func_sym.type_parameters();
                    Some(
                        type_params
                            .iter()
                            .map(|tp| Symbol::metadata(tp.as_ref()).id())
                            .collect(),
                    )
                } else if let Some(struct_sym) = sym.as_ref().downcast_ref::<StructSymbol>() {
                    let type_params = struct_sym.type_parameters();
                    Some(
                        type_params
                            .iter()
                            .map(|tp| Symbol::metadata(tp.as_ref()).id())
                            .collect(),
                    )
                } else if let Some(enum_sym) = sym.as_ref().downcast_ref::<EnumSymbol>() {
                    let type_params = enum_sym.type_parameters();
                    Some(
                        type_params
                            .iter()
                            .map(|tp| Symbol::metadata(tp.as_ref()).id())
                            .collect(),
                    )
                } else {
                    // For initializers and other symbols without type_parameters,
                    // they inherit from parent - just use the fallback
                    None
                };

            if let Some(ids) = param_ids {
                if let Some(ordered_types) = substitutions.types_in_order(&ids) {
                    return ordered_types
                        .into_iter()
                        .map(|ty| lower_type(ctx, ty))
                        .collect();
                }
            }
            // Fallback: use arbitrary order (should only happen for non-generic symbols or errors)
            substitutions
                .types()
                .map(|ty| lower_type(ctx, ty))
                .collect()
        };

    // Get the result type and create a temp for the result
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("call", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit at statement end if the return type needs deinit
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Determine the callee and emit the call
    match &callee.kind {
        ExprKind::SymbolRef(symbol_id) => {
            // Direct function call
            // Look up the symbol to get its qualified name
            let symbol = ctx.model.query(SymbolFor { id: *symbol_id });
            match symbol {
                Some(sym) => {
                    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

                    let kind = sym.metadata().kind();

                    if kind == KestrelSymbolKind::EnumCase {
                        // Enum case with associated values
                        let variant_name = sym.metadata().name().value.clone();

                        ctx.emit_assign(
                            result_place.clone(),
                            Rvalue::EnumVariant {
                                enum_ty: result_ty,
                                variant: variant_name,
                                payload: arg_values,
                            },
                        );
                    } else if kind == KestrelSymbolKind::Initializer {
                        // Initializer call - need to allocate self and pass as first arg
                        // Initializers have signature: func Type.init(self: &var Type, params...) -> ()

                        // Check if return type is a type parameter (T() where T: Factory)
                        let is_type_param_init = matches!(expr.ty.kind(), TyKind::TypeParameter(_));

                        // Create a mutable reference to the result place
                        let ref_ty = ctx.mir.ty_ref_mut(result_ty);
                        let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                        let self_ref_place = Place::local(self_ref_local);

                        // Emit: %self_ref = ref var %result
                        ctx.emit_assign(
                            self_ref_place.clone(),
                            Rvalue::RefMut(result_place.clone()),
                        );

                        // Look up CallableBehavior to get parameter access modes for user args
                        let callable_beh = sym.metadata().get_behavior::<CallableBehavior>();

                        // Build call args: self_ref first (always MutRef), then user args with their modes
                        let mut call_args = vec![CallArg::mutating(Value::Place(self_ref_place))];

                        // Add user-provided arguments with their access modes
                        if let Some(beh) = callable_beh {
                            let params = beh.parameters();
                            for (i, value) in arg_values.into_iter().enumerate() {
                                if let Some(param) = params.get(i) {
                                    // Get the argument type for Copy vs Move determination
                                    if let Some(arg_ty) = arg_types.get(i).copied() {
                                        let mode = access_mode_to_passing_mode(
                                            param.access_mode(),
                                            arg_ty,
                                        );
                                        call_args.push(CallArg::new(value, mode));
                                    } else {
                                        // Fallback: default to Copy for Consuming
                                        let mode = match param.access_mode() {
                                            ParameterAccessMode::Borrow => PassingMode::Ref,
                                            ParameterAccessMode::Mutating => PassingMode::MutRef,
                                            ParameterAccessMode::Consuming => PassingMode::Copy,
                                        };
                                        call_args.push(CallArg::new(value, mode));
                                    }
                                } else {
                                    call_args.push(CallArg::borrow(value));
                                }
                            }
                        } else {
                            call_args.extend(arg_values.into_iter().map(CallArg::borrow));
                        }

                        // Create a temp for the unit return value of init (we discard it)
                        let unit_ty = ctx.mir.ty_unit();
                        let unit_local = ctx.create_temp("init_ret", unit_ty);
                        let unit_place = Place::local(unit_local);

                        // Mark moved args before the call (call_args is consumed)
                        mark_moved_args(ctx, &call_args);

                        if is_type_param_init {
                            // Protocol initializer on type parameter: T() where T: Factory
                            // The parent of the init symbol should be the protocol
                            if let Some(parent) = sym.metadata().parent() {
                                if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                                    let protocol_name = qualified_name_for_symbol(ctx, &parent);
                                    let for_type = lower_type(ctx, &expr.ty);
                                    let mir_callee =
                                        Callee::witness(protocol_name, "init", for_type);
                                    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                                } else {
                                    ctx.emit_error(LoweringError::internal(
                                        "init's parent is not a protocol for type parameter init",
                                        Some(expr.span.clone()),
                                    ));
                                    return Value::Immediate(Immediate::error());
                                }
                            } else {
                                ctx.emit_error(LoweringError::internal(
                                    "init has no parent for type parameter init",
                                    Some(expr.span.clone()),
                                ));
                                return Value::Immediate(Immediate::error());
                            }
                        } else {
                            // Regular initializer call
                            let func_name = qualified_name_for_symbol(ctx, &sym);
                            let type_args = get_ordered_type_args(ctx, &sym);
                            let mir_callee = if type_args.is_empty() {
                                Callee::direct(func_name)
                            } else {
                                Callee::direct_generic(func_name, type_args)
                            };
                            ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
                        }

                        // result_place now contains the initialized struct
                    } else {
                        // Regular function call
                        let func_name = qualified_name_for_symbol(ctx, &sym);
                        let type_args = get_ordered_type_args(ctx, &sym);
                        let mir_callee = if type_args.is_empty() {
                            Callee::direct(func_name)
                        } else {
                            Callee::direct_generic(func_name, type_args)
                        };

                        // Look up CallableBehavior to get parameter access modes
                        let callable_beh = sym.metadata().get_behavior::<CallableBehavior>();
                        let call_args = build_call_args(
                            ctx,
                            arg_values,
                            &arg_types,
                            callable_beh.as_deref(),
                            false,
                        );
                        mark_moved_args(ctx, &call_args);
                        ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
                    }
                }
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("symbol not found for call: {:?}", symbol_id),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                }
            }
        }

        ExprKind::MethodRef {
            receiver,
            candidates,
            method_name,
        } => {
            // Check if this is a call on a type parameter (needs witness method lookup)
            let is_type_param_call = matches!(receiver.ty.kind(), TyKind::TypeParameter(_));
            let is_static_type_param_call = matches!(receiver.kind, ExprKind::TypeParameterRef(_));

            // Check if this is a call on an associated type (also needs witness method lookup)
            let is_assoc_type_call = matches!(receiver.ty.kind(), TyKind::AssociatedType { .. });
            let is_static_assoc_type_call = matches!(receiver.kind, ExprKind::AssociatedTypeRef);

            // Determine if this is an instance method call (has receiver value)
            let is_instance = !(is_static_type_param_call || is_static_assoc_type_call);

            // For instance methods on type params, receiver becomes first argument
            // For static methods on type params/assoc types, there's no receiver value
            let (all_args, all_arg_types): (Vec<Value>, Vec<&Ty>) = if !is_instance {
                // Static method call on type parameter or associated type: T.create(), T.Item.create()
                // No receiver value, just the arguments
                (arg_values, arg_types)
            } else {
                // Instance method call: a.add(b) where a: T
                let receiver_value = lower_expression(ctx, receiver);
                let mut all_args = vec![receiver_value];
                all_args.extend(arg_values);
                // Prepend receiver type to types list
                let mut all_types = vec![&receiver.ty];
                all_types.extend(arg_types);
                (all_args, all_types)
            };

            // For methods, we need to find the resolved method from candidates
            // During type inference, the correct candidate should have been selected
            if let Some(&method_id) = candidates.first() {
                let method_symbol = ctx.model.query(SymbolFor { id: method_id });
                match method_symbol {
                    Some(sym) => {
                        // Look up CallableBehavior to get parameter access modes
                        let callable_beh = sym.metadata().get_behavior::<CallableBehavior>();
                        let call_args = build_call_args(
                            ctx,
                            all_args,
                            &all_arg_types,
                            callable_beh.as_deref(),
                            is_instance,
                        );

                        // Mark moved args before the call (call_args is consumed)
                        mark_moved_args(ctx, &call_args);

                        // Check if this is a witness method call (method on type parameter or associated type)
                        if is_type_param_call
                            || is_static_type_param_call
                            || is_assoc_type_call
                            || is_static_assoc_type_call
                        {
                            // Get the protocol from the method's parent
                            if let Some(parent) = sym.metadata().parent() {
                                if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                                    let protocol_name = qualified_name_for_symbol(ctx, &parent);
                                    let for_type = lower_type(ctx, &receiver.ty);
                                    let mir_callee = Callee::witness(
                                        protocol_name,
                                        method_name.clone(),
                                        for_type,
                                    );
                                    ctx.emit_call_with_modes(
                                        result_place.clone(),
                                        mir_callee,
                                        call_args,
                                    );
                                } else {
                                    // Method's parent is not a protocol - shouldn't happen
                                    ctx.emit_error(LoweringError::internal(
                                        format!(
                                            "method '{}' parent is not a protocol",
                                            method_name
                                        ),
                                        Some(expr.span.clone()),
                                    ));
                                    return Value::Immediate(Immediate::error());
                                }
                            } else {
                                ctx.emit_error(LoweringError::internal(
                                    format!("method '{}' has no parent", method_name),
                                    Some(expr.span.clone()),
                                ));
                                return Value::Immediate(Immediate::error());
                            }
                        } else {
                            // Regular direct method call
                            let func_name = qualified_name_for_symbol(ctx, &sym);
                            let type_args = get_ordered_type_args(ctx, &sym);
                            let mir_callee = if type_args.is_empty() {
                                Callee::direct(func_name)
                            } else {
                                Callee::direct_generic(func_name, type_args)
                            };
                            ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
                        }
                    }
                    None => {
                        ctx.emit_error(LoweringError::internal(
                            format!("method symbol not found for '{}'", method_name),
                            Some(expr.span.clone()),
                        ));
                        return Value::Immediate(Immediate::error());
                    }
                }
            } else {
                ctx.emit_error(LoweringError::internal(
                    format!("no method candidates for '{}'", method_name),
                    Some(expr.span.clone()),
                ));
                return Value::Immediate(Immediate::error());
            }
        }

        ExprKind::TypeRef(symbol_id) => {
            // Calling a type = initializer call
            // Initializers have signature: func Type.init(self: &var Type, params...) -> ()
            // So we need to:
            // 1. Allocate space for the struct (result_place already exists)
            // 2. Take a mutable reference to it
            // 3. Pass the reference as the first argument
            // 4. Call the init (which returns unit)
            // 5. Return the struct value

            let symbol = ctx.model.query(SymbolFor { id: *symbol_id });
            match symbol {
                Some(sym) => {
                    // Build the init function name
                    let mut name_parts = Vec::new();
                    collect_symbol_name_parts(&sym, &mut name_parts);
                    name_parts.push("init".to_string());

                    let init_name = ctx
                        .mir
                        .intern_name(kestrel_execution_graph::QualifiedNameData::new(name_parts));

                    // Create a mutable reference to the result place
                    // The ref type is &var T where T is the struct type
                    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
                    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                    let self_ref_place = Place::local(self_ref_local);

                    // Emit: %self_ref = ref var %result
                    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));

                    // Try to find the initializer symbol to get its CallableBehavior
                    // Look for an "init" child of the type symbol
                    let init_beh = sym
                        .metadata()
                        .children()
                        .iter()
                        .find(|child| {
                            child.metadata().kind() == KestrelSymbolKind::Initializer
                                && child.metadata().name().value == "init"
                        })
                        .and_then(|init_sym| {
                            init_sym.metadata().get_behavior::<CallableBehavior>()
                        });

                    // Build call args: self_ref first (always MutRef), then user args with their modes
                    let mut call_args = vec![CallArg::mutating(Value::Place(self_ref_place))];

                    // Add user-provided arguments with their access modes
                    if let Some(beh) = &init_beh {
                        let params = beh.parameters();
                        for (i, value) in arg_values.into_iter().enumerate() {
                            if let Some(param) = params.get(i) {
                                // Get the argument type for Copy vs Move determination
                                if let Some(arg_ty) = arg_types.get(i).copied() {
                                    let mode =
                                        access_mode_to_passing_mode(param.access_mode(), arg_ty);
                                    call_args.push(CallArg::new(value, mode));
                                } else {
                                    // Fallback: default to Copy for Consuming
                                    let mode = match param.access_mode() {
                                        ParameterAccessMode::Borrow => PassingMode::Ref,
                                        ParameterAccessMode::Mutating => PassingMode::MutRef,
                                        ParameterAccessMode::Consuming => PassingMode::Copy,
                                    };
                                    call_args.push(CallArg::new(value, mode));
                                }
                            } else {
                                call_args.push(CallArg::borrow(value));
                            }
                        }
                    } else {
                        call_args.extend(arg_values.into_iter().map(CallArg::borrow));
                    }

                    // Create a temp for the unit return value of init (we discard it)
                    let unit_ty = ctx.mir.ty_unit();
                    let unit_local = ctx.create_temp("init_ret", unit_ty);
                    let unit_place = Place::local(unit_local);

                    // Call the init function
                    let type_args = get_ordered_type_args(ctx, &sym);
                    let mir_callee = if type_args.is_empty() {
                        Callee::direct(init_name)
                    } else {
                        Callee::direct_generic(init_name, type_args)
                    };
                    mark_moved_args(ctx, &call_args);
                    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

                    // result_place now contains the initialized struct
                    // (init wrote to it via the self_ref)
                }
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!(
                            "type symbol not found for initializer call: {:?}",
                            symbol_id
                        ),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                }
            }
        }

        ExprKind::LocalRef(local_id) => {
            // Indirect call through a local variable (closure)
            let mir_local = ctx.get_local_unwrap(*local_id);
            let callee_place = Place::local(mir_local);

            // Build call args with proper reference creation for indirect calls
            let call_args = build_indirect_call_args(ctx, arg_values, &arg_types, &callee.ty);
            mark_moved_args(ctx, &call_args);

            // Closures are "thick" callables
            ctx.emit_call_with_modes(result_place.clone(), Callee::Thick(callee_place), call_args);
        }

        _ => {
            // Other callee expressions - try to lower as a place for indirect call
            let callee_value = lower_expression(ctx, callee);
            match callee_value {
                Value::Place(callee_place) => {
                    // Build call args with proper reference creation for indirect calls
                    let call_args =
                        build_indirect_call_args(ctx, arg_values, &arg_types, &callee.ty);
                    mark_moved_args(ctx, &call_args);

                    // Determine if this is a thick (closure) or thin function call
                    // by checking the callee's type
                    let is_thick = matches!(
                        callee.ty.kind(),
                        TyKind::Function { .. } | TyKind::UnresolvedFunction { .. }
                    );
                    let mir_callee = if is_thick {
                        Callee::Thick(callee_place)
                    } else {
                        Callee::Thin(callee_place)
                    };
                    ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
                }
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::unsupported_expr(
                        "indirect call on immediate value",
                        expr.span.clone(),
                    ));
                    return Value::Immediate(Immediate::error());
                }
            }
        }
    }

    Value::Place(result_place)
}

/// Collect name segments from a symbol (helper for TypeRef init calls).
fn collect_symbol_name_parts(
    symbol: &std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
    parts: &mut Vec<String>,
) {
    use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

    // First, collect parent segments
    if let Some(parent) = symbol.metadata().parent() {
        collect_symbol_name_parts(&parent, parts);
    }

    // Then add this symbol's name
    let kind = symbol.metadata().kind();
    let name_value = &symbol.metadata().name().value;

    // Skip root
    if name_value == "<root>" {
        return;
    }

    match kind {
        KestrelSymbolKind::SourceFile => {}
        KestrelSymbolKind::Module
        | KestrelSymbolKind::Struct
        | KestrelSymbolKind::Enum
        | KestrelSymbolKind::Protocol
        | KestrelSymbolKind::TypeAlias
        | KestrelSymbolKind::Extension => {
            parts.push(name_value.clone());
        }
        _ => {}
    }
}

/// Lower an if expression.
///
/// Handles both regular `if` and `if let` conditions, including condition chains.
///
/// # If-Let Lowering Strategy
///
/// An `if let pattern = expr { then } else { else }` is lowered by:
/// 1. Compiling the pattern into a decision tree with 2 arms:
///    - Arm 0: the pattern → then block
///    - Arm 1: wildcard → else block
/// 2. The decision tree handles all the pattern matching logic
///
/// For condition chains like `if let .Some(x) = a, x > 0 { ... }`:
/// 1. First evaluate all let-conditions, checking patterns
/// 2. If all patterns match, evaluate boolean conditions
/// 3. If all pass, execute then-branch; otherwise else-branch
///
/// # Branch Merging for Conditional Deinit
///
/// When a variable from a parent scope is moved in one branch but not the other,
/// we need to emit conditional deinit (DeinitIf) at scope exit. This is handled by:
/// 1. Capturing a snapshot of deinit statuses before entering branches
/// 2. Lowering each branch and capturing post-branch statuses
/// 3. Computing which variables diverged and creating deinit flags
/// 4. Emitting SetDeinitFlag statements at the end of each branch
/// 5. Updating parent scope statuses to MaybeMoved for divergent variables
fn lower_if(
    ctx: &mut LoweringContext,
    conditions: &[IfCondition],
    then_branch: &[kestrel_semantic_tree::stmt::Statement],
    then_value: &Option<Box<Expression>>,
    else_branch: &Option<ElseBranch>,
    expr: &Expression,
) -> Value {
    // Get result type
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("if_result", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if the result type needs deinit
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Create the join block where both branches converge
    let join_block = ctx.create_block();

    // Create the else block
    let else_block_id = ctx.create_block();

    // Lower the condition chain
    // This will emit all pattern tests and boolean conditions, eventually
    // jumping to either then_block_start or else_block
    let then_block_start = ctx.create_block();

    // Capture snapshot of parent scope deinit statuses BEFORE entering branches
    let before_statuses = ctx.snapshot_parent_deinit_statuses();

    lower_condition_chain(ctx, conditions, 0, then_block_start, else_block_id);

    // === Lower then branch ===
    ctx.set_current_block(then_block_start);
    ctx.enter_scope();
    for stmt in then_branch {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Evaluate then value before capturing statuses (might cause moves)
    let _then_result_value = if !ctx.is_block_terminated() {
        if let Some(value_expr) = then_value {
            let result = lower_expression(ctx, value_expr);
            if !ctx.is_block_terminated() {
                ctx.emit_assign_value(result_place.clone(), result);
            }
            true
        } else {
            // No then value - assign unit
            ctx.emit_imm(result_place.clone(), Immediate::unit());
            true
        }
    } else {
        false
    };

    // Capture then branch's view of parent scope statuses (before exiting branch scope)
    let then_statuses = ctx.snapshot_parent_deinit_statuses();
    let then_final_terminated = ctx.is_block_terminated();

    // Get the then scope info for branch-local deinits
    let then_scope = ctx.exit_scope_no_emit();

    // Track the block where we need to emit flag settings for then branch
    let then_final_block = ctx.current_block();

    // === Lower else branch ===
    // IMPORTANT: Restore parent scope statuses to pre-then-branch state.
    // The then branch may have marked variables as Moved in the parent scope,
    // but the else branch should see them as still Valid (the "before" state).
    ctx.restore_deinit_statuses(&before_statuses);

    ctx.set_current_block(else_block_id);
    ctx.enter_scope();

    // Track whether else branch is terminated
    let else_statuses;
    let else_scope;
    let else_final_block;
    let else_final_terminated;

    match else_branch {
        Some(ElseBranch::Block { statements, value }) => {
            for stmt in statements {
                lower_statement(ctx, stmt);
                if ctx.is_block_terminated() {
                    break;
                }
            }

            if !ctx.is_block_terminated() {
                if let Some(value_expr) = value {
                    let else_result = lower_expression(ctx, value_expr);
                    if !ctx.is_block_terminated() {
                        ctx.emit_assign_value(result_place.clone(), else_result);
                    }
                } else {
                    ctx.emit_imm(result_place.clone(), Immediate::unit());
                }
            }

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        }

        Some(ElseBranch::ElseIf(else_if_expr)) => {
            // ElseIf is a nested if expression which will handle its own scopes
            let else_result = lower_expression(ctx, else_if_expr);

            if !ctx.is_block_terminated() {
                ctx.emit_assign_value(result_place.clone(), else_result);
            }

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        }

        None => {
            // No else branch - result is unit
            ctx.emit_imm(result_place.clone(), Immediate::unit());

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        }
    }

    // === Compute branch merge for parent-scope locals ===
    // Only do this if at least one branch is not terminated
    // (if both are terminated, no code after the if will run)
    let merge_result = if !then_final_terminated || !else_final_terminated {
        Some(ctx.compute_branch_merge(&before_statuses, &then_statuses, &else_statuses))
    } else {
        None
    };

    // === Emit flag settings and deinits for then branch ===
    if let Some(then_block) = then_final_block {
        if !then_final_terminated {
            ctx.set_current_block(then_block);

            // Emit flag settings for divergent parent-scope locals
            if let Some(ref merge) = merge_result {
                for &flag in &merge.then_flag_false {
                    ctx.set_deinit_flag(flag, false);
                }
                for &flag in &merge.then_flag_true {
                    ctx.set_deinit_flag(flag, true);
                }
            }

            // Emit deinits for branch-local variables
            if let Some(ref scope) = then_scope {
                ctx.emit_scope_deinits(scope);
            }

            ctx.emit_jump(join_block);
        }
    }

    // === Emit flag settings and deinits for else branch ===
    if let Some(else_block) = else_final_block {
        if !else_final_terminated {
            ctx.set_current_block(else_block);

            // Emit flag settings for divergent parent-scope locals
            if let Some(ref merge) = merge_result {
                for &flag in &merge.else_flag_false {
                    ctx.set_deinit_flag(flag, false);
                }
                for &flag in &merge.else_flag_true {
                    ctx.set_deinit_flag(flag, true);
                }
            }

            // Emit deinits for branch-local variables
            if let Some(ref scope) = else_scope {
                ctx.emit_scope_deinits(scope);
            }

            ctx.emit_jump(join_block);
        }
    }

    // === Apply status updates to parent scopes ===
    if let Some(merge) = merge_result {
        ctx.apply_merge_updates(merge.updates);
    }

    // Continue with join block
    ctx.set_current_block(join_block);

    Value::Place(result_place)
}

/// Lower a chain of conditions for if/if-let.
///
/// This recursively processes each condition:
/// - For `Expr` conditions: emit a boolean branch
/// - For `Let` conditions: emit pattern matching via decision tree
///
/// If all conditions pass, jumps to `then_block`.
/// If any condition fails, jumps to `else_block`.
fn lower_condition_chain(
    ctx: &mut LoweringContext,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Base case: all conditions processed, jump to then block
    if index >= conditions.len() {
        ctx.emit_jump(then_block);
        return;
    }

    match &conditions[index] {
        IfCondition::Expr(condition_expr) => {
            // Boolean condition: emit branch
            let cond_value = lower_expression(ctx, condition_expr);

            // If this is the last condition, branch directly to then/else
            if index == conditions.len() - 1 {
                ctx.emit_branch(cond_value, then_block, else_block);
            } else {
                // More conditions to check: create a block for the next condition
                let next_block = ctx.create_block();
                ctx.emit_branch(cond_value, next_block, else_block);
                ctx.set_current_block(next_block);
                lower_condition_chain(ctx, conditions, index + 1, then_block, else_block);
            }
        }

        IfCondition::Let { pattern, value, .. } => {
            // If-let condition: use pattern matching
            lower_if_let_condition(
                ctx, pattern, value, conditions, index, then_block, else_block,
            );
        }
    }
}

/// Lower a single if-let condition using decision tree compilation.
///
/// This compiles the pattern into a decision tree and emits:
/// - Pattern match tests
/// - Bindings (which are visible in subsequent conditions and the then branch)
/// - Branches to either the next condition or else block
fn lower_if_let_condition(
    ctx: &mut LoweringContext,
    pattern: &kestrel_semantic_tree::pattern::Pattern,
    scrutinee_expr: &Expression,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::compile;

    // Lower the scrutinee
    let scrutinee_value = lower_expression(ctx, scrutinee_expr);

    // We need the scrutinee in a place for pattern matching
    let scrutinee_place = match scrutinee_value {
        Value::Place(p) => p,
        Value::Immediate(imm) => {
            // Store the immediate in a temporary
            let scrutinee_ty = lower_type(ctx, &scrutinee_expr.ty);
            let scrutinee_local = ctx.create_temp("scrutinee", scrutinee_ty);
            let place = Place::local(scrutinee_local);
            ctx.emit_assign(place.clone(), Rvalue::Use(imm));
            place
        }
    };

    // Compile the pattern into a decision tree
    // For if-let, we have just one pattern (no guards in if-let itself)
    let patterns = vec![pattern.clone()];
    let has_guards = vec![false];
    let decision_tree = compile(&patterns, &scrutinee_expr.ty, &has_guards);

    // Emit the decision tree for if-let
    // Success means pattern matched → continue to next condition or then block
    // Failure means pattern didn't match → go to else block
    emit_if_let_decision_tree(
        ctx,
        &decision_tree,
        &scrutinee_place,
        conditions,
        index,
        then_block,
        else_block,
    );
}

/// Emit MIR for an if-let decision tree.
///
/// Unlike match expressions, if-let has only one pattern (plus implicit wildcard),
/// so the decision tree emission is simpler:
/// - Success: emit bindings and continue to next condition
/// - Failure: jump to else block
/// - Switch: test constructors and recurse
fn emit_if_let_decision_tree(
    ctx: &mut LoweringContext,
    tree: &kestrel_semantic_pattern_matching::DecisionTree,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::DecisionTree;

    match tree {
        DecisionTree::Success { bindings, .. } => {
            // Pattern matched! Emit bindings and continue to next condition
            crate::match_lowering::emit_bindings(ctx, bindings, scrutinee);

            // Continue with the rest of the condition chain
            lower_condition_chain(ctx, conditions, index + 1, then_block, else_block);
        }

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_if_let_switch(
                ctx, path, ty, cases, default, scrutinee, conditions, index, then_block, else_block,
            );
        }

        DecisionTree::Guard { .. } => {
            // Guards shouldn't appear in if-let (guards are a match-specific feature)
            // If they do, treat as failure
            ctx.emit_jump(else_block);
        }

        DecisionTree::Failure => {
            // Pattern didn't match, go to else block
            ctx.emit_jump(else_block);
        }
    }
}

/// Emit MIR for a switch node in an if-let decision tree.
fn emit_if_let_switch(
    ctx: &mut LoweringContext,
    path: &kestrel_semantic_pattern_matching::AccessPath,
    ty: &kestrel_semantic_tree::ty::Ty,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_tree::ty::TyKind;

    // Get the place to switch on
    let switch_place = crate::match_lowering::apply_path(scrutinee, path);

    match ty.kind() {
        TyKind::Bool => {
            emit_if_let_bool_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        }

        TyKind::Enum { .. } => {
            emit_if_let_enum_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        }

        TyKind::Int(_) => {
            emit_if_let_int_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        }

        TyKind::String => {
            emit_if_let_string_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        }

        TyKind::Tuple(_) | TyKind::Struct { .. } => {
            // Single constructor types - just recurse into the case
            if let Some((_, subtree)) = cases.first() {
                emit_if_let_decision_tree(
                    ctx, subtree, scrutinee, conditions, index, then_block, else_block,
                );
            } else if let Some(default_tree) = default {
                emit_if_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    then_block,
                    else_block,
                );
            } else {
                ctx.emit_jump(else_block);
            }
        }

        _ => {
            // For other types, try the default or first case
            if let Some(default_tree) = default {
                emit_if_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    then_block,
                    else_block,
                );
            } else if let Some((_, tree)) = cases.first() {
                emit_if_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, then_block, else_block,
                );
            } else {
                ctx.emit_jump(else_block);
            }
        }
    }
}

/// Emit boolean switch for if-let.
fn emit_if_let_bool_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Find true and false cases
    let true_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::True))
        .map(|(_, t)| t);
    let false_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::False))
        .map(|(_, t)| t);

    // Create blocks for each case
    let true_block = ctx.create_block();
    let false_block = ctx.create_block();

    // Emit branch
    ctx.emit_branch(Value::Place(switch_place.clone()), true_block, false_block);

    // Emit true case
    ctx.set_current_block(true_block);
    if let Some(tree) = true_tree {
        emit_if_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, then_block, else_block,
        );
    } else if let Some(default_tree) = default {
        emit_if_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            then_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }

    // Emit false case
    ctx.set_current_block(false_block);
    if let Some(tree) = false_tree {
        emit_if_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, then_block, else_block,
        );
    } else if let Some(default_tree) = default {
        emit_if_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            then_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit enum switch for if-let.
fn emit_if_let_enum_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Build switch cases: (variant_name, block)
    let mut switch_cases = Vec::with_capacity(cases.len() + 1);
    let mut case_trees = Vec::with_capacity(cases.len());

    for (ctor, tree) in cases {
        if let Constructor::Variant { name, .. } = ctor {
            let case_block = ctx.create_block();
            switch_cases.push((name.clone(), case_block));
            case_trees.push((case_block, tree));
        }
    }

    // Add default case (for unmatched variants → else block)
    // This is the key difference from match: unmatched variants go to else_block
    let default_case_block = ctx.create_block();
    switch_cases.push(("_".to_string(), default_case_block));

    // Emit the switch terminator
    ctx.emit_switch(switch_place.clone(), switch_cases);

    // Emit each matched variant's body
    for (block, tree) in case_trees {
        ctx.set_current_block(block);
        emit_if_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, then_block, else_block,
        );
    }

    // Emit default case: if there's a default tree use it, otherwise go to else
    ctx.set_current_block(default_case_block);
    if let Some(default_tree) = default {
        emit_if_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            then_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit integer comparison chain for if-let.
fn emit_if_let_int_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_if_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        } else {
            ctx.emit_jump(else_block);
        }
        return;
    }

    // Build a chain of comparisons
    for (ctor, tree) in cases.iter() {
        match ctor {
            Constructor::IntLiteral(value) => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Compare: switch_place == value
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::Eq,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(Immediate::i64(*value)),
                    },
                );

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_if_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, then_block, else_block,
                );

                // Continue with next comparison
                ctx.set_current_block(next_block);
            }

            Constructor::IntRange { start, end } => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Range check: start <= value && value <= end
                let cmp1_ty = ctx.mir.ty_bool();
                let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                let cmp1_place = Place::local(cmp1_local);
                ctx.emit_assign(
                    cmp1_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Immediate(Immediate::i64(*start)),
                        rhs: Value::Place(switch_place.clone()),
                    },
                );

                let cmp2_ty = ctx.mir.ty_bool();
                let cmp2_local = ctx.create_temp("cmp_hi", cmp2_ty);
                let cmp2_place = Place::local(cmp2_local);
                ctx.emit_assign(
                    cmp2_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(Immediate::i64(*end)),
                    },
                );

                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp_range", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::BoolAnd,
                        lhs: Value::Place(cmp1_place),
                        rhs: Value::Place(cmp2_place),
                    },
                );

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                ctx.set_current_block(match_block);
                emit_if_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, then_block, else_block,
                );

                ctx.set_current_block(next_block);
            }

            _ => {
                // Skip unsupported constructors
                continue;
            }
        }
    }

    // After all cases, check default or go to else
    if let Some(default_tree) = default {
        emit_if_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            then_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit string comparison chain for if-let.
fn emit_if_let_string_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_if_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        } else {
            ctx.emit_jump(else_block);
        }
        return;
    }

    // Build a chain of string comparisons
    for (ctor, tree) in cases {
        if let Constructor::StringLiteral(value) = ctor {
            let match_block = ctx.create_block();
            let next_block = ctx.create_block();

            // Compare: switch_place == value
            let cmp_ty = ctx.mir.ty_bool();
            let cmp_local = ctx.create_temp("cmp", cmp_ty);
            let cmp_place = Place::local(cmp_local);
            ctx.emit_assign(
                cmp_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::Eq,
                    lhs: Value::Place(switch_place.clone()),
                    rhs: Value::Immediate(Immediate::string(value.clone())),
                },
            );

            ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

            ctx.set_current_block(match_block);
            emit_if_let_decision_tree(
                ctx, tree, scrutinee, conditions, index, then_block, else_block,
            );

            ctx.set_current_block(next_block);
        }
    }

    // After all cases, check default or go to else
    if let Some(default_tree) = default {
        emit_if_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            then_block,
            else_block,
        );
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Lower a while loop.
fn lower_while(
    ctx: &mut LoweringContext,
    loop_id: kestrel_semantic_tree::expr::LoopId,
    condition: &Expression,
    body: &[kestrel_semantic_tree::stmt::Statement],
    _expr: &Expression,
) -> Value {
    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(loop_id, header_block, exit_block);

    // Jump to header
    ctx.emit_jump(header_block);

    // Header: check condition
    ctx.set_current_block(header_block);
    let cond_value = lower_expression(ctx, condition);
    ctx.emit_branch(cond_value, body_block, exit_block);

    // Body - enter a new scope for the loop body
    ctx.set_current_block(body_block);
    ctx.enter_scope();
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Exit the loop body scope (emits deinits) before jumping back to header
    if !ctx.is_block_terminated() {
        ctx.exit_scope();
        ctx.emit_jump(header_block);
    } else {
        // Block was terminated (by break/return/continue) - scope cleanup happens there
        ctx.exit_scope_no_emit();
    }

    // Pop loop info
    ctx.pop_loop();

    // Continue with exit block
    ctx.set_current_block(exit_block);

    // While loops always return unit
    Value::Immediate(Immediate::unit())
}

/// Lower an infinite loop (`loop { ... }`).
fn lower_loop(
    ctx: &mut LoweringContext,
    loop_id: kestrel_semantic_tree::expr::LoopId,
    body: &[kestrel_semantic_tree::stmt::Statement],
    _expr: &Expression,
) -> Value {
    // Create blocks: header (body entry) and exit
    // For infinite loops, header IS the body - no condition check
    let header_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(loop_id, header_block, exit_block);

    // Jump to header (body entry)
    ctx.emit_jump(header_block);

    // Body - enter a new scope for the loop body
    ctx.set_current_block(header_block);
    ctx.enter_scope();
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Exit the loop body scope (emits deinits) before jumping back to header
    if !ctx.is_block_terminated() {
        ctx.exit_scope();
        ctx.emit_jump(header_block);
    } else {
        // Block was terminated (by break/return/continue) - scope cleanup happens there
        ctx.exit_scope_no_emit();
    }

    // Pop loop info
    ctx.pop_loop();

    // Continue with exit block (reached via break)
    ctx.set_current_block(exit_block);

    // Loop expressions return unit
    Value::Immediate(Immediate::unit())
}

/// Lower a while-let loop.
///
/// `while let pattern = expr { body }` is like a while loop where:
/// - The condition is a pattern match
/// - If the pattern matches, bindings are available in the body
/// - If the pattern doesn't match, the loop exits
///
/// # MIR Structure
///
/// ```text
/// entry_block:
///     jump header_block
///
/// header_block:
///     <evaluate scrutinee>
///     <emit pattern matching decision tree>
///     -> match: jump to body_block (with bindings)
///     -> no match: jump to exit_block
///
/// body_block:
///     <bindings in scope>
///     <lower body statements>
///     jump header_block
///
/// exit_block:
///     // loop finished
/// ```
fn lower_while_let(
    ctx: &mut LoweringContext,
    loop_id: kestrel_semantic_tree::expr::LoopId,
    conditions: &[IfCondition],
    body: &[kestrel_semantic_tree::stmt::Statement],
) -> Value {
    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    // Note: continue should jump to header_block to re-evaluate the condition
    ctx.push_loop(loop_id, header_block, exit_block);

    // Jump to header
    ctx.emit_jump(header_block);

    // Header: evaluate conditions
    // This is where we emit the pattern matching logic
    // If all conditions pass → body_block
    // If any condition fails → exit_block
    ctx.set_current_block(header_block);
    lower_while_let_condition_chain(ctx, conditions, 0, body_block, exit_block);

    // Body - enter a new scope for the loop body
    ctx.set_current_block(body_block);
    ctx.enter_scope();
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Exit the loop body scope (emits deinits) before jumping back to header
    if !ctx.is_block_terminated() {
        ctx.exit_scope();
        ctx.emit_jump(header_block);
    } else {
        // Block was terminated (by break/return/continue) - scope cleanup happens there
        ctx.exit_scope_no_emit();
    }

    // Pop loop info
    ctx.pop_loop();

    // Continue with exit block
    ctx.set_current_block(exit_block);

    // While-let loops always return unit
    Value::Immediate(Immediate::unit())
}

/// Lower a chain of conditions for while-let.
///
/// Similar to if-let condition chains:
/// - For `Expr` conditions: emit a boolean branch
/// - For `Let` conditions: emit pattern matching via decision tree
///
/// If all conditions pass, jumps to `body_block`.
/// If any condition fails, jumps to `exit_block`.
fn lower_while_let_condition_chain(
    ctx: &mut LoweringContext,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    // Base case: all conditions processed, jump to body block
    if index >= conditions.len() {
        ctx.emit_jump(body_block);
        return;
    }

    match &conditions[index] {
        IfCondition::Expr(condition_expr) => {
            // Boolean condition: emit branch
            let cond_value = lower_expression(ctx, condition_expr);

            // If this is the last condition, branch directly to body/exit
            if index == conditions.len() - 1 {
                ctx.emit_branch(cond_value, body_block, exit_block);
            } else {
                // More conditions to check: create a block for the next condition
                let next_block = ctx.create_block();
                ctx.emit_branch(cond_value, next_block, exit_block);
                ctx.set_current_block(next_block);
                lower_while_let_condition_chain(ctx, conditions, index + 1, body_block, exit_block);
            }
        }

        IfCondition::Let { pattern, value, .. } => {
            // While-let condition: use pattern matching
            lower_while_let_pattern_condition(
                ctx, pattern, value, conditions, index, body_block, exit_block,
            );
        }
    }
}

/// Lower a single while-let pattern condition using decision tree compilation.
fn lower_while_let_pattern_condition(
    ctx: &mut LoweringContext,
    pattern: &kestrel_semantic_tree::pattern::Pattern,
    scrutinee_expr: &Expression,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::compile;

    // Lower the scrutinee
    let scrutinee_value = lower_expression(ctx, scrutinee_expr);

    // We need the scrutinee in a place for pattern matching
    let scrutinee_place = match scrutinee_value {
        Value::Place(p) => p,
        Value::Immediate(imm) => {
            // Store the immediate in a temporary
            let scrutinee_ty = lower_type(ctx, &scrutinee_expr.ty);
            let scrutinee_local = ctx.create_temp("scrutinee", scrutinee_ty);
            let place = Place::local(scrutinee_local);
            ctx.emit_assign(place.clone(), Rvalue::Use(imm));
            place
        }
    };

    // Compile the pattern into a decision tree
    let patterns = vec![pattern.clone()];
    let has_guards = vec![false];
    let decision_tree = compile(&patterns, &scrutinee_expr.ty, &has_guards);

    // Emit the decision tree for while-let
    emit_while_let_decision_tree(
        ctx,
        &decision_tree,
        &scrutinee_place,
        conditions,
        index,
        body_block,
        exit_block,
    );
}

/// Emit MIR for a while-let decision tree.
///
/// Similar to if-let:
/// - Success: emit bindings and continue to next condition (or body_block)
/// - Failure: jump to exit_block (loop terminates)
fn emit_while_let_decision_tree(
    ctx: &mut LoweringContext,
    tree: &kestrel_semantic_pattern_matching::DecisionTree,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::DecisionTree;

    match tree {
        DecisionTree::Success { bindings, .. } => {
            // Pattern matched! Emit bindings and continue to next condition
            crate::match_lowering::emit_bindings(ctx, bindings, scrutinee);

            // Continue with the rest of the condition chain
            lower_while_let_condition_chain(ctx, conditions, index + 1, body_block, exit_block);
        }

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_while_let_switch(
                ctx, path, ty, cases, default, scrutinee, conditions, index, body_block, exit_block,
            );
        }

        DecisionTree::Guard { .. } => {
            // Guards shouldn't appear in while-let patterns
            ctx.emit_jump(exit_block);
        }

        DecisionTree::Failure => {
            // Pattern didn't match, exit the loop
            ctx.emit_jump(exit_block);
        }
    }
}

/// Emit MIR for a switch node in a while-let decision tree.
fn emit_while_let_switch(
    ctx: &mut LoweringContext,
    path: &kestrel_semantic_pattern_matching::AccessPath,
    ty: &kestrel_semantic_tree::ty::Ty,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_tree::ty::TyKind;

    // Get the place to switch on
    let switch_place = crate::match_lowering::apply_path(scrutinee, path);

    match ty.kind() {
        TyKind::Bool => {
            emit_while_let_bool_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        }

        TyKind::Enum { .. } => {
            emit_while_let_enum_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        }

        TyKind::Int(_) => {
            emit_while_let_int_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        }

        TyKind::String => {
            emit_while_let_string_switch(
                ctx,
                &switch_place,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        }

        TyKind::Tuple(_) | TyKind::Struct { .. } => {
            // Single constructor types - just recurse into the case
            if let Some((_, subtree)) = cases.first() {
                emit_while_let_decision_tree(
                    ctx, subtree, scrutinee, conditions, index, body_block, exit_block,
                );
            } else if let Some(default_tree) = default {
                emit_while_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    body_block,
                    exit_block,
                );
            } else {
                ctx.emit_jump(exit_block);
            }
        }

        _ => {
            // For other types, try the default or first case
            if let Some(default_tree) = default {
                emit_while_let_decision_tree(
                    ctx,
                    default_tree,
                    scrutinee,
                    conditions,
                    index,
                    body_block,
                    exit_block,
                );
            } else if let Some((_, tree)) = cases.first() {
                emit_while_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, body_block, exit_block,
                );
            } else {
                ctx.emit_jump(exit_block);
            }
        }
    }
}

/// Emit boolean switch for while-let.
fn emit_while_let_bool_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Find true and false cases
    let true_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::True))
        .map(|(_, t)| t);
    let false_tree = cases
        .iter()
        .find(|(c, _)| matches!(c, Constructor::False))
        .map(|(_, t)| t);

    // Create blocks for each case
    let true_block = ctx.create_block();
    let false_block = ctx.create_block();

    // Emit branch
    ctx.emit_branch(Value::Place(switch_place.clone()), true_block, false_block);

    // Emit true case
    ctx.set_current_block(true_block);
    if let Some(tree) = true_tree {
        emit_while_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, body_block, exit_block,
        );
    } else if let Some(default_tree) = default {
        emit_while_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
        );
    } else {
        ctx.emit_jump(exit_block);
    }

    // Emit false case
    ctx.set_current_block(false_block);
    if let Some(tree) = false_tree {
        emit_while_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, body_block, exit_block,
        );
    } else if let Some(default_tree) = default {
        emit_while_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
        );
    } else {
        ctx.emit_jump(exit_block);
    }
}

/// Emit enum switch for while-let.
fn emit_while_let_enum_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Build switch cases: (variant_name, block)
    let mut switch_cases = Vec::with_capacity(cases.len() + 1);
    let mut case_trees = Vec::with_capacity(cases.len());

    for (ctor, tree) in cases {
        if let Constructor::Variant { name, .. } = ctor {
            let case_block = ctx.create_block();
            switch_cases.push((name.clone(), case_block));
            case_trees.push((case_block, tree));
        }
    }

    // Add default case (for unmatched variants → exit_block)
    let default_case_block = ctx.create_block();
    switch_cases.push(("_".to_string(), default_case_block));

    // Emit the switch terminator
    ctx.emit_switch(switch_place.clone(), switch_cases);

    // Emit each matched variant's body
    for (block, tree) in case_trees {
        ctx.set_current_block(block);
        emit_while_let_decision_tree(
            ctx, tree, scrutinee, conditions, index, body_block, exit_block,
        );
    }

    // Emit default case: if there's a default tree use it, otherwise exit loop
    ctx.set_current_block(default_case_block);
    if let Some(default_tree) = default {
        emit_while_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
        );
    } else {
        ctx.emit_jump(exit_block);
    }
}

/// Emit integer comparison chain for while-let.
fn emit_while_let_int_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_while_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        } else {
            ctx.emit_jump(exit_block);
        }
        return;
    }

    // Build a chain of comparisons
    for (ctor, tree) in cases.iter() {
        match ctor {
            Constructor::IntLiteral(value) => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Compare: switch_place == value
                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::Eq,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(Immediate::i64(*value)),
                    },
                );

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                // Emit match body
                ctx.set_current_block(match_block);
                emit_while_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, body_block, exit_block,
                );

                // Continue with next comparison
                ctx.set_current_block(next_block);
            }

            Constructor::IntRange { start, end } => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Range check: start <= value && value <= end
                let cmp1_ty = ctx.mir.ty_bool();
                let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                let cmp1_place = Place::local(cmp1_local);
                ctx.emit_assign(
                    cmp1_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Immediate(Immediate::i64(*start)),
                        rhs: Value::Place(switch_place.clone()),
                    },
                );

                let cmp2_ty = ctx.mir.ty_bool();
                let cmp2_local = ctx.create_temp("cmp_hi", cmp2_ty);
                let cmp2_place = Place::local(cmp2_local);
                ctx.emit_assign(
                    cmp2_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::LeSigned,
                        lhs: Value::Place(switch_place.clone()),
                        rhs: Value::Immediate(Immediate::i64(*end)),
                    },
                );

                let cmp_ty = ctx.mir.ty_bool();
                let cmp_local = ctx.create_temp("cmp_range", cmp_ty);
                let cmp_place = Place::local(cmp_local);
                ctx.emit_assign(
                    cmp_place.clone(),
                    Rvalue::BinaryOp {
                        op: BinOp::BoolAnd,
                        lhs: Value::Place(cmp1_place),
                        rhs: Value::Place(cmp2_place),
                    },
                );

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                ctx.set_current_block(match_block);
                emit_while_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, body_block, exit_block,
                );

                ctx.set_current_block(next_block);
            }

            _ => {
                // Skip unsupported constructors
                continue;
            }
        }
    }

    // After all cases, check default or exit loop
    if let Some(default_tree) = default {
        emit_while_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
        );
    } else {
        ctx.emit_jump(exit_block);
    }
}

/// Emit string comparison chain for while-let.
fn emit_while_let_string_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(
        kestrel_semantic_pattern_matching::Constructor,
        kestrel_semantic_pattern_matching::DecisionTree,
    )],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    body_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    exit_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_execution_graph::BinOp;
    use kestrel_semantic_pattern_matching::Constructor;

    // If no cases, check default
    if cases.is_empty() {
        if let Some(default_tree) = default {
            emit_while_let_decision_tree(
                ctx,
                default_tree,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        } else {
            ctx.emit_jump(exit_block);
        }
        return;
    }

    // Build a chain of string comparisons
    for (ctor, tree) in cases {
        if let Constructor::StringLiteral(value) = ctor {
            let match_block = ctx.create_block();
            let next_block = ctx.create_block();

            // Compare: switch_place == value
            let cmp_ty = ctx.mir.ty_bool();
            let cmp_local = ctx.create_temp("cmp", cmp_ty);
            let cmp_place = Place::local(cmp_local);
            ctx.emit_assign(
                cmp_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::Eq,
                    lhs: Value::Place(switch_place.clone()),
                    rhs: Value::Immediate(Immediate::string(value.clone())),
                },
            );

            ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

            ctx.set_current_block(match_block);
            emit_while_let_decision_tree(
                ctx, tree, scrutinee, conditions, index, body_block, exit_block,
            );

            ctx.set_current_block(next_block);
        }
    }

    // After all cases, check default or exit loop
    if let Some(default_tree) = default {
        emit_while_let_decision_tree(
            ctx,
            default_tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
        );
    } else {
        ctx.emit_jump(exit_block);
    }
}
