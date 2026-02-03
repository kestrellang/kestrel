//! Expression lowering - converts semantic expressions to MIR values.
//!
//! This is the core of the lowering pass. Each expression is converted to
//! a MIR Value (either a Place or an Immediate), potentially generating
//! statements and new basic blocks along the way.

use kestrel_execution_graph::{
    BinOp, CallArg, Callee, CastKind, Id, Immediate, Local, MirTy, PassingMode, Place,
    QualifiedNameData, Rvalue, UnOp, Value,
};
use kestrel_semantic_model::{StructFields, SymbolFor};
use kestrel_semantic_tree::behavior::FileConstantBehavior;
use kestrel_semantic_tree::behavior::callable::{
    CallableBehavior, ParameterAccessMode, ReceiverKind,
};
use kestrel_semantic_tree::behavior::executable::ResolvedExecutableBehavior;
use kestrel_semantic_tree::expr::{
    CallArgument, ElseBranch, ExprKind, Expression, IfCondition, InterpolationPart, LiteralValue,
    PrimitiveMethod,
};
use kestrel_semantic_tree::symbol::field::FieldSymbol;
use kestrel_semantic_tree::symbol::getter::GetterSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;
use kestrel_semantic_tree::ty::{Substitutions, Ty, TyKind};
use semantic_tree::symbol::SymbolId;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::format_spec::{parse_format_spec, Alignment, FormatSpec, FormatType};
use crate::lowerer::get_subscript_type_parameters;
use crate::name::qualified_name_for_symbol;
use crate::stmt::lower_statement;
use crate::ty::{lower_type, make_float_immediate, make_int_immediate, make_int_zero_for_mir_ty};

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
        },
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
                },
            }
        },
        None => {
            // Cloneable builtin not registered - std library may not be loaded
            ctx.emit_error(LoweringError::internal(
                "Cloneable builtin protocol not registered",
                None,
            ));
            // Return a dummy value to allow compilation to continue
            return Value::Immediate(Immediate::unit());
        },
    };

    // Create the witness callee: witness_method Cloneable.clone for T
    // clone() has no method-level type parameters
    let callee = Callee::witness(protocol_name, "clone", mir_ty, vec![]);

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
                        // Handle receiver based on ReceiverKind
                        match beh.receiver() {
                            Some(ReceiverKind::Borrowing) | None => {
                                // Immutable borrow of self - use PassingMode::Ref
                                CallArg::borrow(value)
                            },
                            Some(ReceiverKind::Mutating) => {
                                // Mutable borrow of self - use PassingMode::MutRef
                                CallArg::mutating(value)
                            },
                            Some(ReceiverKind::Consuming) => {
                                // Takes ownership of self
                                if let Some(arg_ty) = arg_types.first().copied() {
                                    let mode = if arg_ty.is_copyable() {
                                        PassingMode::Copy
                                    } else {
                                        PassingMode::Move
                                    };
                                    CallArg::new(value, mode)
                                } else {
                                    // Fallback: assume move
                                    CallArg::moving(value)
                                }
                            },
                            Some(ReceiverKind::Initializing) => {
                                // For initializers, self is being constructed - pass as mutable ref
                                CallArg::mutating(value)
                            },
                        }
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
                                    },
                                    ParameterAccessMode::Mutating => {
                                        // Create a mutable reference to the argument
                                        let ref_value = create_ref(ctx, &value, arg_ty, true);
                                        CallArg::new(ref_value, PassingMode::Copy)
                                    },
                                    ParameterAccessMode::Consuming => {
                                        // Pass by value (copy or move)
                                        let mode = if arg_ty.is_copyable() {
                                            PassingMode::Copy
                                        } else {
                                            PassingMode::Move
                                        };
                                        CallArg::new(value, mode)
                                    },
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
        },
        None => {
            // No behavior available - default to Ref for all arguments
            arg_values.into_iter().map(CallArg::borrow).collect()
        },
    }
}

/// Infer method type parameter substitutions by matching parameter types against argument types.
///
/// This supplements explicit substitutions with inferred mappings for method-level type params,
/// using the call site's argument types.
fn infer_type_param_substitutions_from_call(
    base_subs: &Substitutions,
    callable: &CallableBehavior,
    arg_types: &[&Ty],
    is_instance_method: bool,
    receiver_ty: Option<&Ty>,
    method_param_ids: &[SymbolId],
) -> Substitutions {
    use std::collections::HashSet;

    if method_param_ids.is_empty() {
        return base_subs.clone();
    }

    let mut subs = base_subs.clone();
    let method_param_set: HashSet<SymbolId> = method_param_ids.iter().copied().collect();
    let param_offset = if is_instance_method { 1 } else { 0 };

    for (param_index, param) in callable.parameters().iter().enumerate() {
        let arg_index = param_index + param_offset;
        let Some(arg_ty) = arg_types.get(arg_index).copied() else {
            continue;
        };

        let mut param_ty = param.ty.apply_substitutions(&subs);
        if let Some(self_ty) = receiver_ty {
            param_ty = param_ty.substitute_self(self_ty);
        }

        collect_type_param_substitutions(
            &param_ty,
            arg_ty,
            &method_param_set,
            &mut subs,
        );
    }

    subs
}

fn collect_type_param_substitutions(
    param_ty: &Ty,
    arg_ty: &Ty,
    method_param_ids: &std::collections::HashSet<SymbolId>,
    subs: &mut Substitutions,
) {
    use semantic_tree::symbol::Symbol;

    match param_ty.kind() {
        TyKind::TypeParameter(param_symbol) => {
            let param_id = Symbol::metadata(param_symbol.as_ref()).id();
            if method_param_ids.contains(&param_id) && !subs.contains(param_id) {
                subs.insert(param_id, arg_ty.clone());
            }
        },
        TyKind::Tuple(param_elems) => {
            if let Some(arg_elems) = arg_ty.as_tuple() {
                for (p, a) in param_elems.iter().zip(arg_elems.iter()) {
                    collect_type_param_substitutions(p, a, method_param_ids, subs);
                }
            }
        },
        TyKind::Function {
            params: param_params,
            return_type: param_ret,
        } => {
            if let TyKind::Function {
                params: arg_params,
                return_type: arg_ret,
            } = arg_ty.kind()
            {
                if param_params.len() == arg_params.len() {
                    for (p, a) in param_params.iter().zip(arg_params.iter()) {
                        collect_type_param_substitutions(p, a, method_param_ids, subs);
                    }
                    collect_type_param_substitutions(param_ret, arg_ret, method_param_ids, subs);
                }
            }
        },
        TyKind::Pointer(param_inner) => {
            if let TyKind::Pointer(arg_inner) = arg_ty.kind() {
                collect_type_param_substitutions(param_inner, arg_inner, method_param_ids, subs);
            }
        },
        TyKind::Struct {
            symbol: param_sym,
            substitutions: param_subs,
        } => {
            if let TyKind::Struct {
                symbol: arg_sym,
                substitutions: arg_subs,
            } = arg_ty.kind()
                && param_sym.metadata().id() == arg_sym.metadata().id()
            {
                for (key, p_sub) in param_subs.iter() {
                    if let Some(a_sub) = arg_subs.get(*key) {
                        collect_type_param_substitutions(p_sub, a_sub, method_param_ids, subs);
                    }
                }
            }
        },
        TyKind::Enum {
            symbol: param_sym,
            substitutions: param_subs,
        } => {
            if let TyKind::Enum {
                symbol: arg_sym,
                substitutions: arg_subs,
            } = arg_ty.kind()
                && param_sym.metadata().id() == arg_sym.metadata().id()
            {
                for (key, p_sub) in param_subs.iter() {
                    if let Some(a_sub) = arg_subs.get(*key) {
                        collect_type_param_substitutions(p_sub, a_sub, method_param_ids, subs);
                    }
                }
            }
        },
        TyKind::Protocol {
            symbol: param_sym,
            substitutions: param_subs,
        } => {
            if let TyKind::Protocol {
                symbol: arg_sym,
                substitutions: arg_subs,
            } = arg_ty.kind()
                && param_sym.metadata().id() == arg_sym.metadata().id()
            {
                for (key, p_sub) in param_subs.iter() {
                    if let Some(a_sub) = arg_subs.get(*key) {
                        collect_type_param_substitutions(p_sub, a_sub, method_param_ids, subs);
                    }
                }
            }
        },
        TyKind::TypeAlias {
            symbol: param_sym,
            substitutions: param_subs,
        } => {
            if let TyKind::TypeAlias {
                symbol: arg_sym,
                substitutions: arg_subs,
            } = arg_ty.kind()
                && param_sym.metadata().id() == arg_sym.metadata().id()
            {
                for (key, p_sub) in param_subs.iter() {
                    if let Some(a_sub) = arg_subs.get(*key) {
                        collect_type_param_substitutions(p_sub, a_sub, method_param_ids, subs);
                    }
                }
            }
        },
        _ => {},
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
        },
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
        },
        Value::Unreachable => Value::Unreachable,
    }
}

/// Extract the local ID from a Value if it's a simple local reference.
/// Returns None for complex places (field access, etc.) or immediates.
pub fn try_get_local_from_value(value: &Value) -> Option<Id<Local>> {
    match value {
        Value::Place(place) => place.as_local(),
        _ => None,
    }
}

/// Mark locals as moved for any arguments passed with Move mode.
fn mark_moved_args(ctx: &mut LoweringContext, call_args: &[CallArg]) {
    for arg in call_args {
        if arg.mode == PassingMode::Move
            && let Some(local) = try_get_local_from_value(&arg.value)
        {
            ctx.mark_moved(local);
        }
    }
}

/// Build call arguments for indirect calls (function pointers/closures).
///
/// For indirect calls, we don't have CallableBehavior, so we determine passing
/// semantics from the function type itself.
///
/// For closures and function pointers, the parameters in the function type
/// represent the actual types expected by the callee. If the function type
/// says `(i64, i64) -> i64`, the closure expects `i64` values, not references.
fn build_indirect_call_args(
    _ctx: &mut LoweringContext,
    arg_values: Vec<Value>,
    arg_types: &[&Ty],
    _callee_ty: &Ty,
) -> Vec<CallArg> {
    // For indirect calls (closures/function pointers), pass arguments by value.
    // The closure's function type specifies the exact parameter types it expects.
    // If the closure takes `(i64, i64)`, we pass `i64` values, not references.
    //
    // Note: This differs from direct function calls where Kestrel's default
    // borrow semantics apply. Closures define their own parameter passing
    // convention through their function type signature.
    arg_values
        .into_iter()
        .enumerate()
        .map(|(i, value)| {
            // Get the argument type if available
            if let Some(&arg_ty) = arg_types.get(i) {
                // Pass by value - copy if copyable, move otherwise
                let mode = if arg_ty.is_copyable() {
                    PassingMode::Copy
                } else {
                    PassingMode::Move
                };
                CallArg::new(value, mode)
            } else {
                // Fallback: no type info, use copy mode
                CallArg::new(value, PassingMode::Copy)
            }
        })
        .collect()
}

/// Fill in default arguments for a call if any are missing.
///
/// Returns an extended `Vec<CallArgument>` that includes default expressions
/// for any missing parameters. If no defaults are needed, returns None to
/// indicate the original arguments can be used.
///
/// # Arguments
/// * `ctx` - The lowering context (for symbol queries)
/// * `arguments` - The arguments provided at the call site
/// * `symbol_id` - The symbol ID of the callee (function/method/initializer)
/// * `substitutions` - Type substitutions for the call (for generic functions)
fn fill_default_arguments(
    ctx: &LoweringContext,
    arguments: &[CallArgument],
    symbol_id: SymbolId,
    substitutions: &Substitutions,
) -> Option<Vec<CallArgument>> {
    use kestrel_span::Span;
    use semantic_tree::symbol::Symbol;

    // Look up the symbol
    let symbol = ctx.model.query(SymbolFor { id: symbol_id })?;

    // Get callable behavior to know the expected parameters
    let callable = symbol.metadata().get_behavior::<CallableBehavior>()?;
    let params = callable.parameters();

    // If we have all parameters, no need to fill defaults
    if arguments.len() >= params.len() {
        return None;
    }

    // Get default values from ResolvedExecutableBehavior or ExecutableBehavior
    // Protocol method declarations may only have ExecutableBehavior, not ResolvedExecutableBehavior
    // Clone the defaults into a Vec so we own them
    use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
    let default_values: Vec<Option<Expression>> =
        if let Some(resolved_exec) = symbol.metadata().get_behavior::<ResolvedExecutableBehavior>() {
            resolved_exec.default_values().to_vec()
        } else if let Some(exec_beh) = symbol.metadata().get_behavior::<ExecutableBehavior>() {
            exec_beh.default_values().to_vec()
        } else {
            // No executable behavior - can't fill defaults
            return None;
        };

    // Build the extended arguments list
    let mut filled_args: Vec<CallArgument> = arguments.to_vec();

    // For each missing parameter, try to get its default value
    for i in arguments.len()..params.len() {
        // Get the default expression for this parameter
        let default_expr = match default_values.get(i) {
            Some(Some(expr)) => expr,
            _ => {
                // No default available - this shouldn't happen if semantic analysis
                // validated the call correctly, but we can't fill it in
                return None;
            },
        };

        // Apply type substitutions to the default expression
        // This handles cases like `func foo[T](x: T = T.default())` called as `foo[Int]()`
        let substituted_expr = default_expr.apply_substitutions(substitutions);

        // Get the label for this parameter (if any)
        let param = &params[i];
        let label = param.external_label().map(|s| s.to_string());

        // Create a CallArgument from the default expression
        let call_arg = CallArgument {
            label,
            value: substituted_expr,
            span: Span::new(0, 0..0), // Synthetic span for default argument
        };

        filled_args.push(call_arg);
    }

    Some(filled_args)
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
        ExprKind::Literal(lit) => lower_literal(ctx, lit, expr),

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
        },

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
                            // We need to generate a thunk to adapt the calling convention
                            // from thin (no env) to thick (env as first param).
                            lower_function_ref_as_value(ctx, &sym, &expr.ty, &expr.span)
                        },
                        KestrelSymbolKind::Field => {
                            // Global/module-level field or static field accessed by name
                            // (e.g., `globalLet` at module scope or `_s` in a static context)
                            //
                            // This is similar to static field access via TypeRef, but the
                            // field is accessed directly by name rather than through a type.

                            // Check if this is a computed property
                            let is_computed = sym
                                .as_ref()
                                .downcast_ref::<FieldSymbol>()
                                .map(|f| f.is_computed())
                                .unwrap_or(false);

                            if is_computed {
                                // Computed property - need to call the getter
                                // Look up the getter symbol
                                let getter_id = sym
                                    .as_ref()
                                    .downcast_ref::<FieldSymbol>()
                                    .and_then(|f| f.getter());

                                if let Some(getter_id) = getter_id {
                                    // Generate a call to the getter with no receiver
                                    // (static computed property)
                                    return lower_static_getter_call(ctx, getter_id, expr);
                                } else {
                                    ctx.emit_error(LoweringError::internal(
                                        "computed property has no getter",
                                        Some(expr.span.clone()),
                                    ));
                                    return Value::Immediate(Immediate::error());
                                }
                            }

                            // Stored field - create a global place reference
                            // Build the qualified name for the field
                            let name_id = qualified_name_for_symbol(ctx, &sym);

                            // Register the static in MIR if not already registered
                            let static_exists =
                                ctx.mir.statics.iter().any(|(_, def)| def.name == name_id);
                            if !static_exists {
                                let mir_ty = lower_type(ctx, &expr.ty);
                                // Check if this is a file constant
                                if let Some(fc_behavior) =
                                    sym.metadata().get_behavior::<FileConstantBehavior>()
                                {
                                    // Extract element type from LiteralSlice[T]
                                    let element_ty =
                                        extract_literal_slice_element_type(ctx, &expr.ty);
                                    // Get base path from the symbol's span file_id
                                    let file_id = sym.metadata().span().file_id;
                                    let base_path = ctx.file_directory(file_id).cloned();
                                    ctx.mir.add_file_constant_static(
                                        name_id,
                                        mir_ty,
                                        fc_behavior.relative_path().to_string(),
                                        element_ty,
                                        base_path,
                                    );
                                } else {
                                    ctx.mir.add_static(name_id, mir_ty);
                                }
                            }

                            Value::Place(Place::global(name_id))
                        },
                        _ => {
                            ctx.emit_error(LoweringError::unsupported_expr(
                                format!("SymbolRef to {:?}", kind),
                                expr.span.clone(),
                            ));
                            Value::Immediate(Immediate::error())
                        },
                    }
                },
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("symbol not found: {:?}", symbol_id),
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                },
            }
        },

        // === Field Access ===
        ExprKind::FieldAccess { object, field } => {
            // Check if this is a static field access via TypeRef (e.g., Foo.staticField)
            if matches!(object.kind, ExprKind::TypeRef(_)) {
                // Static field access - need to find the field symbol and create a global place
                let field_info = find_field_info(ctx, &object.ty, field);

                if let Some((field_id, is_computed)) = field_info {
                    if is_computed {
                        // Static computed property - generate a getter call
                        return lower_getter_call(ctx, object, field_id, field, expr);
                    } else {
                        // Static stored field - create a global place reference
                        // Get the field symbol to build the qualified name and type
                        let field_symbol = match ctx.model.query(SymbolFor { id: field_id }) {
                            Some(sym) => sym,
                            None => {
                                ctx.emit_error(LoweringError::internal(
                                    format!("static field symbol not found: {:?}", field_id),
                                    Some(expr.span.clone()),
                                ));
                                return Value::Immediate(Immediate::error());
                            },
                        };

                        // Build the qualified name for the static field
                        let name_id = qualified_name_for_symbol(ctx, &field_symbol);

                        // Register the static in MIR if not already registered
                        // Check if this static already exists
                        let static_exists =
                            ctx.mir.statics.iter().any(|(_, def)| def.name == name_id);
                        if !static_exists {
                            // Get the field type and lower it
                            let field_ty = &expr.ty;
                            let mir_ty = lower_type(ctx, field_ty);
                            // Check if this is a file constant
                            if let Some(fc_behavior) =
                                field_symbol.metadata().get_behavior::<FileConstantBehavior>()
                            {
                                // Extract element type from LiteralSlice[T]
                                let element_ty =
                                    extract_literal_slice_element_type(ctx, field_ty);
                                // Get base path from the symbol's span file_id
                                let file_id = field_symbol.metadata().span().file_id;
                                let base_path = ctx.file_directory(file_id).cloned();
                                ctx.mir.add_file_constant_static(
                                    name_id,
                                    mir_ty,
                                    fc_behavior.relative_path().to_string(),
                                    element_ty,
                                    base_path,
                                );
                            } else {
                                ctx.mir.add_static(name_id, mir_ty);
                            }
                        }

                        return Value::Place(Place::global(name_id));
                    }
                } else {
                    ctx.emit_error(LoweringError::internal(
                        format!("static field '{}' not found in type", field),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                }
            }

            // Instance field access
            // Check if this is a computed property access
            let field_info = find_field_info(ctx, &object.ty, field);

            if let Some((field_id, is_computed)) = field_info
                && is_computed
            {
                // Computed property - generate a getter call
                return lower_getter_call(ctx, object, field_id, field, expr);
            }

            // Not computed - use direct field access
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
                },
                Value::Unreachable => Value::Unreachable,
            }
        },

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
                },
                Value::Unreachable => Value::Unreachable,
            }
        },

        // === Assignment ===
        ExprKind::Assignment { target, value } => {
            // Check if target is a computed property field access
            if let ExprKind::FieldAccess {
                object,
                field: field_name,
            } = &target.kind
            {
                let field_info = find_field_info(ctx, &object.ty, field_name);
                if let Some((field_id, is_computed)) = field_info
                    && is_computed
                {
                    // Computed property assignment - generate a setter call
                    return lower_setter_call(ctx, object, field_id, field_name, value, expr);
                }
            }

            // Check if target is a SymbolRef pointing to a computed field (module-level computed property)
            if let ExprKind::SymbolRef(symbol_id) = &target.kind
                && let Some(symbol) = ctx.model.query(SymbolFor { id: *symbol_id })
                && symbol.metadata().kind() == KestrelSymbolKind::Field
            {
                // Check if it's a computed property
                let is_computed = symbol
                    .as_ref()
                    .downcast_ref::<FieldSymbol>()
                    .map(|f| f.is_computed())
                    .unwrap_or(false);

                if is_computed {
                    // Get the setter ID
                    let setter_id = symbol
                        .as_ref()
                        .downcast_ref::<FieldSymbol>()
                        .and_then(|f| f.setter());

                    if let Some(setter_id) = setter_id {
                        // Generate a call to the setter
                        return lower_static_setter_call(ctx, setter_id, value, expr);
                    } else {
                        ctx.emit_error(LoweringError::internal(
                            "computed property has no setter",
                            Some(expr.span.clone()),
                        ));
                        return Value::Immediate(Immediate::error());
                    }
                }
            }

            // Check if target is a subscript call (subscript assignment)
            if let ExprKind::SubscriptCall {
                receiver,
                getter,
                arguments,
            } = &target.kind
            {
                // Subscript assignment - generate a setter call
                return lower_subscript_setter_call(ctx, receiver, *getter, arguments, value, expr);
            }

            // Check if target is a protocol property access (witness dispatch setter)
            if let ExprKind::ProtocolPropertyAccess {
                receiver,
                property_name,
                protocol_id,
                is_static,
                ..
            } = &target.kind
            {
                return lower_protocol_property_setter(
                    ctx,
                    receiver,
                    property_name,
                    *protocol_id,
                    *is_static,
                    value,
                    expr,
                );
            }

            // Not a computed property or subscript - use direct assignment
            let target_place = match lower_expression(ctx, target) {
                Value::Place(p) => p,
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::internal(
                        "assignment to non-place",
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                },
                Value::Unreachable => return Value::Unreachable,
            };

            let rhs_value = lower_expression(ctx, value);

            // Use copy for Copy types, move for non-Copy types
            match rhs_value {
                Value::Place(src_place) => {
                    let rvalue = if value.ty.is_copyable() {
                        Rvalue::Copy(src_place)
                    } else {
                        Rvalue::Move(src_place)
                    };
                    ctx.emit_assign(target_place, rvalue);
                },
                Value::Immediate(imm) => {
                    ctx.emit_assign(target_place, Rvalue::Use(imm));
                },
                Value::Unreachable => return Value::Unreachable,
            }

            // Assignment expression yields unit (actually Never in semantic tree)
            Value::Immediate(Immediate::unit())
        },

        // === Primitive Method Calls (operators) ===
        ExprKind::PrimitiveMethodCall {
            receiver,
            method,
            arguments,
        } => lower_primitive_method_call(ctx, receiver, *method, arguments, expr),

        // Primitive method reference (not called) - this shouldn't reach lowering
        // If it does, it means the primitive method was used as a first-class value,
        // which is not allowed.
        ExprKind::PrimitiveMethodRef { method, .. } => {
            ctx.emit_error(LoweringError::internal(
                format!(
                    "primitive method '{}' cannot be used as a value",
                    method.name()
                ),
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        // === Struct Construction ===
        ExprKind::ImplicitStructInit {
            struct_type,
            arguments,
        } => lower_struct_init(ctx, struct_type, arguments, expr),

        // === Delegating Initializer ===
        ExprKind::DelegatingInit {
            initializer,
            arguments,
            substitutions,
        } => lower_delegating_init(ctx, *initializer, arguments, substitutions, expr),

        // === Function/Method Calls ===
        ExprKind::Call {
            callee,
            arguments,
            substitutions,
        } => lower_call(ctx, callee, arguments, substitutions, expr),

        ExprKind::SubscriptCall {
            receiver,
            getter,
            arguments,
        } => lower_subscript_call(ctx, receiver, *getter, arguments, expr),

        // === Control Flow ===
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => lower_if(ctx, conditions, then_branch, then_value, else_branch, expr),

        ExprKind::While {
            loop_id,
            label,
            condition,
            body,
        } => lower_while(
            ctx,
            *loop_id,
            label.as_ref().map(|l| l.name.clone()),
            condition,
            body,
            expr,
        ),

        ExprKind::Loop {
            loop_id,
            label,
            body,
        } => lower_loop(
            ctx,
            *loop_id,
            label.as_ref().map(|l| l.name.clone()),
            body,
            expr,
        ),

        ExprKind::WhileLet {
            loop_id,
            label,
            conditions,
            body,
            ..
        } => lower_while_let(
            ctx,
            *loop_id,
            label.as_ref().map(|l| l.name.clone()),
            conditions,
            body,
        ),

        ExprKind::Break { loop_id, label } => {
            // Find the target loop and jump to its exit block
            let loop_info = ctx.find_loop(*loop_id).or_else(|| {
                label
                    .as_ref()
                    .and_then(|l| ctx.find_loop_by_label(l.name.as_str()))
                    .or_else(|| ctx.innermost_loop())
            });
            if let Some(loop_info) = loop_info {
                let exit_block = loop_info.exit_block;
                // Emit deinits for scopes between current and target loop
                ctx.emit_deinits_to_loop(loop_info.loop_id);
                ctx.emit_jump(exit_block);
            } else {
                if std::env::var("KESTREL_DEBUG_LOOPS").is_ok() {
                    let func_name = ctx
                        .current_function()
                        .map(|fid| ctx.mir.name(ctx.mir.function(fid).name).to_string())
                        .unwrap_or_else(|| "<none>".to_string());
                    eprintln!(
                        "break loop not found in {}: {:?}, stack={:?}",
                        func_name,
                        loop_id,
                        ctx.loop_stack_ids()
                    );
                }
                ctx.emit_error(LoweringError::internal(
                    "break: loop not found in loop stack",
                    Some(expr.span.clone()),
                ));
            }
            // Break never produces a value (it transfers control)
            Value::Unreachable
        },

        ExprKind::Continue { loop_id, label } => {
            // Find the target loop and jump to its header block
            let loop_info = ctx.find_loop(*loop_id).or_else(|| {
                label
                    .as_ref()
                    .and_then(|l| ctx.find_loop_by_label(l.name.as_str()))
                    .or_else(|| ctx.innermost_loop())
            });
            if let Some(loop_info) = loop_info {
                let header_block = loop_info.header_block;
                // Emit deinits for scopes between current and target loop
                ctx.emit_deinits_to_loop(loop_info.loop_id);
                ctx.emit_jump(header_block);
            } else {
                ctx.emit_error(LoweringError::internal(
                    "continue: loop not found in loop stack",
                    Some(expr.span.clone()),
                ));
            }
            // Continue never produces a value (it transfers control)
            Value::Unreachable
        },

        ExprKind::Return { value } => {
            let ret_value = if let Some(v) = value {
                lower_expression(ctx, v)
            } else {
                Value::Immediate(Immediate::unit())
            };
            // Mark the return value's local as moved so it doesn't get deinited.
            // The caller takes ownership of the return value.
            if let Some(local) = try_get_local_from_value(&ret_value) {
                ctx.mark_moved(local);
            }
            // Emit deinits for all scopes before returning
            ctx.emit_all_scope_deinits();
            ctx.emit_return(ret_value);
            // Return diverges - this value is never used (block is terminated)
            Value::Unreachable
        },

        // Throw should have been desugared to return by the binder
        // This arm should never be reached in practice
        ExprKind::Throw { .. } => {
            panic!("Throw expression should have been desugared to return by the binder")
        },

        // === Match Expressions ===
        ExprKind::Match { scrutinee, arms } => {
            crate::match_lowering::lower_match_expr(ctx, scrutinee, arms, expr)
        },

        // === Block Expressions ===
        // Used for match arm bodies with statements. NOT a closure - executes inline.
        ExprKind::Block { statements, value } => {
            // Lower statements in order
            for stmt in statements {
                crate::stmt::lower_statement(ctx, stmt);
                if ctx.is_block_terminated() {
                    break;
                }
            }

            // Lower the trailing value expression if present and block not terminated
            if !ctx.is_block_terminated() {
                if let Some(val) = value {
                    lower_expression(ctx, val)
                } else {
                    Value::Immediate(Immediate::unit())
                }
            } else {
                // Block was terminated (e.g., by return) - value is unreachable
                Value::Unreachable
            }
        },

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
        ExprKind::Array(elements) => lower_array_literal(ctx, elements, expr),

        ExprKind::Dictionary(pairs) => lower_dictionary_literal(ctx, pairs, expr),

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
        },

        ExprKind::Grouping(inner) => lower_expression(ctx, inner),

        ExprKind::OverloadedRef(_) => {
            // Should be resolved by now
            ctx.emit_error(LoweringError::internal(
                "unresolved overloaded reference",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        ExprKind::TypeRef(_) => {
            // Type references shouldn't appear as values
            ctx.emit_error(LoweringError::internal(
                "type reference as value",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        ExprKind::TypeParameterRef(_) => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "type parameter reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::error())
        },

        ExprKind::AssociatedTypeRef => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "associated type reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::error())
        },

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
        },

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
                },
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("enum case symbol not found: {:?}", case_id),
                        Some(expr.span.clone()),
                    ));
                    Value::Immediate(Immediate::error())
                },
            }
        },

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
        },

        ExprKind::DeferredMethodCall { method_name, .. } => {
            // Should be resolved by type inference
            ctx.emit_error(LoweringError::internal(
                format!("unresolved deferred method call '.{}'", method_name),
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        ExprKind::DeferredStaticCall { method_name, .. } => {
            // Should be resolved by type inference
            ctx.emit_error(LoweringError::internal(
                format!("unresolved deferred static call '.{}'", method_name),
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        // === Language Intrinsics ===
        ExprKind::LangIntrinsic {
            intrinsic,
            arguments,
        } => {
            use kestrel_execution_graph::TerminatorKind;
            use kestrel_execution_graph::function::ImmediateKind;
            use kestrel_semantic_tree::expr::LangIntrinsic;

            match intrinsic {
                LangIntrinsic::PanicUnwind => {
                    // Lower the message argument
                    let message_value = if let Some(arg) = arguments.first() {
                        lower_expression(ctx, &arg.value)
                    } else {
                        // Should not happen after binder validation
                        Value::Immediate(Immediate::string("panic".to_string()))
                    };

                    // Extract the message string
                    let message = match message_value {
                        Value::Immediate(imm) => {
                            match &imm.kind {
                                ImmediateKind::StringLiteral(s) => s.clone(),
                                _ => {
                                    // For non-constant strings, use a placeholder
                                    "<dynamic panic message>".to_string()
                                },
                            }
                        },
                        _ => {
                            // For non-immediate values, use a placeholder
                            "<dynamic panic message>".to_string()
                        },
                    };

                    // Emit the panic terminator
                    ctx.emit_terminator(TerminatorKind::Panic(message));

                    // Start a new unreachable block (panic never returns)
                    let unreachable_block = ctx.create_block();
                    ctx.set_current_block(unreachable_block);

                    // Panic diverges - return Unreachable so callers don't try to use this value
                    Value::Unreachable
                },
                LangIntrinsic::Cast { from, to } => {
                    // Lower the operand argument
                    let operand = if let Some(arg) = arguments.first() {
                        lower_expression(ctx, &arg.value)
                    } else {
                        // Should not happen after binder validation
                        ctx.emit_error(LoweringError::internal(
                            "cast intrinsic missing argument",
                            Some(expr.span.clone()),
                        ));
                        return Value::Immediate(Immediate::error());
                    };

                    // Determine the cast kind based on from/to primitives
                    let cast_kind = determine_cast_kind(*from, *to);

                    // Lower the target type
                    let target_ty = lower_type(ctx, &expr.ty);

                    // Emit the cast
                    let result = ctx.create_temp("cast", target_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::Cast {
                            kind: cast_kind,
                            operand,
                            target: target_ty,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::IntBinary { op, .. } => {
                    use kestrel_semantic_tree::expr::IntBinaryOp;
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);

                    let bin_op = match op {
                        IntBinaryOp::Add => BinOp::AddSigned,
                        IntBinaryOp::Sub => BinOp::SubSigned,
                        IntBinaryOp::Mul => BinOp::MulSigned,
                        IntBinaryOp::Eq => BinOp::Eq,
                        IntBinaryOp::Ne => BinOp::Ne,
                        IntBinaryOp::And => BinOp::And,
                        IntBinaryOp::Or => BinOp::Or,
                        IntBinaryOp::Xor => BinOp::Xor,
                        IntBinaryOp::Shl => BinOp::Shl,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("int_op", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::BinaryOp {
                            op: bin_op,
                            lhs,
                            rhs,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::IntBinarySigned { op, .. } => {
                    use kestrel_semantic_tree::expr::SignedOp;
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);

                    let bin_op = match op {
                        SignedOp::Div => BinOp::DivSigned,
                        SignedOp::Rem => BinOp::RemSigned,
                        SignedOp::Shr => BinOp::ShrSigned,
                        SignedOp::Lt => BinOp::LtSigned,
                        SignedOp::Le => BinOp::LeSigned,
                        SignedOp::Gt => BinOp::GtSigned,
                        SignedOp::Ge => BinOp::GeSigned,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("signed_op", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::BinaryOp {
                            op: bin_op,
                            lhs,
                            rhs,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::IntBinaryUnsigned { op, .. } => {
                    use kestrel_semantic_tree::expr::SignedOp;
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);

                    let bin_op = match op {
                        SignedOp::Div => BinOp::DivUnsigned,
                        SignedOp::Rem => BinOp::RemUnsigned,
                        SignedOp::Shr => BinOp::ShrUnsigned,
                        SignedOp::Lt => BinOp::LtUnsigned,
                        SignedOp::Le => BinOp::LeUnsigned,
                        SignedOp::Gt => BinOp::GtUnsigned,
                        SignedOp::Ge => BinOp::GeUnsigned,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("unsigned_op", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::BinaryOp {
                            op: bin_op,
                            lhs,
                            rhs,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::IntUnary { op, .. } => {
                    use kestrel_semantic_tree::expr::IntUnaryOp;
                    let operand = lower_expression(ctx, &arguments[0].value);

                    let un_op = match op {
                        IntUnaryOp::Neg => UnOp::Neg,
                        IntUnaryOp::Not => UnOp::Not,
                        IntUnaryOp::Popcount => UnOp::Popcount,
                        IntUnaryOp::Clz => UnOp::Clz,
                        IntUnaryOp::Ctz => UnOp::Ctz,
                        IntUnaryOp::Bswap => UnOp::Bswap,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("int_unary", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::UnaryOp { op: un_op, operand });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatBinary { op, .. } => {
                    use kestrel_semantic_tree::expr::FloatBinaryOp;
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);

                    let bin_op = match op {
                        FloatBinaryOp::Add => BinOp::FAdd,
                        FloatBinaryOp::Sub => BinOp::FSub,
                        FloatBinaryOp::Mul => BinOp::FMul,
                        FloatBinaryOp::Div => BinOp::FDiv,
                        FloatBinaryOp::Eq => BinOp::FEq,
                        FloatBinaryOp::Ne => BinOp::FNe,
                        FloatBinaryOp::Lt => BinOp::FLt,
                        FloatBinaryOp::Le => BinOp::FLe,
                        FloatBinaryOp::Gt => BinOp::FGt,
                        FloatBinaryOp::Ge => BinOp::FGe,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_op", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::BinaryOp {
                            op: bin_op,
                            lhs,
                            rhs,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatUnary { op, .. } => {
                    use kestrel_semantic_tree::expr::FloatUnaryOp;
                    let operand = lower_expression(ctx, &arguments[0].value);

                    let un_op = match op {
                        FloatUnaryOp::Neg => UnOp::FNeg,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_unary", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::UnaryOp { op: un_op, operand });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatConst {
                    primitive,
                    constant,
                } => {
                    use kestrel_execution_graph::function::{FloatBits, FloatConstantKind};
                    use kestrel_semantic_tree::expr::{FloatConstant, LangPrimitive};

                    let bits = match primitive {
                        LangPrimitive::F16 => FloatBits::F16,
                        LangPrimitive::F32 => FloatBits::F32,
                        LangPrimitive::F64 => FloatBits::F64,
                        _ => unreachable!("float constant on non-float primitive"),
                    };

                    let const_kind = match constant {
                        FloatConstant::Infinity => FloatConstantKind::Infinity,
                        FloatConstant::Nan => FloatConstantKind::Nan,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_const", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::FloatConst {
                            bits,
                            constant: const_kind,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatPred { primitive, pred } => {
                    use kestrel_execution_graph::function::{FloatBits, FloatPredicateKind};
                    use kestrel_semantic_tree::expr::{FloatPredicate, LangPrimitive};

                    let operand = lower_expression(ctx, &arguments[0].value);

                    let bits = match primitive {
                        LangPrimitive::F16 => FloatBits::F16,
                        LangPrimitive::F32 => FloatBits::F32,
                        LangPrimitive::F64 => FloatBits::F64,
                        _ => unreachable!("float predicate on non-float primitive"),
                    };

                    let pred_kind = match pred {
                        FloatPredicate::IsNan => FloatPredicateKind::IsNan,
                        FloatPredicate::IsInfinite => FloatPredicateKind::IsInfinite,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_pred", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::FloatPred {
                            bits,
                            pred: pred_kind,
                            operand,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatMath { primitive, op } => {
                    use kestrel_execution_graph::function::{FloatBits, FloatMathKind};
                    use kestrel_semantic_tree::expr::{FloatMathOp, LangPrimitive};

                    let operand = lower_expression(ctx, &arguments[0].value);

                    let bits = match primitive {
                        LangPrimitive::F16 => FloatBits::F16,
                        LangPrimitive::F32 => FloatBits::F32,
                        LangPrimitive::F64 => FloatBits::F64,
                        _ => unreachable!("float math on non-float primitive"),
                    };

                    let math_kind = match op {
                        FloatMathOp::Floor => FloatMathKind::Floor,
                        FloatMathOp::Ceil => FloatMathKind::Ceil,
                        FloatMathOp::Round => FloatMathKind::Round,
                        FloatMathOp::Trunc => FloatMathKind::Trunc,
                        FloatMathOp::Sqrt => FloatMathKind::Sqrt,
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_math", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::FloatMath {
                            bits,
                            op: math_kind,
                            operand,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatFma { primitive } => {
                    use kestrel_execution_graph::function::FloatBits;
                    use kestrel_semantic_tree::expr::LangPrimitive;

                    let a = lower_expression(ctx, &arguments[0].value);
                    let b = lower_expression(ctx, &arguments[1].value);
                    let c = lower_expression(ctx, &arguments[2].value);

                    let bits = match primitive {
                        LangPrimitive::F16 => FloatBits::F16,
                        LangPrimitive::F32 => FloatBits::F32,
                        LangPrimitive::F64 => FloatBits::F64,
                        _ => unreachable!("float fma on non-float primitive"),
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_fma", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::FloatFma { bits, a, b, c });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::FloatCopysign { primitive } => {
                    use kestrel_execution_graph::function::FloatBits;
                    use kestrel_semantic_tree::expr::LangPrimitive;

                    let magnitude = lower_expression(ctx, &arguments[0].value);
                    let sign_source = lower_expression(ctx, &arguments[1].value);

                    let bits = match primitive {
                        LangPrimitive::F16 => FloatBits::F16,
                        LangPrimitive::F32 => FloatBits::F32,
                        LangPrimitive::F64 => FloatBits::F64,
                        _ => unreachable!("float copysign on non-float primitive"),
                    };

                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("float_copysign", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::FloatCopysign {
                            bits,
                            magnitude,
                            sign_source,
                        },
                    );
                    Value::Place(Place::local(result))
                },

                // === Pointer intrinsics ===
                LangIntrinsic::PtrNull { .. } => {
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_null", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrNull { ty: result_ty });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrFromAddress { .. } => {
                    let address = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_from_addr", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::PtrFromAddress {
                            ty: result_ty,
                            address,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrToAddress => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_to_addr", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrToAddress { ptr });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrTo { .. } => {
                    let value = lower_expression(ctx, &arguments[0].value);
                    let ref_value = create_ref(ctx, &value, &arguments[0].value.ty, false);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_to", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::RefToPtr(ref_value));
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrRead { .. } => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_read", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrRead { ptr, ty: result_ty });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrWrite { .. } => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let value = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_write", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrWrite { ptr, value });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrOffset => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let offset = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_offset", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrOffset { ptr, offset });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::PtrIsNull => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("ptr_is_null", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::PtrIsNull { ptr });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::CastPtr { .. } => {
                    let ptr = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("cast_ptr", result_ty);
                    ctx.emit_assign(
                        Place::local(result),
                        Rvalue::PtrCast {
                            ptr,
                            target_ty: result_ty,
                        },
                    );
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::SizeOf { .. } => {
                    // sizeof returns the size of the type parameter
                    // For now, we need to get the type from the intrinsic
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("sizeof", result_ty);
                    // Get the pointee type from the SizeOf variant
                    let size_ty = match intrinsic {
                        LangIntrinsic::SizeOf { ty } => lower_type(ctx, ty),
                        _ => unreachable!(),
                    };
                    ctx.emit_assign(Place::local(result), Rvalue::SizeOf { ty: size_ty });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::AlignOf { .. } => {
                    // alignof returns the alignment of the type parameter
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("alignof", result_ty);
                    let align_ty = match intrinsic {
                        LangIntrinsic::AlignOf { ty } => lower_type(ctx, ty),
                        _ => unreachable!(),
                    };
                    ctx.emit_assign(Place::local(result), Rvalue::AlignOf { ty: align_ty });
                    Value::Place(Place::local(result))
                },
                // Boolean (i1) intrinsics
                LangIntrinsic::I1Eq => {
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("i1_eq", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::I1Eq { lhs, rhs });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::I1And => {
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("i1_and", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::I1And { lhs, rhs });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::I1Or => {
                    let lhs = lower_expression(ctx, &arguments[0].value);
                    let rhs = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("i1_or", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::I1Or { lhs, rhs });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::I1Not => {
                    let operand = lower_expression(ctx, &arguments[0].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("i1_not", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::I1Not { operand });
                    Value::Place(Place::local(result))
                },
                // Atomic intrinsics
                LangIntrinsic::AtomicAdd => {
                    // First argument is a place expression - we need its address
                    let place_value = lower_expression(ctx, &arguments[0].value);
                    let ptr = match place_value {
                        Value::Place(p) => {
                            // Get address of place: ref -> ptr
                            let ptr_ty = lower_type(ctx, &expr.ty);
                            let ref_temp = ctx.create_temp("atomic_ref", ptr_ty);
                            ctx.emit_assign(Place::local(ref_temp), Rvalue::Ref(p));
                            let ptr_temp = ctx.create_temp("atomic_ptr", ptr_ty);
                            ctx.emit_assign(
                                Place::local(ptr_temp),
                                Rvalue::RefToPtr(Value::Place(Place::local(ref_temp))),
                            );
                            Value::Place(Place::local(ptr_temp))
                        },
                        v => v, // Already a value (shouldn't happen, but pass through)
                    };
                    let delta = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("atomic_add", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::AtomicAdd { ptr, delta });
                    Value::Place(Place::local(result))
                },
                LangIntrinsic::AtomicSub => {
                    // First argument is a place expression - we need its address
                    let place_value = lower_expression(ctx, &arguments[0].value);
                    let ptr = match place_value {
                        Value::Place(p) => {
                            // Get address of place: ref -> ptr
                            let ptr_ty = lower_type(ctx, &expr.ty);
                            let ref_temp = ctx.create_temp("atomic_ref", ptr_ty);
                            ctx.emit_assign(Place::local(ref_temp), Rvalue::Ref(p));
                            let ptr_temp = ctx.create_temp("atomic_ptr", ptr_ty);
                            ctx.emit_assign(
                                Place::local(ptr_temp),
                                Rvalue::RefToPtr(Value::Place(Place::local(ref_temp))),
                            );
                            Value::Place(Place::local(ptr_temp))
                        },
                        v => v, // Already a value (shouldn't happen, but pass through)
                    };
                    let delta = lower_expression(ctx, &arguments[1].value);
                    let result_ty = lower_type(ctx, &expr.ty);
                    let result = ctx.create_temp("atomic_sub", result_ty);
                    ctx.emit_assign(Place::local(result), Rvalue::AtomicSub { ptr, delta });
                    Value::Place(Place::local(result))
                },
            }
        },

        ExprKind::LangIntrinsicRef(_) => {
            // Intrinsic reference without a call - this is an error
            // (intrinsics cannot be used as first-class values)
            ctx.emit_error(LoweringError::internal(
                "lang intrinsic cannot be used as a value",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },

        // === Protocol Property Access (via witness table) ===
        ExprKind::ProtocolPropertyAccess {
            receiver,
            field_id: _,
            property_name,
            protocol_id,
            is_static,
            has_setter: _,
        } => {
            // Access a computed property on a type parameter through witness dispatch
            lower_protocol_property_access(
                ctx,
                receiver,
                property_name,
                *protocol_id,
                *is_static,
                expr,
            )
        },

        ExprKind::InterpolatedString { parts } => lower_interpolated_string(ctx, parts, expr),

        ExprKind::Error => {
            // Error expression - return error value (error already reported)
            Value::Immediate(Immediate::error())
        },
    }
}

/// Build FormatOptions struct fields from a parsed format spec.
///
/// Maps parsed format spec values to FormatOptions struct field values.
fn build_format_options_fields(
    ctx: &mut LoweringContext,
    spec: &FormatSpec,
    _span: &kestrel_span::Span,
) -> Vec<(String, Value)> {


    let mut fields: Vec<(String, Value)> = Vec::new();

    // Get type IDs needed for construction
    let i64_ty = ctx.mir.ty_i64();

    // Build Optional[Int64] type for width and precision fields
    let optional_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
        "std".to_string(),
        "result".to_string(),
        "Optional".to_string(),
    ]));
    let optional_i64_ty = ctx.mir.ty_named(optional_name, vec![i64_ty]);

    // width: Int64?
    let width_value = if let Some(w) = spec.width {
        // Create Some(w)
        let some_local = ctx.create_temp("width_some", optional_i64_ty);
        let some_place = Place::local(some_local);
        let width_imm = Value::Immediate(Immediate::i64(w as i64));
        ctx.emit_assign(
            some_place.clone(),
            Rvalue::EnumVariant {
                enum_ty: optional_i64_ty,
                variant: "Some".to_string(),
                payload: vec![width_imm],
            },
        );
        Value::Place(some_place)
    } else {
        // Create None
        let none_local = ctx.create_temp("width_none", optional_i64_ty);
        let none_place = Place::local(none_local);
        ctx.emit_assign(
            none_place.clone(),
            Rvalue::EnumVariant {
                enum_ty: optional_i64_ty,
                variant: "None".to_string(),
                payload: vec![],
            },
        );
        Value::Place(none_place)
    };
    fields.push(("width".to_string(), width_value));

    // precision: Int64?
    let precision_value = if let Some(p) = spec.precision {
        // Create Some(p)
        let some_local = ctx.create_temp("precision_some", optional_i64_ty);
        let some_place = Place::local(some_local);
        let precision_imm = Value::Immediate(Immediate::i64(p as i64));
        ctx.emit_assign(
            some_place.clone(),
            Rvalue::EnumVariant {
                enum_ty: optional_i64_ty,
                variant: "Some".to_string(),
                payload: vec![precision_imm],
            },
        );
        Value::Place(some_place)
    } else {
        // Create None
        let none_local = ctx.create_temp("precision_none", optional_i64_ty);
        let none_place = Place::local(none_local);
        ctx.emit_assign(
            none_place.clone(),
            Rvalue::EnumVariant {
                enum_ty: optional_i64_ty,
                variant: "None".to_string(),
                payload: vec![],
            },
        );
        Value::Place(none_place)
    };
    fields.push(("precision".to_string(), precision_value));

    // alignment: Alignment
    let alignment_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
        "std".to_string(),
        "text".to_string(),
        "Alignment".to_string(),
    ]));
    let alignment_ty = ctx.mir.ty_named(alignment_name, vec![]);
    let alignment_local = ctx.create_temp("alignment", alignment_ty);
    let alignment_place = Place::local(alignment_local);
    let alignment_variant = match spec.alignment {
        Alignment::Left => "Left",
        Alignment::Right => "Right",
        Alignment::Center => "Center",
    };
    ctx.emit_assign(
        alignment_place.clone(),
        Rvalue::EnumVariant {
            enum_ty: alignment_ty,
            variant: alignment_variant.to_string(),
            payload: vec![],
        },
    );
    fields.push(("alignment".to_string(), Value::Place(alignment_place)));

    // fill: Char (represented as i32 in MIR)
    let fill_value = Value::Immediate(Immediate::i32(spec.fill as i32));
    fields.push(("fill".to_string(), fill_value));

    // radix: Int64 - determined by format type
    let radix: i64 = match spec.format_type {
        FormatType::Binary => 2,
        FormatType::Octal => 8,
        FormatType::Hex | FormatType::HexUpper => 16,
        _ => 10,
    };
    fields.push(("radix".to_string(), Value::Immediate(Immediate::i64(radix))));

    // uppercase: Bool
    let uppercase = matches!(spec.format_type, FormatType::HexUpper);
    fields.push(("uppercase".to_string(), Value::Immediate(Immediate::bool(uppercase))));

    // sign: Sign
    let sign_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
        "std".to_string(),
        "text".to_string(),
        "Sign".to_string(),
    ]));
    let sign_ty = ctx.mir.ty_named(sign_name, vec![]);
    let sign_local = ctx.create_temp("sign", sign_ty);
    let sign_place = Place::local(sign_local);
    let sign_variant = match spec.sign {
        crate::format_spec::SignMode::Negative => "Negative",
        crate::format_spec::SignMode::Always => "Always",
        crate::format_spec::SignMode::Space => "Space",
    };
    ctx.emit_assign(
        sign_place.clone(),
        Rvalue::EnumVariant {
            enum_ty: sign_ty,
            variant: sign_variant.to_string(),
            payload: vec![],
        },
    );
    fields.push(("sign".to_string(), Value::Place(sign_place)));

    // alternate: Bool
    fields.push(("alternate".to_string(), Value::Immediate(Immediate::bool(spec.alternate))));

    // floatStyle: FloatStyle
    let float_style_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
        "std".to_string(),
        "text".to_string(),
        "FloatStyle".to_string(),
    ]));
    let float_style_ty = ctx.mir.ty_named(float_style_name, vec![]);
    let float_style_local = ctx.create_temp("float_style", float_style_ty);
    let float_style_place = Place::local(float_style_local);
    let float_style_variant = match spec.format_type {
        FormatType::Fixed => "Fixed",
        FormatType::Scientific => "Scientific",
        FormatType::ScientificUpper => "ScientificUpper",
        FormatType::Percent => "Percent",
        _ => "Auto",
    };
    ctx.emit_assign(
        float_style_place.clone(),
        Rvalue::EnumVariant {
            enum_ty: float_style_ty,
            variant: float_style_variant.to_string(),
            payload: vec![],
        },
    );
    fields.push(("floatStyle".to_string(), Value::Place(float_style_place)));

    // debug: Bool
    let debug = matches!(spec.format_type, FormatType::Debug);
    fields.push(("debug".to_string(), Value::Immediate(Immediate::bool(debug))));

    fields
}

/// Lower an interpolated string expression.
///
/// Interpolated strings like `"Hello \(name)!"` are lowered to:
/// 1. Create a DefaultStringInterpolation instance
/// 2. For each part:
///    - If literal: call appendLiteral(literal:)
///    - If interpolation: call appendInterpolation(value:options:)
/// 3. Call build() to get the final string
fn lower_interpolated_string(
    ctx: &mut LoweringContext,
    parts: &[InterpolationPart],
    expr: &Expression,
) -> Value {
    let span = expr.span.clone();
    let result_ty = lower_type(ctx, &expr.ty);

    // Count literal and interpolation parts for capacity hints
    let mut literal_capacity: i64 = 0;
    let mut interpolation_count: i64 = 0;
    for part in parts {
        match part {
            InterpolationPart::Literal { text, .. } => {
                literal_capacity += text.len() as i64;
            },
            InterpolationPart::Interpolation { .. } => {
                interpolation_count += 1;
            },
        }
    }

    // Look up DefaultStringInterpolation struct
    let Some(dsi_id) = ctx.model.builtin_registry().default_string_interpolation() else {
        ctx.emit_error(LoweringError::internal(
            "DefaultStringInterpolation not found in builtin registry",
            Some(span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    };

    let Some(dsi_symbol) = ctx.model.query(SymbolFor { id: dsi_id }) else {
        ctx.emit_error(LoweringError::internal(
            "DefaultStringInterpolation symbol not found",
            Some(span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    };

    // Get the DSI type
    let dsi_struct_sym = dsi_symbol
        .clone()
        .downcast_arc::<kestrel_semantic_tree::symbol::r#struct::StructSymbol>()
        .unwrap();
    let dsi_ty = Ty::r#struct(dsi_struct_sym, span.clone());
    let mir_dsi_ty = lower_type(ctx, &dsi_ty);

    // Allocate space for the DefaultStringInterpolation instance
    let dsi_local = ctx.create_temp("interpolation", mir_dsi_ty);
    let dsi_place = Place::local(dsi_local);

    // Create a mutable reference to the DSI
    let ref_ty = ctx.mir.ty_ref_mut(mir_dsi_ty);
    let dsi_ref_local = ctx.create_temp("interp_ref", ref_ty);
    let dsi_ref_place = Place::local(dsi_ref_local);

    // Emit: %dsi_ref = ref mut %dsi
    ctx.emit_assign(dsi_ref_place.clone(), Rvalue::RefMut(dsi_place.clone()));

    // Build qualified name for DefaultStringInterpolation
    let dsi_name = qualified_name_for_symbol(ctx, &dsi_symbol);
    let dsi_name_parts = ctx.mir.name(dsi_name).segments.clone();

    // Call init(literalCapacity:interpolationCount:)
    let init_name_parts = {
        let mut parts = dsi_name_parts.clone();
        parts.push("init$literalCapacity$interpolationCount".to_string());
        parts
    };
    let init_name = ctx.mir.intern_name(QualifiedNameData::new(init_name_parts));

    let unit_ty = ctx.mir.ty_unit();
    let init_ret_local = ctx.create_temp("init_ret", unit_ty);
    let init_ret_place = Place::local(init_ret_local);

    let init_args = vec![
        CallArg::mutating(Value::Place(dsi_ref_place.clone())),
        CallArg::borrow(Value::Immediate(Immediate::i64(literal_capacity))),
        CallArg::borrow(Value::Immediate(Immediate::i64(interpolation_count))),
    ];

    ctx.emit_call_with_modes(init_ret_place, Callee::direct(init_name), init_args);

    // For each part, call the appropriate append method
    for part in parts {
        // Re-create the mutable reference (it may have been consumed)
        let part_ref_local = ctx.create_temp("part_ref", ref_ty);
        let part_ref_place = Place::local(part_ref_local);
        ctx.emit_assign(part_ref_place.clone(), Rvalue::RefMut(dsi_place.clone()));

        match part {
            InterpolationPart::Literal { text, .. } => {
                // Call appendLiteral(literal:)
                let append_name_parts = {
                    let mut parts = dsi_name_parts.clone();
                    parts.push("appendLiteral$literal".to_string());
                    parts
                };
                let append_name = ctx
                    .mir
                    .intern_name(QualifiedNameData::new(append_name_parts));

                // Create a string immediate for the literal
                let literal_ptr = Value::Immediate(Immediate::string_ptr(text.clone()));
                let literal_len = Value::Immediate(Immediate::i64(text.len() as i64));

                // Get String type and create a temp for it
                let string_ty = lower_type(ctx, &Ty::string(span.clone()));
                let string_local = ctx.create_temp("literal_str", string_ty);
                let string_place = Place::local(string_local);

                // Create mutable ref to string for init
                let string_ref_ty = ctx.mir.ty_ref_mut(string_ty);
                let string_ref_local = ctx.create_temp("string_ref", string_ref_ty);
                let string_ref_place = Place::local(string_ref_local);
                ctx.emit_assign(
                    string_ref_place.clone(),
                    Rvalue::RefMut(string_place.clone()),
                );

                // Call String.init(stringLiteral:length:)
                let string_init_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
                    "std".to_string(),
                    "text".to_string(),
                    "String".to_string(),
                    "init$stringLiteral$length".to_string(),
                ]));
                let string_init_ret_local = ctx.create_temp("str_init_ret", unit_ty);
                let string_init_ret_place = Place::local(string_init_ret_local);
                ctx.emit_call_with_modes(
                    string_init_ret_place,
                    Callee::direct(string_init_name),
                    vec![
                        CallArg::mutating(Value::Place(string_ref_place)),
                        CallArg::borrow(literal_ptr),
                        CallArg::borrow(literal_len),
                    ],
                );

                // Create borrow ref to string for appendLiteral
                let string_borrow_ty = ctx.mir.ty_ref(string_ty);
                let string_borrow_local = ctx.create_temp("string_borrow", string_borrow_ty);
                let string_borrow_place = Place::local(string_borrow_local);
                ctx.emit_assign(
                    string_borrow_place.clone(),
                    Rvalue::Ref(string_place.clone()),
                );

                let append_ret_local = ctx.create_temp("append_ret", unit_ty);
                let append_ret_place = Place::local(append_ret_local);

                ctx.emit_call_with_modes(
                    append_ret_place,
                    Callee::direct(append_name),
                    vec![
                        CallArg::mutating(Value::Place(part_ref_place)),
                        CallArg::borrow(Value::Place(string_borrow_place)),
                    ],
                );
            },
            InterpolationPart::Interpolation {
                expr: interp_expr,
                format_spec,
                ..
            } => {
                // Lower the interpolation expression
                let interp_value = lower_expression(ctx, interp_expr);

                // Call appendInterpolation(value:options:)
                // The function name only includes 'value' because 'options' has a default
                // (parameters with defaults are excluded from the qualified name)
                // But we still need to pass the options argument to the function
                let append_name_parts = {
                    let mut parts = dsi_name_parts.clone();
                    parts.push("appendInterpolation$value".to_string());
                    parts
                };
                let append_name = ctx
                    .mir
                    .intern_name(QualifiedNameData::new(append_name_parts));

                // Get the expression's type for the generic call
                let interp_ty = lower_type(ctx, &interp_expr.ty);

                // Create a ref to the interpolation value
                // If it's already a place, ref it. If it's an immediate, store it first.
                let interp_ref_ty = ctx.mir.ty_ref(interp_ty);
                let interp_ref_local = ctx.create_temp("interp_ref", interp_ref_ty);
                let interp_ref_place = Place::local(interp_ref_local);

                match interp_value {
                    Value::Place(p) => {
                        ctx.emit_assign(interp_ref_place.clone(), Rvalue::Ref(p));
                    },
                    Value::Immediate(imm) => {
                        // Store the immediate in a temp, then ref it
                        let temp_local = ctx.create_temp("interp_temp", interp_ty);
                        let temp_place = Place::local(temp_local);
                        ctx.emit_assign(temp_place.clone(), Rvalue::Use(imm));
                        ctx.emit_assign(interp_ref_place.clone(), Rvalue::Ref(temp_place));
                    },
                    Value::Unreachable => {
                        // Unreachable value - skip this interpolation
                        continue;
                    },
                }

                // Look up FormatOptions type from the model
                let format_opts_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
                    "std".to_string(),
                    "text".to_string(),
                    "FormatOptions".to_string(),
                ]));
                let format_opts_ty = ctx.mir.ty_named(format_opts_name, vec![]);
                let format_opts_local = ctx.create_temp("format_opts", format_opts_ty);
                let format_opts_place = Place::local(format_opts_local);

                // Parse format spec and create FormatOptions with parsed values
                if let Some(spec_str) = format_spec {
                    if let Ok(spec) = parse_format_spec(spec_str) {
                        // Build FormatOptions struct with parsed values
                        let fields = build_format_options_fields(ctx, &spec, &span);
                        ctx.emit_assign(
                            format_opts_place.clone(),
                            Rvalue::Construct { ty: format_opts_ty, fields },
                        );
                    } else {
                        // Failed to parse format spec, use defaults
                        let format_opts_default_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
                            "std".to_string(),
                            "text".to_string(),
                            "FormatOptions".to_string(),
                            "default".to_string(),
                        ]));
                        ctx.emit_call_with_modes(
                            format_opts_place.clone(),
                            Callee::direct(format_opts_default_name),
                            vec![],
                        );
                    }
                } else {
                    // No format spec, use FormatOptions.default()
                    let format_opts_default_name = ctx.mir.intern_name(QualifiedNameData::new(vec![
                        "std".to_string(),
                        "text".to_string(),
                        "FormatOptions".to_string(),
                        "default".to_string(),
                    ]));
                    ctx.emit_call_with_modes(
                        format_opts_place.clone(),
                        Callee::direct(format_opts_default_name),
                        vec![],
                    );
                }

                // Borrow FormatOptions for passing to appendInterpolation
                let format_opts_borrow_ty = ctx.mir.ty_ref(format_opts_ty);
                let format_opts_borrow_local =
                    ctx.create_temp("format_opts_borrow", format_opts_borrow_ty);
                let format_opts_borrow_place = Place::local(format_opts_borrow_local);
                ctx.emit_assign(
                    format_opts_borrow_place.clone(),
                    Rvalue::Ref(format_opts_place),
                );

                let append_ret_local = ctx.create_temp("append_ret", unit_ty);
                let append_ret_place = Place::local(append_ret_local);

                // Call as a generic method with type parameter
                ctx.emit_call_with_modes(
                    append_ret_place,
                    Callee::direct_generic(append_name, vec![interp_ty]),
                    vec![
                        CallArg::mutating(Value::Place(part_ref_place)),
                        CallArg::borrow(Value::Place(interp_ref_place)),
                        CallArg::borrow(Value::Place(format_opts_borrow_place)),
                    ],
                );
            },
        }
    }

    // Call build() to get the final string
    // First, borrow the DSI
    let build_ref_ty = ctx.mir.ty_ref(mir_dsi_ty);
    let build_ref_local = ctx.create_temp("build_ref", build_ref_ty);
    let build_ref_place = Place::local(build_ref_local);
    ctx.emit_assign(build_ref_place.clone(), Rvalue::Ref(dsi_place.clone()));

    let build_name_parts = {
        let mut parts = dsi_name_parts;
        parts.push("build".to_string());
        parts
    };
    let build_name = ctx
        .mir
        .intern_name(QualifiedNameData::new(build_name_parts));

    // Allocate space for the result string
    let result_local = ctx.create_temp("result", result_ty);
    let result_place = Place::local(result_local);

    // Call build() - returns String by value
    ctx.emit_call_with_modes(
        result_place.clone(),
        Callee::direct(build_name),
        vec![CallArg::borrow(Value::Place(build_ref_place))],
    );

    Value::Place(result_place)
}

/// Lower an array literal expression.
///
/// Array literals like `[1, 2, 3]` are lowered to:
/// 1. Stack allocate a buffer for the elements
/// 2. Write each element to the buffer
/// 3. Call the target type's init(_arrayLiteralPointer:_arrayLiteralCount:) method
fn lower_array_literal(
    ctx: &mut LoweringContext,
    elements: &[Expression],
    expr: &Expression,
) -> Value {
    // Expand type aliases (e.g., [T] -> ArrayTypeOperator[T] -> Array[T])
    let target_ty = expr.ty.expand_aliases();

    // Get element type from Array[T] struct type
    let element_sem_ty: Ty = match target_ty.kind() {
        TyKind::Struct { substitutions, .. } => {
            // For Array[T] struct types, get T from substitutions
            // This works for both Array[T] and Array[T, Allocator]
            substitutions
                .iter()
                .next()
                .map(|(_, t)| t.clone())
                .or_else(|| elements.first().map(|e| e.ty.clone()))
                .unwrap_or_else(|| {
                    // Empty array with no type info - use unit type as placeholder
                    Ty::unit(expr.span.clone())
                })
        },
        _ => {
            ctx.emit_error(LoweringError::internal(
                "array literal with non-array type",
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    let element_ty = lower_type(ctx, &element_sem_ty);
    let count = elements.len();

    // Lower each element expression
    let element_values: Vec<Value> = elements.iter().map(|e| lower_expression(ctx, e)).collect();

    // Allocate stack buffer for elements
    let ptr_ty = ctx.mir.ty_ptr(element_ty);
    let ptr_local = ctx.create_temp("array_literal_ptr", ptr_ty);
    let ptr_place = Place::local(ptr_local);

    let i64_ty = ctx.mir.ty_i64();
    let count_value = Value::Immediate(Immediate::i64(count as i64));

    ctx.emit_assign(
        ptr_place.clone(),
        Rvalue::StackAlloc {
            element_ty,
            count: count_value.clone(),
        },
    );

    // Write each element to the buffer using PtrOffset and PtrWrite
    // For computing byte offsets, we use index * sizeof(element_ty)
    let sizeof_local = ctx.create_temp("elem_size", i64_ty);
    ctx.emit_assign(
        Place::local(sizeof_local),
        Rvalue::SizeOf { ty: element_ty },
    );

    // Pre-compute unit type to avoid borrow issues
    let unit_ty = ctx.mir.ty_unit();

    for (i, elem_value) in element_values.into_iter().enumerate() {
        if i == 0 {
            // First element: write directly to ptr
            let unit_local = ctx.create_temp("ptr_write", unit_ty);
            ctx.emit_assign(
                Place::local(unit_local),
                Rvalue::PtrWrite {
                    ptr: Value::Place(ptr_place.clone()),
                    value: elem_value,
                },
            );
        } else {
            // Compute byte offset: i * sizeof(element_ty)
            let index_value = Value::Immediate(Immediate::i64(i as i64));
            let offset_local = ctx.create_temp("elem_offset", i64_ty);
            ctx.emit_assign(
                Place::local(offset_local),
                Rvalue::BinaryOp {
                    op: BinOp::MulSigned,
                    lhs: index_value,
                    rhs: Value::Place(Place::local(sizeof_local)),
                },
            );

            // Compute element pointer: ptr + offset
            let elem_ptr_local = ctx.create_temp("elem_ptr", ptr_ty);
            ctx.emit_assign(
                Place::local(elem_ptr_local),
                Rvalue::PtrOffset {
                    ptr: Value::Place(ptr_place.clone()),
                    offset: Value::Place(Place::local(offset_local)),
                },
            );

            // Write element value
            let unit_local = ctx.create_temp("ptr_write", unit_ty);
            ctx.emit_assign(
                Place::local(unit_local),
                Rvalue::PtrWrite {
                    ptr: Value::Place(Place::local(elem_ptr_local)),
                    value: elem_value,
                },
            );
        }
    }

    // Look up the init method with _arrayLiteralPointer and _arrayLiteralCount labels
    // For now, we need to find the target type's init method
    // Use target_ty which has type aliases expanded
    match target_ty.kind() {
        TyKind::Struct { symbol, .. } => {
            // Call the struct's array literal init
            lower_array_literal_init_call(ctx, expr, &target_ty, symbol, ptr_place, count_value)
        },
        _ => {
            ctx.emit_error(LoweringError::internal(
                "unexpected array literal target type",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },
    }
}

/// Lower an array literal init call to a struct type.
fn lower_array_literal_init_call(
    ctx: &mut LoweringContext,
    expr: &Expression,
    target_ty: &Ty,
    struct_symbol: &std::sync::Arc<kestrel_semantic_tree::symbol::r#struct::StructSymbol>,
    ptr_place: Place,
    count_value: Value,
) -> Value {
    use semantic_tree::symbol::Symbol;

    // Find the init with _arrayLiteralPointer and _arrayLiteralCount parameters
    // Note: We check bind_name (internal name) not label (external label) because
    // the protocol uses single-name syntax `_arrayLiteralPointer: Type` where the
    // underscore prefix makes the external label `_` (no label) while the full name
    // becomes the internal bind name.
    let init_symbol = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .find(|child| {
            if child.metadata().kind() != KestrelSymbolKind::Initializer {
                return false;
            }
            // Check if this init has parameters with the right bind names
            if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                let params = callable.parameters();
                params.len() >= 2
                    && params
                        .first()
                        .is_some_and(|p| p.bind_name.value == "_arrayLiteralPointer")
                    && params
                        .get(1)
                        .is_some_and(|p| p.bind_name.value == "_arrayLiteralCount")
            } else {
                false
            }
        });

    let Some(_init_sym) = init_symbol else {
        ctx.emit_error(LoweringError::internal(
            "array literal target type has no init(_arrayLiteralPointer:_arrayLiteralCount:)",
            Some(expr.span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    };

    // Build the qualified name for the init function
    // Array literal inits are named: init$_arrayLiteralPointer$_arrayLiteralCount
    let mut name_parts = Vec::new();
    collect_symbol_name_parts(
        &(struct_symbol.clone()
            as std::sync::Arc<
                dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
            >),
        &mut name_parts,
    );
    name_parts.push("init$_arrayLiteralPointer$_arrayLiteralCount".to_string());

    let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

    // Lower the result type (use the expanded target type, not the original which may be a type alias)
    let result_ty = lower_type(ctx, target_ty);

    // Allocate space for the result
    let result_local = ctx.create_temp("array_literal", result_ty);
    let result_place = Place::local(result_local);

    // Create a mutable reference to the result place
    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
    let self_ref_place = Place::local(self_ref_local);

    // Emit: %self_ref = ref var %result
    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));

    // Build call args: self_ref first (MutRef), then pointer and count (both borrowed)
    // The function signature takes references: &p[T] and &i64
    let call_args = vec![
        CallArg::mutating(Value::Place(self_ref_place)),
        CallArg::borrow(Value::Place(ptr_place)),
        CallArg::borrow(count_value),
    ];

    // Create a temp for the unit return value of init (we discard it)
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("init_ret", unit_ty);
    let unit_place = Place::local(unit_local);

    // Extract type arguments from the struct type
    let type_args = match extract_type_args_from_receiver(ctx, &expr.ty, Some(expr.span.clone())) {
        Some(args) => args,
        None => return Value::Immediate(Immediate::error()),
    };

    // Call the init function
    let mir_callee = if type_args.is_empty() {
        Callee::direct(init_name)
    } else {
        Callee::direct_generic(init_name, type_args)
    };
    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

    // Return the initialized struct
    Value::Place(result_place)
}

/// Lower a dictionary literal expression.
///
/// Dictionary literals work similarly to array literals:
/// 1. Stack allocate a buffer for (Key, Value) tuples
/// 2. Write each key-value pair as a tuple to the buffer
/// 3. Call the target type's init(_dictionaryLiteralPointer:_dictionaryLiteralCount:) method
fn lower_dictionary_literal(
    ctx: &mut LoweringContext,
    pairs: &[(Expression, Expression)],
    expr: &Expression,
) -> Value {
    // Expand type aliases (e.g., [K: V] -> DictionaryTypeOperator[K, V] -> Dictionary[K, V])
    let target_ty = expr.ty.expand_aliases();

    // Get Key and Value types from Dictionary[K, V] struct type
    let (key_sem_ty, value_sem_ty): (Ty, Ty) = match target_ty.kind() {
        TyKind::Struct { substitutions, .. } => {
            // For Dictionary[K, V] struct types, get K and V from substitutions
            let mut iter = substitutions.iter();
            let key_ty = iter
                .next()
                .map(|(_, t)| t.clone())
                .or_else(|| pairs.first().map(|(k, _)| k.ty.clone()))
                .unwrap_or_else(|| Ty::unit(expr.span.clone()));
            let value_ty = iter
                .next()
                .map(|(_, t)| t.clone())
                .or_else(|| pairs.first().map(|(_, v)| v.ty.clone()))
                .unwrap_or_else(|| Ty::unit(expr.span.clone()));
            (key_ty, value_ty)
        },
        _ => {
            ctx.emit_error(LoweringError::internal(
                "dictionary literal with non-dictionary type",
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    let key_ty = lower_type(ctx, &key_sem_ty);
    let value_ty = lower_type(ctx, &value_sem_ty);
    let count = pairs.len();

    // Create tuple type for (Key, Value)
    let pair_ty = ctx.mir.ty_tuple(vec![key_ty, value_ty]);

    // Lower each key-value pair and create tuple values
    let pair_values: Vec<Value> = pairs
        .iter()
        .map(|(k, v)| {
            let key_value = lower_expression(ctx, k);
            let value_value = lower_expression(ctx, v);

            // Create a tuple value from key and value
            let tuple_local = ctx.create_temp("dict_pair", pair_ty);
            let tuple_place = Place::local(tuple_local);
            ctx.emit_assign(
                tuple_place.clone(),
                Rvalue::Tuple(vec![key_value, value_value]),
            );
            Value::Place(tuple_place)
        })
        .collect();

    // Allocate stack buffer for tuples
    let ptr_ty = ctx.mir.ty_ptr(pair_ty);
    let ptr_local = ctx.create_temp("dict_literal_ptr", ptr_ty);
    let ptr_place = Place::local(ptr_local);

    let i64_ty = ctx.mir.ty_i64();
    let count_value = Value::Immediate(Immediate::i64(count as i64));

    ctx.emit_assign(
        ptr_place.clone(),
        Rvalue::StackAlloc {
            element_ty: pair_ty,
            count: count_value.clone(),
        },
    );

    // Write each pair to the buffer
    let sizeof_local = ctx.create_temp("pair_size", i64_ty);
    ctx.emit_assign(Place::local(sizeof_local), Rvalue::SizeOf { ty: pair_ty });

    let unit_ty = ctx.mir.ty_unit();

    for (i, pair_value) in pair_values.into_iter().enumerate() {
        if i == 0 {
            // First pair: write directly to ptr
            let unit_local = ctx.create_temp("ptr_write", unit_ty);
            ctx.emit_assign(
                Place::local(unit_local),
                Rvalue::PtrWrite {
                    ptr: Value::Place(ptr_place.clone()),
                    value: pair_value,
                },
            );
        } else {
            // Compute byte offset: i * sizeof(pair_ty)
            let index_value = Value::Immediate(Immediate::i64(i as i64));
            let offset_local = ctx.create_temp("pair_offset", i64_ty);
            ctx.emit_assign(
                Place::local(offset_local),
                Rvalue::BinaryOp {
                    op: BinOp::MulSigned,
                    lhs: index_value,
                    rhs: Value::Place(Place::local(sizeof_local)),
                },
            );

            // Compute offset pointer
            let offset_ptr_local = ctx.create_temp("offset_ptr", ptr_ty);
            ctx.emit_assign(
                Place::local(offset_ptr_local),
                Rvalue::PtrOffset {
                    ptr: Value::Place(ptr_place.clone()),
                    offset: Value::Place(Place::local(offset_local)),
                },
            );

            // Write pair to offset pointer
            let unit_local = ctx.create_temp("ptr_write", unit_ty);
            ctx.emit_assign(
                Place::local(unit_local),
                Rvalue::PtrWrite {
                    ptr: Value::Place(Place::local(offset_ptr_local)),
                    value: pair_value,
                },
            );
        }
    }

    // Call init(_dictionaryLiteralPointer:_dictionaryLiteralCount:) on the target type
    match target_ty.kind() {
        TyKind::Struct { symbol, .. } => lower_dictionary_literal_init_call(
            ctx,
            expr,
            &target_ty,
            symbol,
            ptr_place,
            count_value,
        ),
        _ => {
            ctx.emit_error(LoweringError::internal(
                "unexpected dictionary literal target type",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },
    }
}

/// Lower a dictionary literal init call to a struct type.
fn lower_dictionary_literal_init_call(
    ctx: &mut LoweringContext,
    expr: &Expression,
    target_ty: &Ty,
    struct_symbol: &std::sync::Arc<kestrel_semantic_tree::symbol::r#struct::StructSymbol>,
    ptr_place: Place,
    count_value: Value,
) -> Value {
    use semantic_tree::symbol::Symbol;

    // Find the init with _dictionaryLiteralPointer and _dictionaryLiteralCount parameters
    let init_symbol = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .find(|child| {
            if child.metadata().kind() != KestrelSymbolKind::Initializer {
                return false;
            }
            if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                let params = callable.parameters();
                params.len() >= 2
                    && params
                        .first()
                        .is_some_and(|p| p.bind_name.value == "_dictionaryLiteralPointer")
                    && params
                        .get(1)
                        .is_some_and(|p| p.bind_name.value == "_dictionaryLiteralCount")
            } else {
                false
            }
        });

    let Some(_init_sym) = init_symbol else {
        ctx.emit_error(LoweringError::internal(
            "dictionary literal target type has no init(_dictionaryLiteralPointer:_dictionaryLiteralCount:)",
            Some(expr.span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    };

    // Build the qualified name for the init function
    let mut name_parts = Vec::new();
    collect_symbol_name_parts(
        &(struct_symbol.clone()
            as std::sync::Arc<
                dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
            >),
        &mut name_parts,
    );
    name_parts.push("init$_dictionaryLiteralPointer$_dictionaryLiteralCount".to_string());

    let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

    // Lower the result type
    let result_ty = lower_type(ctx, target_ty);

    // Allocate space for the result
    let result_local = ctx.create_temp("dict_literal", result_ty);
    let result_place = Place::local(result_local);

    // Create a mutable reference to the result place
    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
    let self_ref_place = Place::local(self_ref_local);

    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));

    // Build call args: self_ref first (MutRef), then pointer and count (both borrowed)
    let call_args = vec![
        CallArg::mutating(Value::Place(self_ref_place)),
        CallArg::borrow(Value::Place(ptr_place)),
        CallArg::borrow(count_value),
    ];

    // Create a temp for the unit return value of init (we discard it)
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("init_ret", unit_ty);
    let unit_place = Place::local(unit_local);

    // Extract type arguments from the struct type
    let type_args = match extract_type_args_from_receiver(ctx, &expr.ty, Some(expr.span.clone())) {
        Some(args) => args,
        None => return Value::Immediate(Immediate::error()),
    };

    // Call the init function
    let mir_callee = if type_args.is_empty() {
        Callee::direct(init_name)
    } else {
        Callee::direct_generic(init_name, type_args)
    };
    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

    // Return the initialized struct
    Value::Place(result_place)
}

/// Lower a literal expression.
///
/// For primitive types (lang.i64, lang.f64, lang.i1, lang.str), returns the
/// immediate value directly.
///
/// For struct types (Int64, Float64, Bool, String, etc. that conform to
/// ExpressibleBy* protocols), generates an init call like:
///   Int64.init(intLiteral: <immediate value>)
fn lower_literal(ctx: &mut LoweringContext, lit: &LiteralValue, expr: &Expression) -> Value {
    // Expand type aliases before matching - type aliases should be transparent
    let ty = expr.ty.expand_aliases();

    // Check the resolved type of the literal expression
    match ty.kind() {
        // Primitive types - return immediate directly
        TyKind::Int(bits) => {
            let LiteralValue::Integer(n) = lit else {
                return Value::Immediate(Immediate::error());
            };
            Value::Immediate(make_int_immediate(*bits, *n))
        },
        TyKind::Float(bits) => {
            let LiteralValue::Float(f) = lit else {
                return Value::Immediate(Immediate::error());
            };
            Value::Immediate(make_float_immediate(*bits, *f))
        },
        TyKind::Bool => {
            let LiteralValue::Bool(b) = lit else {
                return Value::Immediate(Immediate::error());
            };
            Value::Immediate(Immediate::bool(*b))
        },
        TyKind::String => {
            let LiteralValue::String(s) = lit else {
                return Value::Immediate(Immediate::error());
            };
            Value::Immediate(Immediate::string(s.clone()))
        },
        TyKind::Unit => Value::Immediate(Immediate::unit()),

        // Struct types - generate init call
        TyKind::Struct { symbol, .. } => lower_literal_init_call(ctx, lit, expr, symbol),

        // Enum types - generate init call for null literals (Optional)
        TyKind::Enum { symbol, .. } => lower_enum_literal_init_call(ctx, lit, expr, symbol),

        // For inference variables or error types, fall back to immediate
        TyKind::Infer | TyKind::Error => match lit {
            LiteralValue::Unit => Value::Immediate(Immediate::unit()),
            LiteralValue::Integer(n) => Value::Immediate(Immediate::i64(*n)),
            LiteralValue::Float(f) => Value::Immediate(Immediate::f64(*f)),
            LiteralValue::Bool(b) => Value::Immediate(Immediate::bool(*b)),
            LiteralValue::Char(c) => Value::Immediate(Immediate::i32(*c as i32)),
            LiteralValue::String(s) => Value::Immediate(Immediate::string(s.clone())),
            LiteralValue::Null => Value::Immediate(Immediate::error()), // Null requires concrete type
        },

        // Other types - this shouldn't happen for literals
        other => {
            // Note: Don't use {:?} on TyKind as it can cause infinite recursion
            // due to circular symbol references in the Debug impl
            let type_desc = match other {
                TyKind::Enum { .. } => "enum",
                TyKind::Protocol { .. } => "protocol",
                TyKind::TypeParameter { .. } => "type parameter",
                TyKind::Tuple { .. } => "tuple",
                TyKind::Function { .. } => "function",
                TyKind::Pointer { .. } => "pointer",
                TyKind::TypeAlias { .. } => "type alias",
                TyKind::SelfType => "Self",
                TyKind::AssociatedType { .. } => "associated type",
                _ => "unknown",
            };
            ctx.emit_error(LoweringError::internal(
                format!("unexpected type for literal: {}", type_desc),
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::error())
        },
    }
}

/// Lower a literal to an init call for struct types that conform to ExpressibleBy* protocols.
///
/// For example, `42` with type `Int64` becomes:
///   1. Allocate temp for Int64 result
///   2. Create mutable reference to it
///   3. Call Int64.init(intLiteral: Immediate::i64(42))
fn lower_literal_init_call(
    ctx: &mut LoweringContext,
    lit: &LiteralValue,
    expr: &Expression,
    struct_symbol: &std::sync::Arc<kestrel_semantic_tree::symbol::r#struct::StructSymbol>,
) -> Value {
    use semantic_tree::symbol::Symbol;

    // Determine the init parameter label and primitive value based on literal type
    // Null literals use init() with no parameters, handled specially below
    let (init_label, primitive_value) = match lit {
        LiteralValue::Integer(n) => (Some("intLiteral"), Value::Immediate(Immediate::i64(*n))),
        LiteralValue::Float(f) => (Some("floatLiteral"), Value::Immediate(Immediate::f64(*f))),
        LiteralValue::Bool(b) => (Some("boolLiteral"), Value::Immediate(Immediate::bool(*b))),
        LiteralValue::Char(c) => (
            Some("charLiteral"),
            Value::Immediate(Immediate::i32(*c as i32)),
        ),
        LiteralValue::String(s) => (
            Some("stringLiteral"),
            Value::Immediate(Immediate::string(s.clone())),
        ),
        LiteralValue::Unit => return Value::Immediate(Immediate::unit()),
        LiteralValue::Null => (None, Value::Immediate(Immediate::unit())), // No parameter for null
    };

    // Find the init with the matching parameter label (or no parameters for null)
    let init_symbol = struct_symbol
        .metadata()
        .children()
        .into_iter()
        .find(|child| {
            if child.metadata().kind() != KestrelSymbolKind::Initializer {
                return false;
            }
            // Check if this init has the right parameters
            if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
                match init_label {
                    Some(label) => {
                        // Match init with specific parameter label
                        callable
                            .parameters()
                            .first()
                            .is_some_and(|p| p.label.as_ref().is_some_and(|l| l.value == label))
                    },
                    None => {
                        // Match init with no parameters (for null literals)
                        callable.parameters().is_empty()
                    },
                }
            } else {
                false
            }
        });

    let Some(init_sym) = init_symbol else {
        // No init found - fall back to immediate (this is the case for types
        // where the init is trivial or the type doesn't have the protocol)
        return match lit {
            LiteralValue::Integer(n) => Value::Immediate(Immediate::i64(*n)),
            LiteralValue::Float(f) => Value::Immediate(Immediate::f64(*f)),
            LiteralValue::Bool(b) => Value::Immediate(Immediate::bool(*b)),
            LiteralValue::Char(c) => Value::Immediate(Immediate::i32(*c as i32)),
            LiteralValue::String(s) => Value::Immediate(Immediate::string(s.clone())),
            LiteralValue::Unit => Value::Immediate(Immediate::unit()),
            LiteralValue::Null => Value::Immediate(Immediate::error()), // Null requires init
        };
    };

    // Build the qualified name for the init function
    // Initializers are named with ALL their parameter labels: init$intLiteral, init$stringLiteral$length, etc.
    let mut name_parts = Vec::new();
    collect_symbol_name_parts(
        &(struct_symbol.clone()
            as std::sync::Arc<
                dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
            >),
        &mut name_parts,
    );

    // Get all labels from the found init symbol
    let init_name_suffix =
        if let Some(callable) = init_sym.metadata().get_behavior::<CallableBehavior>() {
            // Use external labels if present, otherwise fall back to internal names
            let label_parts: Vec<&str> = callable
                .parameters()
                .iter()
                .map(|p| p.external_label().unwrap_or_else(|| p.internal_name()))
                .collect();
            if label_parts.is_empty() {
                "init".to_string()
            } else {
                format!("init${}", label_parts.join("$"))
            }
        } else if let Some(label) = init_label {
            format!("init${}", label)
        } else {
            "init".to_string()
        };
    name_parts.push(init_name_suffix);

    let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

    // Lower the result type
    let result_ty = lower_type(ctx, &expr.ty);

    // Allocate space for the result
    let result_local = ctx.create_temp("literal", result_ty);
    let result_place = Place::local(result_local);

    // Create a mutable reference to the result place
    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
    let self_ref_place = Place::local(self_ref_local);

    // Emit: %self_ref = ref var %result
    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));

    // Build call args: self_ref first (MutRef), then the primitive value(s)
    // String literals are special: they need both ptr and length as separate args
    // Null literals have no parameters (just self)
    let call_args = match lit {
        LiteralValue::String(s) => {
            // String.init(stringLiteral ptr: lang.ptr[lang.i8], length: lang.i64)
            // expects two primitive args: ptr and length (passed by reference)
            let ptr_value = Value::Immediate(Immediate::string_ptr(s.clone()));
            let len_value = Value::Immediate(Immediate::i64(s.len() as i64));
            vec![
                CallArg::mutating(Value::Place(self_ref_place)),
                CallArg::borrow(ptr_value),
                CallArg::borrow(len_value),
            ]
        },
        LiteralValue::Null => {
            // ExpressibleByNullLiteral.init() - no parameters, just self
            vec![CallArg::mutating(Value::Place(self_ref_place))]
        },
        _ => {
            // All other literal init methods take the primitive by reference (borrow)
            vec![
                CallArg::mutating(Value::Place(self_ref_place)),
                CallArg::borrow(primitive_value),
            ]
        },
    };

    // Create a temp for the unit return value of init (we discard it)
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("init_ret", unit_ty);
    let unit_place = Place::local(unit_local);

    // Call the init function
    let mir_callee = Callee::direct(init_name);
    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

    // Return the initialized struct
    Value::Place(result_place)
}

/// Lower a literal to an init call for enum types that conform to ExpressibleBy* protocols.
///
/// Currently only supports null literals with Optional types that implement
/// ExpressibleByNullLiteral with init().
fn lower_enum_literal_init_call(
    ctx: &mut LoweringContext,
    lit: &LiteralValue,
    expr: &Expression,
    enum_symbol: &std::sync::Arc<kestrel_semantic_tree::symbol::enum_symbol::EnumSymbol>,
) -> Value {
    use semantic_tree::symbol::Symbol;

    // Only null literals are supported for enum types currently
    if !matches!(lit, LiteralValue::Null) {
        ctx.emit_error(LoweringError::internal(
            "non-null literal with enum type".to_string(),
            Some(expr.span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    }

    // Find init() with no parameters (from ExpressibleByNullLiteral)
    let mut init_symbol: Option<
        std::sync::Arc<
            dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
        >,
    > = enum_symbol.metadata().children().into_iter().find(|child| {
        if child.metadata().kind() != KestrelSymbolKind::Initializer {
            return false;
        }
        if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>() {
            callable.parameters().is_empty()
        } else {
            false
        }
    });

    // If not found on the enum, check extension initializers
    if init_symbol.is_none() {
        use kestrel_semantic_model::queries::ExtensionsFor;

        let extensions = ctx.model.query(ExtensionsFor {
            target_id: enum_symbol.metadata().id(),
        });
        for extension in extensions {
            for child in extension.metadata().children() {
                if child.metadata().kind() != KestrelSymbolKind::Initializer {
                    continue;
                }
                if let Some(callable) = child.metadata().get_behavior::<CallableBehavior>()
                    && callable.parameters().is_empty()
                {
                    init_symbol = Some(child);
                    break;
                }
            }
            if init_symbol.is_some() {
                break;
            }
        }
    }

    let Some(_init_sym) = init_symbol else {
        ctx.emit_error(LoweringError::internal(
            "no init() found for null literal".to_string(),
            Some(expr.span.clone()),
        ));
        return Value::Immediate(Immediate::error());
    };

    // Build the qualified name for the init function
    let mut name_parts = Vec::new();
    collect_symbol_name_parts(
        &(enum_symbol.clone()
            as std::sync::Arc<
                dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
            >),
        &mut name_parts,
    );
    name_parts.push("init".to_string());

    // Get type arguments from the enum type (expand aliases like OptionalTypeOperator[T])
    let resolved_ty = expr.ty.expand_aliases();
    let type_args: Vec<_> = match resolved_ty.kind() {
        TyKind::Enum { substitutions, .. } => substitutions
            .iter()
            .map(|(_, ty)| lower_type(ctx, ty))
            .collect(),
        _ => Vec::new(),
    };

    let init_name = ctx.mir.intern_name(QualifiedNameData::new(name_parts));

    // Lower the result type
    let result_ty = lower_type(ctx, &expr.ty);

    // Allocate space for the result
    let result_local = ctx.create_temp("literal", result_ty);
    let result_place = Place::local(result_local);

    // Create a mutable reference to the result place
    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
    let self_ref_place = Place::local(self_ref_local);

    // Emit: %self_ref = ref var %result
    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));

    // Build call args: just self_ref (no other parameters for null literal)
    let call_args = vec![CallArg::mutating(Value::Place(self_ref_place))];

    // Create a temp for the unit return value of init (we discard it)
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("init_ret", unit_ty);
    let unit_place = Place::local(unit_local);

    // Call the init function
    let mir_callee = if type_args.is_empty() {
        Callee::direct(init_name)
    } else {
        Callee::direct_generic(init_name, type_args)
    };
    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

    // Return the initialized enum
    Value::Place(result_place)
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

    // Early return if receiver diverged
    if receiver_value.is_unreachable() {
        return Value::Unreachable;
    }

    // Determine if this is a unary or binary operation
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("prim", result_ty);
    let result_place = Place::local(result_local);

    match method {
        // === Unary Operations ===
        PrimitiveMethod::IntNeg => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::Neg,
                    operand: receiver_value,
                },
            );
        },

        PrimitiveMethod::FloatNeg => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::FNeg,
                    operand: receiver_value,
                },
            );
        },

        PrimitiveMethod::BoolNot => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::BoolNot,
                    operand: receiver_value,
                },
            );
        },

        PrimitiveMethod::IntBitNot => {
            ctx.emit_assign(
                result_place.clone(),
                Rvalue::UnaryOp {
                    op: UnOp::Not,
                    operand: receiver_value,
                },
            );
        },

        // === String methods (unary) ===
        PrimitiveMethod::StringLength => {
            // string.length() -> StrLen(string)
            ctx.emit_assign(result_place.clone(), Rvalue::StrLen(receiver_value));
        },

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
        },

        PrimitiveMethod::StringUnsafePtr => {
            // string.unsafePtr() -> StrPtr(string)
            ctx.emit_assign(result_place.clone(), Rvalue::StrPtr(receiver_value));
        },

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
                },
                Value::Unreachable => unreachable!("already handled above"),
            };

            // Create blocks for the conditional
            let neg_block = ctx.create_block();
            let pos_block = ctx.create_block();
            let join_block = ctx.create_block();

            // Check if value < 0
            let cmp_ty = ctx.mir.ty_bool();
            let cmp_local = ctx.create_temp("is_neg", cmp_ty);
            let cmp_place = Place::local(cmp_local);
            // Create a zero with the same bit width as the integer type
            let zero_imm =
                make_int_zero_for_mir_ty(ctx, int_ty).unwrap_or_else(|| Immediate::i64(0));
            ctx.emit_assign(
                cmp_place.clone(),
                Rvalue::BinaryOp {
                    op: BinOp::LtSigned,
                    lhs: Value::Place(receiver_place.clone()),
                    rhs: Value::Immediate(zero_imm),
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
        },

        PrimitiveMethod::IntToString => {
            // Convert integer to string using the IntToString operation
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("str", result_ty);
            let result_place = Place::local(result_local);

            ctx.emit_assign(result_place.clone(), Rvalue::IntToString(receiver_value));
            return Value::Place(result_place);
        },

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
                PrimitiveMethod::StringEq => BinOp::StrEq,
                PrimitiveMethod::StringNe => BinOp::Ne, // TODO: Add BinOp::StrNe

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
        },
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

/// Lower a delegating initializer call: `self.init(...)`
///
/// This is called from within an initializer body and calls another initializer
/// on the same struct. The `self` parameter is passed implicitly.
fn lower_delegating_init(
    ctx: &mut LoweringContext,
    initializer: SymbolId,
    arguments: &[CallArgument],
    substitutions: &kestrel_semantic_tree::ty::Substitutions,
    _expr: &Expression,
) -> Value {
    use kestrel_semantic_tree::symbol::EnumSymbol;
    use kestrel_semantic_tree::symbol::local::LocalId;
    use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
    use semantic_tree::symbol::Symbol;

    // Get the initializer symbol
    let Some(init_sym) = ctx.model.query(SymbolFor { id: initializer }) else {
        ctx.emit_error(LoweringError::internal(
            "delegating init: initializer symbol not found",
            None,
        ));
        return Value::Immediate(Immediate::unit());
    };

    // Get the qualified name for the initializer
    let init_name = qualified_name_for_symbol(ctx, &init_sym);

    // Lower the arguments (excluding self - it's passed implicitly)
    let arg_values: Vec<Value> = arguments
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Get `self` as the first argument - it's always local 0 in initializers
    // self is already a &var Self in initializers
    let self_local = ctx.get_local_unwrap(LocalId(0));
    let self_value = Value::Place(Place::local(self_local));

    // Build the full argument list: self + other args
    let mut all_args = vec![self_value];
    all_args.extend(arg_values);

    // Get type arguments from the parent struct/enum
    // Delegating inits need to pass the same type arguments as the enclosing generic type
    let type_args: Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> =
        if let Some(parent) = init_sym.metadata().parent() {
            let param_ids: Vec<SymbolId> =
                if let Some(struct_sym) = parent.as_ref().downcast_ref::<StructSymbol>() {
                    struct_sym
                        .type_parameters()
                        .iter()
                        .map(|tp| Symbol::metadata(tp.as_ref()).id())
                        .collect()
                } else if let Some(enum_sym) = parent.as_ref().downcast_ref::<EnumSymbol>() {
                    enum_sym
                        .type_parameters()
                        .iter()
                        .map(|tp| Symbol::metadata(tp.as_ref()).id())
                        .collect()
                } else {
                    vec![]
                };

            if let Some(ordered_types) = substitutions.types_in_order(&param_ids) {
                ordered_types
                    .into_iter()
                    .map(|ty| lower_type(ctx, ty))
                    .collect()
            } else {
                vec![]
            }
        } else {
            vec![]
        };

    // Emit the call to the delegated initializer
    // Delegating inits return unit (they modify self in-place)
    let callee = if type_args.is_empty() {
        Callee::direct(init_name)
    } else {
        Callee::direct_generic(init_name, type_args)
    };
    ctx.emit_call_unit(callee, all_args);

    // Delegating init returns unit
    Value::Immediate(Immediate::unit())
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

    // Try to extract the callee symbol ID to check for default arguments
    let callee_symbol_id: Option<SymbolId> = match &callee.kind {
        ExprKind::SymbolRef(symbol_id) => Some(*symbol_id),
        ExprKind::MethodRef { candidates, .. } => candidates.first().copied(),
        _ => None,
    };

    // Fill in default arguments if any are missing
    let filled_arguments: Option<Vec<CallArgument>> = callee_symbol_id
        .and_then(|symbol_id| fill_default_arguments(ctx, arguments, symbol_id, substitutions));

    // Use filled arguments if available, otherwise use original
    let arguments_to_use: &[CallArgument] = match &filled_arguments {
        Some(filled) => filled.as_slice(),
        None => arguments,
    };

    // Lower arguments
    let arg_values: Vec<Value> = arguments_to_use
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Extract argument types for determining Copy vs Move passing modes
    let arg_types: Vec<&Ty> = arguments_to_use.iter().map(|arg| &arg.value.ty).collect();

    // Helper to get ordered type args from a symbol's type parameters
    let get_ordered_type_args =
        |ctx: &mut LoweringContext,
         sym: &std::sync::Arc<dyn Symbol<kestrel_semantic_tree::language::KestrelLanguage>>,
         callable: Option<&CallableBehavior>,
         call_arg_types: &[&Ty],
         is_instance_method: bool,
         receiver_ty: Option<&Ty>|
         -> Option<Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>>> {
            use kestrel_semantic_tree::symbol::EnumSymbol;
            use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
            use kestrel_semantic_tree::symbol::r#struct::StructSymbol;
            use semantic_tree::symbol::SymbolId;

            // Try to get type parameters from different symbol types
            let mut method_param_ids: Vec<SymbolId> = Vec::new();
            let param_ids: Option<Vec<SymbolId>> = if let Some(func_sym) =
                sym.as_ref().downcast_ref::<FunctionSymbol>()
            {
                // For methods on generic structs/enums, we need BOTH parent type params
                // AND the function's own type params (in that order, matching how MIR
                // functions are lowered in lowerer/function.rs)
                let mut all_params: Vec<SymbolId> = Vec::new();

                // First, collect parent type parameters (from struct/enum/extension)
                if let Some(parent) = func_sym.metadata().parent() {
                    if let Some(struct_sym) = parent.as_ref().downcast_ref::<StructSymbol>() {
                        for tp in struct_sym.type_parameters() {
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    } else if let Some(enum_sym) = parent.as_ref().downcast_ref::<EnumSymbol>() {
                        for tp in enum_sym.type_parameters() {
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    } else if let Some(ext_sym) = parent.as_ref().downcast_ref::<ExtensionSymbol>() {
                        let is_protocol_extension = ext_sym
                            .target_type()
                            .as_ref()
                            .is_some_and(|ty| is_protocol_type(ty));
                        for tp in ext_sym.referenced_type_parameters() {
                            if is_protocol_extension && tp.metadata().name().value == "Self" {
                                continue;
                            }
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    }
                }

                // Then, collect function's own type parameters
                for tp in func_sym.type_parameters() {
                    let tp_id = Symbol::metadata(tp.as_ref()).id();
                    all_params.push(tp_id);
                    method_param_ids.push(tp_id);
                }

                Some(all_params)
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
            } else if let Some(init_sym) = sym.as_ref().downcast_ref::<InitializerSymbol>() {
                // Initializers inherit type parameters from their parent struct/enum/extension
                let mut all_params: Vec<SymbolId> = Vec::new();
                if let Some(parent) = sym.metadata().parent() {
                    if let Some(struct_sym) = parent.as_ref().downcast_ref::<StructSymbol>() {
                        for tp in struct_sym.type_parameters() {
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    } else if let Some(enum_sym) = parent.as_ref().downcast_ref::<EnumSymbol>() {
                        for tp in enum_sym.type_parameters() {
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    } else if let Some(ext_sym) = parent.as_ref().downcast_ref::<ExtensionSymbol>() {
                        let is_protocol_extension = ext_sym
                            .target_type()
                            .as_ref()
                            .is_some_and(|ty| is_protocol_type(ty));
                        for tp in ext_sym.referenced_type_parameters() {
                            if is_protocol_extension && tp.metadata().name().value == "Self" {
                                continue;
                            }
                            all_params.push(Symbol::metadata(tp.as_ref()).id());
                        }
                    }
                }
                // Then, collect initializer's own type parameters
                for tp in init_sym.type_parameters() {
                    let tp_id = Symbol::metadata(tp.as_ref()).id();
                    all_params.push(tp_id);
                    method_param_ids.push(tp_id);
                }
                Some(all_params)
            } else {
                // For other symbols without type_parameters, use the fallback
                None
            };

            let mut effective_subs = if !method_param_ids.is_empty() && callable.is_some() {
                infer_type_param_substitutions_from_call(
                    substitutions,
                    callable.unwrap(),
                    call_arg_types,
                    is_instance_method,
                    receiver_ty,
                    &method_param_ids,
                )
            } else {
                substitutions.clone()
            };

            if let Some(ids) = param_ids {
                // If the extension introduced a synthetic `Self` type parameter,
                // map it to the receiver type (or SelfType for protocol receivers).
                if let Some(self_ty) = receiver_ty {
                    let self_sub =
                        if is_protocol_type(self_ty) || matches!(self_ty.kind(), TyKind::SelfType)
                        {
                        Ty::self_type(self_ty.span().clone())
                    } else {
                        self_ty.clone()
                    };
                    for param_id in &ids {
                        if effective_subs.contains(*param_id) {
                            continue;
                        }
                        if let Some(sym) = ctx.model.query(SymbolFor { id: *param_id })
                            && sym.metadata().name().value == "Self"
                        {
                            effective_subs.insert(*param_id, self_sub.clone());
                        }
                    }
                }

                let mut filtered_subs = Substitutions::new();
                for param_id in &ids {
                    if let Some(sub_ty) = effective_subs.get(*param_id) {
                        filtered_subs.insert(*param_id, sub_ty.clone());
                    }
                }
                effective_subs = filtered_subs;

                if ids.is_empty() {
                    if effective_subs.is_empty() {
                        return Some(Vec::new());
                    }
                    ctx.emit_error(LoweringError::internal(
                        format!(
                            "missing type arguments for generic call to {}",
                            sym.metadata().name().value
                        ),
                        Some(expr.span.clone()),
                    ));
                    return None;
                }
                if let Some(ordered_types) = effective_subs.types_in_order(&ids) {
                    return Some(
                        ordered_types
                            .into_iter()
                            .map(|ty| lower_type(ctx, ty))
                            .collect(),
                    );
                }
                ctx.emit_error(LoweringError::internal(
                    format!(
                        "missing type arguments for generic call to {}",
                        sym.metadata().name().value
                    ),
                    Some(expr.span.clone()),
                ));
                return None;
            }
            if effective_subs.is_empty() {
                return Some(Vec::new());
            }
            ctx.emit_error(LoweringError::internal(
                format!(
                    "missing type parameter order for generic call to {}",
                    sym.metadata().name().value
                ),
                Some(expr.span.clone()),
            ));
            None
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
                        if let Some(ref beh) = callable_beh {
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
                                    // init() has no method-level type parameters
                                    let mir_callee =
                                        Callee::witness(protocol_name, "init", for_type, vec![]);
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
                            let type_args = match get_ordered_type_args(
                                ctx,
                                &sym,
                                callable_beh.as_deref(),
                                &arg_types,
                                false,
                                None,
                            ) {
                                Some(args) => args,
                                None => return Value::Immediate(Immediate::error()),
                            };
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
                        let callable_beh = sym.metadata().get_behavior::<CallableBehavior>();
                        let type_args = match get_ordered_type_args(
                            ctx,
                            &sym,
                            callable_beh.as_deref(),
                            &arg_types,
                            false,
                            None,
                        ) {
                            Some(args) => args,
                            None => return Value::Immediate(Immediate::error()),
                        };
                        let mir_callee = if type_args.is_empty() {
                            Callee::direct(func_name)
                        } else {
                            Callee::direct_generic(func_name, type_args)
                        };

                        // Look up CallableBehavior to get parameter access modes
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
                },
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("symbol not found for call: {:?}", symbol_id),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                },
            }
        },

        ExprKind::EnumCase { case_id } => {
            // Enum case with associated values (e.g., .Success(name: "...", potency: 100))
            let symbol = ctx.model.query(SymbolFor { id: *case_id });
            match symbol {
                Some(sym) => {
                    let variant_name = sym.metadata().name().value.clone();

                    ctx.emit_assign(
                        result_place.clone(),
                        Rvalue::EnumVariant {
                            enum_ty: result_ty,
                            variant: variant_name,
                            payload: arg_values,
                        },
                    );
                },
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("enum case symbol not found: {:?}", case_id),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                },
            }
        },

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

            // Check if this is a call on Self type in a protocol context (needs witness method lookup)
            let is_self_type_call = matches!(receiver.ty.kind(), TyKind::SelfType);

            // Check if this is a call on a protocol type (protocol extension methods)
            // When inside a protocol extension, `self` has type `Protocol` which also needs witness dispatch
            let is_protocol_type_call = is_protocol_type(&receiver.ty);

            // Check if this is a static call on a concrete type (Type.staticMethod())
            // This happens when deferred static calls are resolved during type inference
            let is_static_type_ref_call = matches!(receiver.kind, ExprKind::TypeRef(_));

            // Determine if this is an instance method call (has receiver value)
            let is_instance = !(is_static_type_param_call
                || is_static_assoc_type_call
                || is_static_type_ref_call);

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

                        use kestrel_semantic_tree::behavior::implements::ImplementsBehavior;

                        // Find the protocol that defines this method.
                        // Priority: 1) ImplementsBehavior, 2) Protocol parent, 3) Extension conformances
                        let protocol_symbol = if let Some(implements) =
                            sym.metadata().get_behavior::<ImplementsBehavior>()
                        {
                            // Method explicitly implements a protocol method
                            ctx.model.query(SymbolFor {
                                id: implements.protocol(),
                            })
                        } else if let Some(parent) = sym.metadata().parent() {
                            if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                                // Method is defined directly in a protocol
                                Some(parent)
                            } else if parent.metadata().kind() == KestrelSymbolKind::Extension {
                                // Method is in an extension - find which protocol conformance it belongs to
                                find_protocol_for_extension_method(&parent, method_name)
                            } else {
                                None
                            }
                        } else {
                            None
                        };

                        // Check if receiver is a builtin primitive type (these use primitive methods, not witnesses)
                        let is_builtin_type = matches!(
                            receiver.ty.kind(),
                            TyKind::Int(_) | TyKind::Float(_) | TyKind::Bool | TyKind::String
                        );

                        // Check if this requires witness dispatch:
                        // - Only protocol methods (protocol_symbol.is_some()) can be dispatched via witnesses.
                        // - For protocol-typed receivers calling extension-only methods, use direct calls.
                        let needs_witness_dispatch = protocol_symbol.is_some()
                            && ((is_type_param_call
                                || is_static_type_param_call
                                || is_assoc_type_call
                                || is_static_assoc_type_call
                                || is_self_type_call
                                || is_protocol_type_call)
                                || !is_builtin_type);

                        if needs_witness_dispatch {
                            if let Some(protocol_sym) = protocol_symbol {
                                let protocol_name = qualified_name_for_symbol(ctx, &protocol_sym);
                                // For protocol type calls (inside protocol extensions), use Self type
                                // which will be substituted with the concrete type at monomorphization
                                let for_type = if is_protocol_type_call {
                                    ctx.mir.ty_self()
                                } else {
                                    lower_type(ctx, &receiver.ty)
                                };

                                // Extract the method's own type arguments (not parent type args).
                                // For example, in `Hash.hash[H]`, we need to extract the concrete type for `H`.
                                let method_type_args: Vec<_> = if let Some(func_sym) =
                                    sym.as_ref().downcast_ref::<FunctionSymbol>()
                                {
                                    let method_param_ids: Vec<_> = func_sym
                                        .type_parameters()
                                        .iter()
                                        .map(|tp| Symbol::metadata(tp.as_ref()).id())
                                        .collect();

                                    if method_param_ids.is_empty() {
                                        vec![]
                                    } else {
                                        let effective_subs = if let Some(callable) =
                                            callable_beh.as_deref()
                                        {
                                            infer_type_param_substitutions_from_call(
                                                substitutions,
                                                callable,
                                                &all_arg_types,
                                                is_instance,
                                                Some(&receiver.ty),
                                                &method_param_ids,
                                            )
                                        } else {
                                            substitutions.clone()
                                        };

                                        if let Some(ordered_types) =
                                            effective_subs.types_in_order(&method_param_ids)
                                        {
                                            ordered_types
                                                .into_iter()
                                                .map(|ty| lower_type(ctx, ty))
                                                .collect()
                                        } else {
                                            vec![]
                                        }
                                    }
                                } else {
                                    vec![]
                                };

                                let mir_callee = Callee::witness(
                                    protocol_name,
                                    method_name.clone(),
                                    for_type,
                                    method_type_args,
                                );
                                ctx.emit_call_with_modes(
                                    result_place.clone(),
                                    mir_callee,
                                    call_args,
                                );
                            } else {
                                // Receiver type requires witness dispatch but method doesn't implement a protocol - shouldn't happen
                                ctx.emit_error(LoweringError::internal(
                                    format!(
                                        "method '{}' on generic/protocol type doesn't implement a protocol method",
                                        method_name
                                    ),
                                    Some(expr.span.clone()),
                                ));
                                return Value::Immediate(Immediate::error());
                            }
                        } else {
                            // Regular direct method call (concrete type, non-protocol method)
                            let func_name = qualified_name_for_symbol(ctx, &sym);
                            let type_args = match get_ordered_type_args(
                                ctx,
                                &sym,
                                callable_beh.as_deref(),
                                &all_arg_types,
                                is_instance,
                                Some(&receiver.ty),
                            ) {
                                Some(args) => args,
                                None => return Value::Immediate(Immediate::error()),
                            };
                            let mir_callee = if type_args.is_empty() {
                                Callee::direct(func_name)
                            } else {
                                Callee::direct_generic(func_name, type_args)
                            };
                            ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
                        }
                    },
                    None => {
                        ctx.emit_error(LoweringError::internal(
                            format!("method symbol not found for '{}'", method_name),
                            Some(expr.span.clone()),
                        ));
                        return Value::Immediate(Immediate::error());
                    },
                }
            } else {
                ctx.emit_error(LoweringError::internal(
                    format!("no method candidates for '{}'", method_name),
                    Some(expr.span.clone()),
                ));
                return Value::Immediate(Immediate::error());
            }
        },

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
                    // Try to find the initializer symbol to get its CallableBehavior
                    // Look for an "init" child of the type symbol
                    let init_sym = sym
                        .metadata()
                        .children()
                        .iter()
                        .find(|child| {
                            child.metadata().kind() == KestrelSymbolKind::Initializer
                                && child.metadata().name().value == "init"
                        })
                        .cloned();

                    let init_beh = init_sym
                        .as_ref()
                        .and_then(|s| s.metadata().get_behavior::<CallableBehavior>());

                    // Build the init function name with labels
                    // Initializers are named: init$label1$label2 (or just "init" if no labels)
                    let mut name_parts = Vec::new();
                    collect_symbol_name_parts(&sym, &mut name_parts);

                    let init_name_part = if let Some(beh) = &init_beh {
                        let labels: Vec<&str> = beh
                            .parameters()
                            .iter()
                            .filter_map(|p| p.external_label())
                            .collect();
                        if labels.is_empty() {
                            "init".to_string()
                        } else {
                            format!("init${}", labels.join("$"))
                        }
                    } else {
                        "init".to_string()
                    };
                    name_parts.push(init_name_part);

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
                    let type_args = match get_ordered_type_args(
                        ctx,
                        &sym,
                        init_beh.as_deref(),
                        &arg_types,
                        false,
                        None,
                    ) {
                        Some(args) => args,
                        None => return Value::Immediate(Immediate::error()),
                    };
                    let mir_callee = if type_args.is_empty() {
                        Callee::direct(init_name)
                    } else {
                        Callee::direct_generic(init_name, type_args)
                    };
                    mark_moved_args(ctx, &call_args);
                    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

                    // result_place now contains the initialized struct
                    // (init wrote to it via the self_ref)
                },
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!(
                            "type symbol not found for initializer call: {:?}",
                            symbol_id
                        ),
                        Some(expr.span.clone()),
                    ));
                    return Value::Immediate(Immediate::error());
                },
            }
        },

        ExprKind::LocalRef(local_id) => {
            // Indirect call through a local variable (closure)
            let mir_local = ctx.get_local_unwrap(*local_id);
            let callee_place = Place::local(mir_local);

            // Build call args with proper reference creation for indirect calls
            let call_args = build_indirect_call_args(ctx, arg_values, &arg_types, &callee.ty);
            mark_moved_args(ctx, &call_args);

            // Closures are "thick" callables
            ctx.emit_call_with_modes(result_place.clone(), Callee::Thick(callee_place), call_args);
        },

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
                },
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::unsupported_expr(
                        "indirect call on immediate value",
                        expr.span.clone(),
                    ));
                    return Value::Immediate(Immediate::error());
                },
                Value::Unreachable => {
                    return Value::Unreachable;
                },
            }
        },
    }

    Value::Place(result_place)
}

/// Lower a subscript call: `array(0)`, `dict(key: "foo")`.
///
/// Subscript calls have a receiver (the collection), a getter function to call,
/// and arguments (the index/key).
fn lower_subscript_call(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    getter_id: SymbolId,
    arguments: &[CallArgument],
    expr: &Expression,
) -> Value {
    use kestrel_semantic_tree::behavior::callable::CallableBehavior;

    // Fill in default arguments if any are missing
    // Subscripts don't have generic type parameters, so use empty substitutions
    let empty_subs = Substitutions::new();
    let filled_arguments: Option<Vec<CallArgument>> =
        fill_default_arguments(ctx, arguments, getter_id, &empty_subs);

    // Use filled arguments if available, otherwise use original
    let arguments_to_use: &[CallArgument] = match &filled_arguments {
        Some(filled) => filled.as_slice(),
        None => arguments,
    };

    // Lower the receiver expression (e.g., the array)
    let receiver_value = lower_expression(ctx, receiver);

    // Lower arguments (e.g., the index)
    let arg_values: Vec<Value> = arguments_to_use
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Build the full argument list: receiver first, then subscript arguments
    let mut all_args = vec![receiver_value];
    all_args.extend(arg_values);

    // Build argument types list
    let mut all_arg_types: Vec<&Ty> = vec![&receiver.ty];
    all_arg_types.extend(arguments_to_use.iter().map(|arg| &arg.value.ty));

    // Get the result type and create a temp for it
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("subscript", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if needed
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Get the getter symbol to build the call
    let getter_symbol = ctx.model.query(SymbolFor { id: getter_id });
    match getter_symbol {
        Some(sym) => {
            // Get the callable behavior for access mode info
            let callable_beh = sym.metadata().get_behavior::<CallableBehavior>();

            // Build the call arguments with proper access modes
            let call_args = build_call_args(
                ctx,
                all_args,
                &all_arg_types,
                callable_beh.as_deref(),
                true, // has_receiver = true for instance subscripts
            );

            // Mark moved arguments
            mark_moved_args(ctx, &call_args);

            // Get the qualified name of the getter
            let func_name = qualified_name_for_symbol(ctx, &sym);

            // Build type arguments from receiver's substitutions (for generic containing types)
            let mut type_args =
                match get_type_args_for_receiver(ctx, &receiver.ty, Some(expr.span.clone())) {
                    Some(args) => args,
                    None => return Value::Immediate(Immediate::error()),
                };

            // Add type arguments for subscript's own type parameters (e.g., subscript[F](value: F))
            // We infer these from the argument types by matching parameter types to argument types
            if let Ok(getter_sym) = sym.clone().downcast_arc::<GetterSymbol>() {

                let subscript_type_params = get_subscript_type_parameters(&getter_sym);
                if !subscript_type_params.is_empty() {
                    // Get the callable behavior to find parameter types
                    if let Some(callable) = sym.metadata().get_behavior::<CallableBehavior>() {
                        let params = callable.parameters();
                        // For each subscript type param, find which parameter uses it and get the arg's type
                        for type_param in &subscript_type_params {
                            let type_param_id = type_param.metadata().id();
                            // Find the parameter whose type is this type parameter
                            for (param_idx, param) in params.iter().enumerate() {
                                if let TyKind::TypeParameter(param_tp) = param.ty.kind() {
                                    if param_tp.metadata().id() == type_param_id {
                                        // Found it! Use the corresponding argument's type
                                        if let Some(arg) = arguments.get(param_idx) {
                                            let arg_mir_ty = lower_type(ctx, &arg.value.ty);
                                            type_args.push(arg_mir_ty);
                                        }
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Create the callee
            let mir_callee = if type_args.is_empty() {
                Callee::direct(func_name)
            } else {
                Callee::direct_generic(func_name, type_args)
            };

            // Emit the call
            ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
        },
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("subscript getter symbol not found: {:?}", getter_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    }

    Value::Place(result_place)
}

/// Get type arguments from a receiver type for generic subscript calls.
fn get_type_args_for_receiver(
    ctx: &mut LoweringContext,
    receiver_ty: &Ty,
    span: Option<kestrel_span::Span>,
) -> Option<Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>>> {
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use semantic_tree::symbol::Symbol;

    let (type_params, substitutions) = match receiver_ty.kind() {
        TyKind::Struct {
            symbol,
            substitutions,
        } => {
            let params: Vec<_> =
                if let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>() {
                    generics
                        .type_parameters()
                        .iter()
                        .map(|p| p.metadata().id())
                        .collect()
                } else {
                    vec![]
                };
            (params, substitutions)
        },
        TyKind::Enum {
            symbol,
            substitutions,
        } => {
            let params: Vec<_> =
                if let Some(generics) = symbol.metadata().get_behavior::<GenericsBehavior>() {
                    generics
                        .type_parameters()
                        .iter()
                        .map(|p| p.metadata().id())
                        .collect()
                } else {
                    vec![]
                };
            (params, substitutions)
        },
        _ => return Some(Vec::new()),
    };

    if type_params.is_empty() {
        if substitutions.is_empty() {
            return Some(Vec::new());
        }
        ctx.emit_error(LoweringError::internal(
            "missing type parameter order for generic receiver".to_string(),
            span,
        ));
        return None;
    }

    if let Some(ordered_types) = substitutions.types_in_order(&type_params) {
        Some(
            ordered_types
                .into_iter()
                .map(|ty| lower_type(ctx, ty))
                .collect(),
        )
    } else {
        ctx.emit_error(LoweringError::internal(
            "missing type arguments for generic receiver".to_string(),
            span,
        ));
        None
    }
}

/// Lower a subscript setter call: `array(0) = value`.
///
/// This finds the setter function from the subscript symbol and generates a call
/// with the receiver, index arguments, and new value.
fn lower_subscript_setter_call(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    getter_id: SymbolId,
    arguments: &[CallArgument],
    new_value: &Expression,
    expr: &Expression,
) -> Value {
    use kestrel_semantic_tree::behavior::callable::CallableBehavior;
    use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;

    // The getter_id is the getter symbol. We need to find the parent subscript
    // and get the setter from it.
    let getter_symbol = match ctx.model.query(SymbolFor { id: getter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("subscript getter symbol not found: {:?}", getter_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the parent subscript symbol
    let subscript = match getter_symbol.metadata().parent() {
        Some(parent) if parent.metadata().kind() == KestrelSymbolKind::Subscript => parent,
        _ => {
            ctx.emit_error(LoweringError::internal(
                "subscript getter has no subscript parent",
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the subscript symbol to find the setter
    let subscript_sym = match subscript.as_ref().downcast_ref::<SubscriptSymbol>() {
        Some(s) => s,
        None => {
            ctx.emit_error(LoweringError::internal(
                "parent is not a SubscriptSymbol",
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the setter ID
    let setter_id = match subscript_sym.setter_id() {
        Some(id) => id,
        None => {
            ctx.emit_error(LoweringError::internal(
                "subscript has no setter (read-only)",
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the setter symbol
    let setter_symbol = match ctx.model.query(SymbolFor { id: setter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("subscript setter symbol not found: {:?}", setter_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Lower the receiver
    let receiver_value = lower_expression(ctx, receiver);

    // Lower the subscript arguments (e.g., the index)
    let arg_values: Vec<Value> = arguments
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Lower the new value
    let new_value_result = lower_expression(ctx, new_value);

    // Build the full argument list: receiver first, then subscript arguments, then new value
    let mut all_args = vec![receiver_value];
    all_args.extend(arg_values);
    all_args.push(new_value_result);

    // Build argument types list
    let mut all_arg_types: Vec<&Ty> = vec![&receiver.ty];
    all_arg_types.extend(arguments.iter().map(|arg| &arg.value.ty));
    all_arg_types.push(&new_value.ty);

    // Setters return unit
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("subscript_set", unit_ty);
    let unit_place = Place::local(unit_local);

    // Get the callable behavior for access mode info
    let callable_beh = setter_symbol.metadata().get_behavior::<CallableBehavior>();

    // Build the call arguments with proper access modes
    let call_args = build_call_args(
        ctx,
        all_args,
        &all_arg_types,
        callable_beh.as_deref(),
        true, // has_receiver = true for instance subscripts
    );

    // Mark moved arguments
    mark_moved_args(ctx, &call_args);

    // Get the qualified name of the setter
    let setter_name = qualified_name_for_symbol(ctx, &setter_symbol);

    // Build type arguments from receiver's substitutions
    let type_args = match get_type_args_for_receiver(ctx, &receiver.ty, Some(expr.span.clone())) {
        Some(args) => args,
        None => return Value::Immediate(Immediate::error()),
    };

    // Create the callee
    let mir_callee = if type_args.is_empty() {
        Callee::direct(setter_name)
    } else {
        Callee::direct_generic(setter_name, type_args)
    };

    // Emit the call
    ctx.emit_call_with_modes(unit_place, mir_callee, call_args);

    // Setter returns unit
    Value::Immediate(Immediate::unit())
}

/// Find field information from a type.
/// Returns (field_id, is_computed) if the field is found.
fn find_field_info(ctx: &LoweringContext, ty: &Ty, field_name: &str) -> Option<(SymbolId, bool)> {
    use semantic_tree::symbol::Symbol;

    // Try to get the struct symbol from the type
    if let Some(struct_sym) = ty.as_struct() {
        // Query fields from the struct symbol
        let struct_id = struct_sym.metadata().id();
        let fields = ctx.model.query(StructFields { struct_id });
        for field_info in fields {
            if field_info.name == field_name {
                return Some((field_info.field_id, field_info.is_computed));
            }
        }
    }

    // Try enum type - enums can also have computed properties
    if let Some(enum_sym) = ty.as_enum() {
        // Look through children for fields
        for child in enum_sym.metadata().children() {
            if child.metadata().kind() == KestrelSymbolKind::Field
                && child.metadata().name().value == field_name
            {
                // Check if this field is computed
                if let Ok(field_sym) = child.clone().downcast_arc::<FieldSymbol>() {
                    return Some((field_sym.metadata().id(), field_sym.is_computed()));
                }
            }
        }
    }

    None
}

/// Lower a getter call for a computed property.
fn lower_getter_call(
    ctx: &mut LoweringContext,
    object: &Expression,
    field_id: SymbolId,
    field_name: &str,
    expr: &Expression,
) -> Value {
    // Get the field symbol to find the getter
    let field_symbol = match ctx.model.query(SymbolFor { id: field_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("field symbol not found: {:?}", field_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Downcast to FieldSymbol to get getter
    let field_sym: std::sync::Arc<FieldSymbol> = match field_symbol.clone().downcast_arc() {
        Ok(f) => f,
        Err(_) => {
            ctx.emit_error(LoweringError::internal(
                format!("could not downcast to FieldSymbol: {}", field_name),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the getter symbol ID
    let getter_id = match field_sym.getter() {
        Some(id) => id,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("computed property '{}' has no getter", field_name),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the getter symbol
    let getter_symbol = match ctx.model.query(SymbolFor { id: getter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("getter symbol not found for '{}'", field_name),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Check if this is a static computed property
    let is_static = field_sym.is_static();

    // Get the result type and create a temp for the result
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("getter_result", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if needed
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Build the qualified name for the getter
    let getter_name = qualified_name_for_symbol(ctx, &getter_symbol);

    // Get type arguments from the receiver's type for generic types
    let type_args = match extract_type_args_from_receiver(ctx, &object.ty, Some(expr.span.clone()))
    {
        Some(args) => args,
        None => return Value::Immediate(Immediate::error()),
    };

    // Look up CallableBehavior to get receiver access mode
    let callable_beh = getter_symbol.metadata().get_behavior::<CallableBehavior>();

    if is_static {
        // Static computed property - no receiver
        let mir_callee = if type_args.is_empty() {
            Callee::direct(getter_name)
        } else {
            Callee::direct_generic(getter_name, type_args)
        };
        ctx.emit_call_with_modes(result_place.clone(), mir_callee, vec![]);
    } else {
        // Instance computed property - pass receiver
        let receiver_value = lower_expression(ctx, object);

        // Build call args with receiver
        let call_args = if let Some(beh) = callable_beh {
            match beh.receiver() {
                Some(ReceiverKind::Borrowing) | None => {
                    // Getter borrows self
                    let ref_value = create_ref(ctx, &receiver_value, &object.ty, false);
                    vec![CallArg::new(ref_value, PassingMode::Copy)]
                },
                Some(ReceiverKind::Mutating) => {
                    // Getter needs mutable self (unusual but possible)
                    let ref_value = create_ref(ctx, &receiver_value, &object.ty, true);
                    vec![CallArg::new(ref_value, PassingMode::Copy)]
                },
                Some(ReceiverKind::Consuming) => {
                    // Getter consumes self
                    let mode = if object.ty.is_copyable() {
                        PassingMode::Copy
                    } else {
                        PassingMode::Move
                    };
                    vec![CallArg::new(receiver_value, mode)]
                },
                Some(ReceiverKind::Initializing) => {
                    // Shouldn't happen for getter
                    vec![CallArg::mutating(receiver_value)]
                },
            }
        } else {
            // Default: borrow receiver
            let ref_value = create_ref(ctx, &receiver_value, &object.ty, false);
            vec![CallArg::new(ref_value, PassingMode::Copy)]
        };

        mark_moved_args(ctx, &call_args);
        let mir_callee = if type_args.is_empty() {
            Callee::direct(getter_name)
        } else {
            Callee::direct_generic(getter_name, type_args)
        };
        ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
    }

    Value::Place(result_place)
}

/// Lower a protocol property access through witness dispatch.
///
/// This is used when accessing a computed property on a type parameter through protocol bounds.
/// For example: `T.defaultValue` or `item.value` where T/item conforms to a protocol with
/// that property requirement.
fn lower_protocol_property_access(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    property_name: &str,
    protocol_id: SymbolId,
    is_static: bool,
    expr: &Expression,
) -> Value {
    // Get the protocol symbol for the qualified name
    let protocol_symbol = match ctx.model.query(SymbolFor { id: protocol_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("protocol symbol not found: {:?}", protocol_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };
    let protocol_name = qualified_name_for_symbol(ctx, &protocol_symbol);

    // Get the result type and create a temp for the result
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("getter_result", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if needed
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Build the getter method name for witness lookup
    let getter_method_name = format!("get:{}", property_name);

    // Lower the receiver type for witness dispatch
    let for_type = lower_type(ctx, &receiver.ty);

    if is_static {
        // Static property access on type parameter: T.property
        // No receiver value, just call the witness getter
        let mir_callee = Callee::witness(protocol_name, getter_method_name, for_type, vec![]);
        ctx.emit_call_with_modes(result_place.clone(), mir_callee, vec![]);
    } else {
        // Instance property access on type parameter: item.property where item: T
        // Pass receiver as argument (borrowed)
        let receiver_value = lower_expression(ctx, receiver);

        // Getters typically borrow self
        let ref_value = create_ref(ctx, &receiver_value, &receiver.ty, false);
        let call_args = vec![CallArg::new(ref_value, PassingMode::Copy)];

        mark_moved_args(ctx, &call_args);
        let mir_callee = Callee::witness(protocol_name, getter_method_name, for_type, vec![]);
        ctx.emit_call_with_modes(result_place.clone(), mir_callee, call_args);
    }

    Value::Place(result_place)
}

/// Lower a protocol property setter through witness dispatch.
///
/// This is used when assigning to a computed property on a type parameter through protocol bounds.
/// For example: `T.count = 5` or `item.value = 10` where T/item conforms to a protocol with
/// that property requirement.
fn lower_protocol_property_setter(
    ctx: &mut LoweringContext,
    receiver: &Expression,
    property_name: &str,
    protocol_id: SymbolId,
    is_static: bool,
    value: &Expression,
    expr: &Expression,
) -> Value {
    // Get the protocol symbol for the qualified name
    let protocol_symbol = match ctx.model.query(SymbolFor { id: protocol_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("protocol symbol not found: {:?}", protocol_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };
    let protocol_name = qualified_name_for_symbol(ctx, &protocol_symbol);

    // Build the setter method name for witness lookup
    let setter_method_name = format!("set:{}", property_name);

    // Lower the receiver type for witness dispatch
    let for_type = lower_type(ctx, &receiver.ty);

    // Lower the value to be set
    let rhs_value = lower_expression(ctx, value);

    // Build call args: value is the "newValue" parameter (consuming)
    // Setters return unit, so we need a unit result place
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("setter_result", unit_ty);
    let unit_place = Place::local(unit_local);

    if is_static {
        // Static property setter: T.property = value
        // Only pass the newValue argument
        let call_args = if value.ty.is_copyable() {
            vec![CallArg::new(rhs_value, PassingMode::Copy)]
        } else {
            vec![CallArg::new(rhs_value, PassingMode::Move)]
        };

        mark_moved_args(ctx, &call_args);
        let mir_callee = Callee::witness(protocol_name, setter_method_name, for_type, vec![]);
        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
    } else {
        // Instance property setter: item.property = value
        // Pass receiver (mutating) and newValue
        let receiver_value = lower_expression(ctx, receiver);

        // Setter needs mutable reference to self
        let ref_value = create_ref(ctx, &receiver_value, &receiver.ty, true);

        // Build call args: mutable self reference + newValue
        let value_arg = if value.ty.is_copyable() {
            CallArg::new(rhs_value, PassingMode::Copy)
        } else {
            CallArg::new(rhs_value, PassingMode::Move)
        };
        let call_args = vec![
            CallArg::new(ref_value, PassingMode::Copy), // self: &var Self
            value_arg,                                  // newValue: T
        ];

        mark_moved_args(ctx, &call_args);
        let mir_callee = Callee::witness(protocol_name, setter_method_name, for_type, vec![]);
        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
    }

    // Assignment returns unit
    Value::Immediate(Immediate::unit())
}

/// Lower a static getter call for a module-level or static computed property.
///
/// This is used when a computed property is accessed directly by name (e.g., `globalComputedVar`)
/// rather than through a type (e.g., `Foo.staticComputedVar`).
fn lower_static_getter_call(
    ctx: &mut LoweringContext,
    getter_id: SymbolId,
    expr: &Expression,
) -> Value {
    // Get the getter symbol
    let getter_symbol = match ctx.model.query(SymbolFor { id: getter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("getter symbol not found: {:?}", getter_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the result type and create a temp for the result
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("getter_result", result_ty);
    let result_place = Place::local(result_local);

    // Track the temp for deinit if needed
    if ctx.type_needs_deinit(&expr.ty) {
        ctx.track_statement_temp(result_local);
    }

    // Build the qualified name for the getter
    let getter_name = qualified_name_for_symbol(ctx, &getter_symbol);

    // Static computed property - no receiver, no type args (module-level fields aren't generic)
    let mir_callee = Callee::direct(getter_name);
    ctx.emit_call_with_modes(result_place.clone(), mir_callee, vec![]);

    Value::Place(result_place)
}

/// Lower a static setter call for a module-level or static computed property.
///
/// This is used when a computed property is assigned directly by name (e.g., `globalComputedVar = 2`)
/// rather than through a type (e.g., `Foo.staticComputedVar = 2`).
fn lower_static_setter_call(
    ctx: &mut LoweringContext,
    setter_id: SymbolId,
    value: &Expression,
    expr: &Expression,
) -> Value {
    // Get the setter symbol
    let setter_symbol = match ctx.model.query(SymbolFor { id: setter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("setter symbol not found: {:?}", setter_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Build the qualified name for the setter
    let setter_name = qualified_name_for_symbol(ctx, &setter_symbol);

    // Lower the value to be set
    let rhs_value = lower_expression(ctx, value);

    // Build the call argument for the value (newValue parameter)
    let passing_mode = if value.ty.is_copyable() {
        PassingMode::Copy
    } else {
        PassingMode::Move
    };
    let call_arg = CallArg::new(rhs_value, passing_mode);

    // Create a dummy result place (setters return unit)
    let unit_ty = ctx.mir.ty_unit();
    let result_local = ctx.create_temp("setter_result", unit_ty);
    let result_place = Place::local(result_local);

    // Static computed property - no receiver, just the newValue argument
    let mir_callee = Callee::direct(setter_name);
    ctx.emit_call_with_modes(result_place, mir_callee, vec![call_arg]);

    // Assignment expression yields unit
    Value::Immediate(Immediate::unit())
}

/// Extract type arguments from a receiver's type.
///
/// For generic types like `Slice<Int>`, this extracts the type arguments `[Int]`
/// in the order the type parameters are declared on the type.
fn extract_type_args_from_receiver(
    ctx: &mut LoweringContext,
    receiver_ty: &Ty,
    span: Option<kestrel_span::Span>,
) -> Option<Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>>> {
    use kestrel_semantic_tree::behavior::generics::GenericsBehavior;
    use semantic_tree::symbol::Symbol;

    // Get the type's substitutions and the parent type's parameter order
    let (type_params, substitutions) =
        if let Some((struct_sym, subs)) = receiver_ty.as_struct_with_subs() {
            // Get type parameters from struct
            let params: Vec<_> =
                if let Some(generics) = struct_sym.metadata().get_behavior::<GenericsBehavior>() {
                    generics
                        .type_parameters()
                        .iter()
                        .map(|p| p.metadata().id())
                        .collect()
                } else {
                    vec![]
                };
            (params, subs)
        } else if let Some((enum_sym, subs)) = receiver_ty.as_enum_with_subs() {
            // Get type parameters from enum
            let params: Vec<_> =
                if let Some(generics) = enum_sym.metadata().get_behavior::<GenericsBehavior>() {
                    generics
                        .type_parameters()
                        .iter()
                        .map(|p| p.metadata().id())
                        .collect()
                } else {
                    vec![]
                };
            (params, subs)
        } else {
            return Some(vec![]);
        };

    // Get types in the correct order based on type parameter declaration order
    if type_params.is_empty() {
        if substitutions.is_empty() {
            return Some(vec![]);
        }
        ctx.emit_error(LoweringError::internal(
            "missing type parameter order for generic receiver".to_string(),
            span,
        ));
        return None;
    }

    if let Some(ordered_types) = substitutions.types_in_order(&type_params) {
        Some(
            ordered_types
                .into_iter()
                .map(|ty| lower_type(ctx, ty))
                .collect(),
        )
    } else {
        ctx.emit_error(LoweringError::internal(
            "missing type arguments for generic receiver".to_string(),
            span,
        ));
        None
    }
}

/// Lower a setter call for a computed property assignment.
fn lower_setter_call(
    ctx: &mut LoweringContext,
    object: &Expression,
    field_id: SymbolId,
    field_name: &str,
    rhs: &Expression,
    expr: &Expression,
) -> Value {
    // Get the field symbol to find the setter
    let field_symbol = match ctx.model.query(SymbolFor { id: field_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("field symbol not found: {:?}", field_id),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Downcast to FieldSymbol to get setter
    let field_sym: std::sync::Arc<FieldSymbol> = match field_symbol.clone().downcast_arc() {
        Ok(f) => f,
        Err(_) => {
            ctx.emit_error(LoweringError::internal(
                format!("could not downcast to FieldSymbol: {}", field_name),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the setter symbol ID
    let setter_id = match field_sym.setter() {
        Some(id) => id,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!(
                    "computed property '{}' has no setter (read-only)",
                    field_name
                ),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Get the setter symbol
    let setter_symbol = match ctx.model.query(SymbolFor { id: setter_id }) {
        Some(sym) => sym,
        None => {
            ctx.emit_error(LoweringError::internal(
                format!("setter symbol not found for '{}'", field_name),
                Some(expr.span.clone()),
            ));
            return Value::Immediate(Immediate::error());
        },
    };

    // Check if this is a static computed property
    let is_static = field_sym.is_static();

    // Lower the right-hand side value
    let rhs_value = lower_expression(ctx, rhs);

    // Create a temp for the unit return value of setter
    let unit_ty = ctx.mir.ty_unit();
    let unit_local = ctx.create_temp("setter_ret", unit_ty);
    let unit_place = Place::local(unit_local);

    // Build the qualified name for the setter
    let setter_name = qualified_name_for_symbol(ctx, &setter_symbol);

    // Get type arguments from the receiver's type for generic types
    let type_args = match extract_type_args_from_receiver(ctx, &object.ty, Some(expr.span.clone()))
    {
        Some(args) => args,
        None => return Value::Immediate(Immediate::error()),
    };

    // Look up CallableBehavior to get parameter access modes
    let callable_beh = setter_symbol.metadata().get_behavior::<CallableBehavior>();

    if is_static {
        // Static computed property - just pass the new value
        let call_args = if let Some(beh) = callable_beh {
            let params = beh.parameters();
            if let Some(param) = params.first() {
                let mode = access_mode_to_passing_mode(param.access_mode(), &rhs.ty);
                vec![CallArg::new(rhs_value, mode)]
            } else {
                // Default: consuming parameter
                let mode = if rhs.ty.is_copyable() {
                    PassingMode::Copy
                } else {
                    PassingMode::Move
                };
                vec![CallArg::new(rhs_value, mode)]
            }
        } else {
            // Default: consuming parameter
            let mode = if rhs.ty.is_copyable() {
                PassingMode::Copy
            } else {
                PassingMode::Move
            };
            vec![CallArg::new(rhs_value, mode)]
        };

        mark_moved_args(ctx, &call_args);
        let mir_callee = if type_args.is_empty() {
            Callee::direct(setter_name)
        } else {
            Callee::direct_generic(setter_name, type_args)
        };
        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
    } else {
        // Instance computed property - pass receiver and new value
        let receiver_value = lower_expression(ctx, object);

        // Build call args with receiver and newValue
        let call_args = if let Some(beh) = callable_beh {
            let mut args = Vec::new();

            // Handle receiver based on ReceiverKind
            // Setters typically need mutable receiver
            match beh.receiver() {
                Some(ReceiverKind::Borrowing) | None => {
                    // Immutable borrow (unusual for setter but respect the behavior)
                    let ref_value = create_ref(ctx, &receiver_value, &object.ty, false);
                    args.push(CallArg::new(ref_value, PassingMode::Copy));
                },
                Some(ReceiverKind::Mutating) => {
                    // Mutable borrow - typical for setters
                    let ref_value = create_ref(ctx, &receiver_value, &object.ty, true);
                    args.push(CallArg::mutating(ref_value));
                },
                Some(ReceiverKind::Consuming) => {
                    // Consumes self
                    let mode = if object.ty.is_copyable() {
                        PassingMode::Copy
                    } else {
                        PassingMode::Move
                    };
                    args.push(CallArg::new(receiver_value, mode));
                },
                Some(ReceiverKind::Initializing) => {
                    // Shouldn't happen for setter
                    let ref_value = create_ref(ctx, &receiver_value, &object.ty, true);
                    args.push(CallArg::new(ref_value, PassingMode::Copy));
                },
            }

            // Handle newValue parameter
            let params = beh.parameters();
            if let Some(param) = params.first() {
                let mode = access_mode_to_passing_mode(param.access_mode(), &rhs.ty);
                args.push(CallArg::new(rhs_value, mode));
            } else {
                // Default: consuming parameter
                let mode = if rhs.ty.is_copyable() {
                    PassingMode::Copy
                } else {
                    PassingMode::Move
                };
                args.push(CallArg::new(rhs_value, mode));
            }

            args
        } else {
            // Default: mutable receiver, consuming newValue
            let ref_value = create_ref(ctx, &receiver_value, &object.ty, true);
            let mode = if rhs.ty.is_copyable() {
                PassingMode::Copy
            } else {
                PassingMode::Move
            };
            vec![
                CallArg::new(ref_value, PassingMode::Copy),
                CallArg::new(rhs_value, mode),
            ]
        };

        mark_moved_args(ctx, &call_args);
        let mir_callee = if type_args.is_empty() {
            Callee::direct(setter_name)
        } else {
            Callee::direct_generic(setter_name, type_args)
        };
        ctx.emit_call_with_modes(unit_place, mir_callee, call_args);
    }

    // Assignment expression yields unit
    Value::Immediate(Immediate::unit())
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
        KestrelSymbolKind::SourceFile => {},
        KestrelSymbolKind::Module
        | KestrelSymbolKind::Struct
        | KestrelSymbolKind::Enum
        | KestrelSymbolKind::Protocol
        | KestrelSymbolKind::TypeAlias
        | KestrelSymbolKind::Extension => {
            parts.push(name_value.clone());
        },
        _ => {},
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
    // Check if all branches diverge (result type is Never)
    // In this case, we should NOT create a result local since it will never be assigned
    // This avoids SSA issues in codegen where an undefined local gets aliased to wrong types
    let all_branches_diverge = matches!(expr.ty.kind(), kestrel_semantic_tree::ty::TyKind::Never);

    // Only create result local and join block if branches converge
    let (result_place, join_block) = if all_branches_diverge {
        (None, None)
    } else {
        let result_ty = lower_type(ctx, &expr.ty);
        let result_local = ctx.create_temp("if_result", result_ty);
        let result_place = Place::local(result_local);

        // Track the temp for deinit if the result type needs deinit
        if ctx.type_needs_deinit(&expr.ty) {
            ctx.track_statement_temp(result_local);
        }

        // Create the join block where both branches converge
        let join_block = ctx.create_block();
        (Some(result_place), Some(join_block))
    };

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
    // Only assign to result_place if branches converge (result_place is Some)
    let _then_result_value = if !ctx.is_block_terminated() {
        if let Some(value_expr) = then_value {
            let result = lower_expression(ctx, value_expr);
            if !ctx.is_block_terminated()
                && let Some(ref place) = result_place
            {
                // Use emit_move_value to mark the temp as moved, preventing double-free
                ctx.emit_move_value(place.clone(), result);
            }
            true
        } else if let Some(ref place) = result_place {
            // No then value - assign unit (only if we have a result place)
            ctx.emit_imm(place.clone(), Immediate::unit());
            true
        } else {
            false
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
                    if !ctx.is_block_terminated()
                        && let Some(ref place) = result_place
                    {
                        // Use emit_move_value to mark the temp as moved, preventing double-free
                        ctx.emit_move_value(place.clone(), else_result);
                    }
                } else if let Some(ref place) = result_place {
                    ctx.emit_imm(place.clone(), Immediate::unit());
                }
            }

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        },

        Some(ElseBranch::ElseIf(else_if_expr)) => {
            // ElseIf is a nested if expression which will handle its own scopes
            let else_result = lower_expression(ctx, else_if_expr);

            if !ctx.is_block_terminated()
                && let Some(ref place) = result_place
            {
                // Use emit_move_value to mark the temp as moved, preventing double-free
                ctx.emit_move_value(place.clone(), else_result);
            }

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        },

        None => {
            // No else branch - result is unit (only if we have a result place)
            if let Some(ref place) = result_place {
                ctx.emit_imm(place.clone(), Immediate::unit());
            }

            else_statuses = ctx.snapshot_parent_deinit_statuses();
            else_final_terminated = ctx.is_block_terminated();
            else_scope = ctx.exit_scope_no_emit();
            else_final_block = ctx.current_block();
        },
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
    if let Some(then_block) = then_final_block
        && !then_final_terminated
    {
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

        // Jump to join block if we have one (branches converge)
        if let Some(jb) = join_block {
            ctx.emit_jump(jb);
        }
    }

    // === Emit flag settings and deinits for else branch ===
    if let Some(else_block) = else_final_block
        && !else_final_terminated
    {
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

        // Jump to join block if we have one (branches converge)
        if let Some(jb) = join_block {
            ctx.emit_jump(jb);
        }
    }

    // === Apply status updates to parent scopes ===
    if let Some(merge) = merge_result {
        ctx.apply_merge_updates(merge.updates);
    }

    // If all branches diverge, return unit placeholder (unreachable code)
    // Otherwise, continue with join block and return the result place
    if let (Some(jb), Some(place)) = (join_block, result_place) {
        ctx.set_current_block(jb);
        Value::Place(place)
    } else {
        // All branches diverge - return unit placeholder
        // The current block is now undefined, but no code will use it
        Value::Immediate(Immediate::unit())
    }
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
        },

        IfCondition::Let { pattern, value, .. } => {
            // If-let condition: use pattern matching
            lower_if_let_condition(
                ctx, pattern, value, conditions, index, then_block, else_block,
            );
        },
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
        },
        Value::Unreachable => {
            // Scrutinee diverged, if-let is unreachable
            return;
        },
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
        },

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_if_let_switch(
                ctx, path, ty, cases, default, scrutinee, conditions, index, then_block, else_block,
            );
        },

        DecisionTree::Guard { .. } => {
            // Guards shouldn't appear in if-let (guards are a match-specific feature)
            // If they do, treat as failure
            ctx.emit_jump(else_block);
        },

        DecisionTree::Failure => {
            // Pattern didn't match, go to else block
            ctx.emit_jump(else_block);
        },
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

    // Expand type aliases so enum/bool/etc. switches work with alias types (e.g., T?).
    let expanded_ty = ty.expand_aliases();

    match expanded_ty.kind() {
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
        },

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
        },

        TyKind::Int(int_bits) => {
            emit_if_let_int_switch(
                ctx,
                &switch_place,
                *int_bits,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                then_block,
                else_block,
            );
        },

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
        },

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
        },

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
        },
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
    int_bits: kestrel_semantic_tree::ty::IntBits,
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
                        rhs: Value::Immediate(make_int_immediate(int_bits, *value)),
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
            },

            Constructor::IntRange { start, end } => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Build comparison for the range bounds we have
                let cmp_place = match (start, end) {
                    (Some(s), Some(e)) => {
                        // Full range: start <= value && value <= end
                        let cmp1_ty = ctx.mir.ty_bool();
                        let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                        let cmp1_place = Place::local(cmp1_local);
                        ctx.emit_assign(
                            cmp1_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Immediate(make_int_immediate(int_bits, *s)),
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
                                rhs: Value::Immediate(make_int_immediate(int_bits, *e)),
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
                        cmp_place
                    },
                    (Some(s), None) => {
                        let cmp_ty = ctx.mir.ty_bool();
                        let cmp_local = ctx.create_temp("cmp_lo", cmp_ty);
                        let cmp_place = Place::local(cmp_local);
                        ctx.emit_assign(
                            cmp_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Immediate(make_int_immediate(int_bits, *s)),
                                rhs: Value::Place(switch_place.clone()),
                            },
                        );
                        cmp_place
                    },
                    (None, Some(e)) => {
                        let cmp_ty = ctx.mir.ty_bool();
                        let cmp_local = ctx.create_temp("cmp_hi", cmp_ty);
                        let cmp_place = Place::local(cmp_local);
                        ctx.emit_assign(
                            cmp_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Place(switch_place.clone()),
                                rhs: Value::Immediate(make_int_immediate(int_bits, *e)),
                            },
                        );
                        cmp_place
                    },
                    (None, None) => {
                        emit_if_let_decision_tree(
                            ctx, tree, scrutinee, conditions, index, then_block, else_block,
                        );
                        continue;
                    },
                };

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                ctx.set_current_block(match_block);
                emit_if_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, then_block, else_block,
                );

                ctx.set_current_block(next_block);
            },

            _ => {
                // Skip unsupported constructors
                continue;
            },
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
    label: Option<String>,
    condition: &Expression,
    body: &[kestrel_semantic_tree::stmt::Statement],
    _expr: &Expression,
) -> Value {
    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(loop_id, header_block, exit_block, label);

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
    label: Option<String>,
    body: &[kestrel_semantic_tree::stmt::Statement],
    _expr: &Expression,
) -> Value {
    // Create blocks: header (body entry) and exit
    // For infinite loops, header IS the body - no condition check
    let header_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(loop_id, header_block, exit_block, label);

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
    label: Option<String>,
    conditions: &[IfCondition],
    body: &[kestrel_semantic_tree::stmt::Statement],
) -> Value {
    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    // Note: continue should jump to header_block to re-evaluate the condition
    ctx.push_loop(loop_id, header_block, exit_block, label);

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
        },

        IfCondition::Let { pattern, value, .. } => {
            // While-let condition: use pattern matching
            lower_while_let_pattern_condition(
                ctx, pattern, value, conditions, index, body_block, exit_block,
            );
        },
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
        },
        Value::Unreachable => {
            // Scrutinee diverged, while-let is unreachable
            return;
        },
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
        },

        DecisionTree::Switch {
            path,
            ty,
            cases,
            default,
        } => {
            emit_while_let_switch(
                ctx, path, ty, cases, default, scrutinee, conditions, index, body_block, exit_block,
            );
        },

        DecisionTree::Guard { .. } => {
            // Guards shouldn't appear in while-let patterns
            ctx.emit_jump(exit_block);
        },

        DecisionTree::Failure => {
            // Pattern didn't match, exit the loop
            ctx.emit_jump(exit_block);
        },
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

    // Expand type aliases so enum/bool/etc. switches work with alias types (e.g., T?).
    let expanded_ty = ty.expand_aliases();

    match expanded_ty.kind() {
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
        },

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
        },

        TyKind::Int(int_bits) => {
            emit_while_let_int_switch(
                ctx,
                &switch_place,
                *int_bits,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
            );
        },

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
        },

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
        },

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
        },
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
    int_bits: kestrel_semantic_tree::ty::IntBits,
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
                        rhs: Value::Immediate(make_int_immediate(int_bits, *value)),
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
            },

            Constructor::IntRange { start, end } => {
                let match_block = ctx.create_block();
                let next_block = ctx.create_block();

                // Build comparison for the range bounds we have
                let cmp_place = match (start, end) {
                    (Some(s), Some(e)) => {
                        let cmp1_ty = ctx.mir.ty_bool();
                        let cmp1_local = ctx.create_temp("cmp_lo", cmp1_ty);
                        let cmp1_place = Place::local(cmp1_local);
                        ctx.emit_assign(
                            cmp1_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Immediate(make_int_immediate(int_bits, *s)),
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
                                rhs: Value::Immediate(make_int_immediate(int_bits, *e)),
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
                        cmp_place
                    },
                    (Some(s), None) => {
                        let cmp_ty = ctx.mir.ty_bool();
                        let cmp_local = ctx.create_temp("cmp_lo", cmp_ty);
                        let cmp_place = Place::local(cmp_local);
                        ctx.emit_assign(
                            cmp_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Immediate(make_int_immediate(int_bits, *s)),
                                rhs: Value::Place(switch_place.clone()),
                            },
                        );
                        cmp_place
                    },
                    (None, Some(e)) => {
                        let cmp_ty = ctx.mir.ty_bool();
                        let cmp_local = ctx.create_temp("cmp_hi", cmp_ty);
                        let cmp_place = Place::local(cmp_local);
                        ctx.emit_assign(
                            cmp_place.clone(),
                            Rvalue::BinaryOp {
                                op: BinOp::LeSigned,
                                lhs: Value::Place(switch_place.clone()),
                                rhs: Value::Immediate(make_int_immediate(int_bits, *e)),
                            },
                        );
                        cmp_place
                    },
                    (None, None) => {
                        emit_while_let_decision_tree(
                            ctx, tree, scrutinee, conditions, index, body_block, exit_block,
                        );
                        continue;
                    },
                };

                ctx.emit_branch(Value::Place(cmp_place), match_block, next_block);

                ctx.set_current_block(match_block);
                emit_while_let_decision_tree(
                    ctx, tree, scrutinee, conditions, index, body_block, exit_block,
                );

                ctx.set_current_block(next_block);
            },

            _ => {
                // Skip unsupported constructors
                continue;
            },
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

/// Determine the MIR CastKind based on source and target primitive types.
fn determine_cast_kind(
    from: kestrel_semantic_tree::expr::LangPrimitive,
    to: kestrel_semantic_tree::expr::LangPrimitive,
) -> CastKind {
    // Float <-> Float
    if from.is_float() && to.is_float() {
        if from.bit_width() < to.bit_width() {
            return CastKind::FloatWiden;
        } else {
            return CastKind::FloatTruncate;
        }
    }

    // Int <-> Float
    if from.is_int() && to.is_float() {
        return CastKind::IntToFloat;
    }
    if from.is_float() && to.is_int() {
        return CastKind::FloatToInt;
    }

    // Int <-> Int (signed or unsigned)
    if from.is_int() && to.is_int() {
        if from.bit_width() < to.bit_width() {
            // Use unsigned widen (zero-extend) for unsigned source types
            if from.is_unsigned() {
                return CastKind::IntUnsignedWiden;
            }
            return CastKind::IntWiden;
        } else {
            return CastKind::IntTruncate;
        }
    }

    // Default: truncate (should not happen with valid casts)
    CastKind::IntTruncate
}

fn is_protocol_type(ty: &Ty) -> bool {
    matches!(ty.expand_aliases().kind(), TyKind::Protocol { .. })
}

/// Find the protocol that an extension method belongs to.
///
/// When an extension adds conformances (e.g., `extend Comparable: Less[Self]`),
/// the methods in that extension implement protocol methods. This function
/// finds the protocol that contains a method with the given name.
fn find_protocol_for_extension_method(
    extension: &std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
    method_name: &str,
) -> Option<
    std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
> {
    use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
    use kestrel_semantic_tree::language::KestrelLanguage;
    use semantic_tree::symbol::Symbol;

    // Get the conformances added by this extension
    let conformances = extension
        .metadata()
        .get_behavior::<ConformancesBehavior>()?;

    // Search through each protocol conformance
    for protocol_ty in conformances.conformances() {
        if let TyKind::Protocol { symbol, .. } = protocol_ty.kind() {
            // Check if this protocol has a method with the given name
            for child in symbol.metadata().children() {
                let child_name = child.metadata().name().value.clone();
                let child_kind = child.metadata().kind();
                if child_kind == KestrelSymbolKind::Function && child_name == method_name {
                    return Some(symbol.clone() as std::sync::Arc<dyn Symbol<KestrelLanguage>>);
                }
            }
        }
    }

    None
}

/// Lower a function reference being used as a first-class value.
///
/// When a function is used as a value (e.g., `let f = myFunction`), we need to
/// wrap it in a thunk that adapts its calling convention to match the thick
/// function calling convention used by closures.
///
/// The thunk accepts `(env_ptr, ...args...)` and ignores env_ptr, forwarding
/// only the args to the original function.
fn lower_function_ref_as_value(
    ctx: &mut LoweringContext,
    sym: &std::sync::Arc<
        dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>,
    >,
    expr_ty: &Ty,
    _span: &kestrel_span::Span,
) -> Value {
    use kestrel_execution_graph::Rvalue;

    // Get the function's qualified name
    let func_name = qualified_name_for_symbol(ctx, sym);

    // Extract parameter and return types from the expression's type
    let (param_types, return_type) = match expr_ty.kind() {
        TyKind::Function {
            params,
            return_type,
        } => {
            let mir_params: Vec<_> = params.iter().map(|p| lower_type(ctx, p)).collect();
            let mir_ret = lower_type(ctx, return_type);
            (mir_params, mir_ret)
        },
        _ => {
            // Expression type is not a function - this shouldn't happen after type checking
            // Fall back to old behavior (direct function ref)
            return Value::Immediate(Immediate::function_ref(func_name));
        },
    };

    // Generate or retrieve the thunk for this function
    // For now, we don't handle generic type args - that would need more work
    let type_args: Vec<kestrel_execution_graph::Id<kestrel_execution_graph::Ty>> = vec![];
    let thunk_name =
        ctx.get_or_create_function_thunk(func_name, &param_types, return_type, &type_args);

    // Create a thick callable via ApplyPartial with empty captures
    // This produces a struct { thunk_ptr, null_env }
    let thick_ty = ctx
        .mir
        .intern_type(kestrel_execution_graph::MirTy::FuncThick {
            params: param_types,
            ret: return_type,
        });
    let result_local = ctx.create_temp("func_ref", thick_ty);
    let result_place = Place::local(result_local);

    ctx.emit_assign(
        result_place.clone(),
        Rvalue::ApplyPartial {
            func: thunk_name,
            captures: vec![],
        },
    );

    Value::Place(result_place)
}

/// Extract the element type T from a LiteralSlice[T] type.
///
/// If the type is not a LiteralSlice, returns unit type as a fallback.
fn extract_literal_slice_element_type(
    ctx: &mut LoweringContext,
    ty: &Ty,
) -> kestrel_execution_graph::Id<kestrel_execution_graph::Ty> {
    // Expand type aliases to get the actual struct type
    let expanded = ty.expand_aliases();

    // LiteralSlice[T] is a struct with one type parameter
    if let TyKind::Struct { substitutions, .. } = expanded.kind() {
        // The first substitution is the element type T
        if let Some((_, element_ty)) = substitutions.iter().next() {
            return lower_type(ctx, element_ty);
        }
    }

    // Fallback: return unit type (shouldn't happen for valid LiteralSlice types)
    ctx.mir.ty_unit()
}
