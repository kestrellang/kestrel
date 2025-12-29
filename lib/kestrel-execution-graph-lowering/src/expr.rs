//! Expression lowering - converts semantic expressions to MIR values.
//!
//! This is the core of the lowering pass. Each expression is converted to
//! a MIR Value (either a Place or an Immediate), potentially generating
//! statements and new basic blocks along the way.

use kestrel_execution_graph::{BinOp, Callee, Immediate, Place, Rvalue, UnOp, Value};
use kestrel_semantic_model::SymbolFor;
use kestrel_semantic_tree::expr::{
    CallArgument, ElseBranch, ExprKind, Expression, IfCondition, LiteralValue, PrimitiveMethod,
};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;

use crate::context::LoweringContext;
use crate::error::LoweringError;
use crate::name::qualified_name_for_symbol;
use crate::stmt::lower_statement;
use crate::ty::lower_type;

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
            Value::Place(Place::local(mir_local))
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
            let element_values: Vec<Value> = elements
                .iter()
                .map(|e| lower_expression(ctx, e))
                .collect();

            // Get the array element type from the expression type
            let element_ty = match expr.ty.kind() {
                kestrel_semantic_tree::ty::TyKind::Array(elem_ty) => lower_type(ctx, elem_ty),
                _ => {
                    ctx.emit_error(LoweringError::internal(
                        "array literal with non-array type",
                        Some(expr.span.clone()),
                    ));
                    ctx.mir.ty_error()
                }
            };

            // Create result local and emit array construction
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("array", result_ty);
            let result_place = Place::local(result_local);

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
            let element_values: Vec<Value> = elements
                .iter()
                .map(|e| lower_expression(ctx, e))
                .collect();

            // Create result local and emit tuple construction
            let result_ty = lower_type(ctx, &expr.ty);
            let result_local = ctx.create_temp("tuple", result_ty);
            let result_place = Place::local(result_local);

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
    // Lower arguments first
    let arg_values: Vec<Value> = arguments
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Lower type arguments for generic calls
    let type_args: Vec<_> = substitutions
        .types()
        .map(|ty| lower_type(ctx, ty))
        .collect();

    // Get the result type and create a temp for the result
    let result_ty = lower_type(ctx, &expr.ty);
    let result_local = ctx.create_temp("call", result_ty);
    let result_place = Place::local(result_local);

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
                        ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));
                        
                        // Build args: self_ref first, then the user-provided arguments
                        let mut all_args = vec![Value::Place(self_ref_place)];
                        all_args.extend(arg_values);
                        
                        // Create a temp for the unit return value of init (we discard it)
                        let unit_ty = ctx.mir.ty_unit();
                        let unit_local = ctx.create_temp("init_ret", unit_ty);
                        let unit_place = Place::local(unit_local);
                        
                        if is_type_param_init {
                            // Protocol initializer on type parameter: T() where T: Factory
                            // The parent of the init symbol should be the protocol
                            if let Some(parent) = sym.metadata().parent() {
                                if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                                    let protocol_name = qualified_name_for_symbol(ctx, &parent);
                                    let for_type = lower_type(ctx, &expr.ty);
                                    let mir_callee = Callee::witness(protocol_name, "init", for_type);
                                    ctx.emit_call(unit_place, mir_callee, all_args);
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
                            let mir_callee = if type_args.is_empty() {
                                Callee::direct(func_name)
                            } else {
                                Callee::direct_generic(func_name, type_args.clone())
                            };
                            ctx.emit_call(unit_place, mir_callee, all_args);
                        }
                        
                        // result_place now contains the initialized struct
                    } else {
                        // Regular function call
                        let func_name = qualified_name_for_symbol(ctx, &sym);
                        let mir_callee = if type_args.is_empty() {
                            Callee::direct(func_name)
                        } else {
                            Callee::direct_generic(func_name, type_args)
                        };
                        ctx.emit_call(result_place.clone(), mir_callee, arg_values);
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

            // For instance methods on type params, receiver becomes first argument
            // For static methods on type params/assoc types, there's no receiver value
            let all_args = if is_static_type_param_call || is_static_assoc_type_call {
                // Static method call on type parameter or associated type: T.create(), T.Item.create()
                // No receiver value, just the arguments
                arg_values
            } else {
                // Instance method call: a.add(b) where a: T
                let receiver_value = lower_expression(ctx, receiver);
                let mut all_args = vec![receiver_value];
                all_args.extend(arg_values);
                all_args
            };

            // For methods, we need to find the resolved method from candidates
            // During type inference, the correct candidate should have been selected
            if let Some(&method_id) = candidates.first() {
                let method_symbol = ctx.model.query(SymbolFor { id: method_id });
                match method_symbol {
                    Some(sym) => {
                        // Check if this is a witness method call (method on type parameter or associated type)
                        if is_type_param_call || is_static_type_param_call || is_assoc_type_call || is_static_assoc_type_call {
                            // Get the protocol from the method's parent
                            if let Some(parent) = sym.metadata().parent() {
                                if parent.metadata().kind() == KestrelSymbolKind::Protocol {
                                    let protocol_name = qualified_name_for_symbol(ctx, &parent);
                                    let for_type = lower_type(ctx, &receiver.ty);
                                    let mir_callee =
                                        Callee::witness(protocol_name, method_name.clone(), for_type);
                                    ctx.emit_call(result_place.clone(), mir_callee, all_args);
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
                            let mir_callee = if type_args.is_empty() {
                                Callee::direct(func_name)
                            } else {
                                Callee::direct_generic(func_name, type_args.clone())
                            };
                            ctx.emit_call(result_place.clone(), mir_callee, all_args);
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

                    let init_name = ctx.mir.intern_name(
                        kestrel_execution_graph::QualifiedNameData::new(name_parts),
                    );
                    
                    // Create a mutable reference to the result place
                    // The ref type is &var T where T is the struct type
                    let ref_ty = ctx.mir.ty_ref_mut(result_ty);
                    let self_ref_local = ctx.create_temp("self_ref", ref_ty);
                    let self_ref_place = Place::local(self_ref_local);
                    
                    // Emit: %self_ref = ref var %result
                    ctx.emit_assign(self_ref_place.clone(), Rvalue::RefMut(result_place.clone()));
                    
                    // Build args: self_ref first, then the user-provided arguments
                    let mut all_args = vec![Value::Place(self_ref_place)];
                    all_args.extend(arg_values);
                    
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
                    ctx.emit_call(unit_place, mir_callee, all_args);
                    
                    // result_place now contains the initialized struct
                    // (init wrote to it via the self_ref)
                }
                None => {
                    ctx.emit_error(LoweringError::internal(
                        format!("type symbol not found for initializer call: {:?}", symbol_id),
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
            // Closures are "thick" callables
            ctx.emit_call(result_place.clone(), Callee::Thick(callee_place), arg_values);
        }

        _ => {
            // Other callee expressions - try to lower as a place for indirect call
            let callee_value = lower_expression(ctx, callee);
            match callee_value {
                Value::Place(callee_place) => {
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
                    ctx.emit_call(result_place.clone(), mir_callee, arg_values);
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
    symbol: &std::sync::Arc<dyn semantic_tree::symbol::Symbol<kestrel_semantic_tree::language::KestrelLanguage>>,
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

    // Create the join block where both branches converge
    let join_block = ctx.create_block();
    
    // Create the else block
    let else_block = ctx.create_block();

    // Lower the condition chain
    // This will emit all pattern tests and boolean conditions, eventually
    // jumping to either then_block_start or else_block
    let then_block_start = ctx.create_block();
    
    lower_condition_chain(ctx, conditions, 0, then_block_start, else_block);

    // Lower then branch
    ctx.set_current_block(then_block_start);
    for stmt in then_branch {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    if !ctx.is_block_terminated() {
        if let Some(value_expr) = then_value {
            let then_result = lower_expression(ctx, value_expr);
            if !ctx.is_block_terminated() {
                ctx.emit_assign_value(result_place.clone(), then_result);
                ctx.emit_jump(join_block);
            }
        } else {
            // No then value - assign unit
            ctx.emit_imm(result_place.clone(), Immediate::unit());
            ctx.emit_jump(join_block);
        }
    }

    // Lower else branch
    ctx.set_current_block(else_block);
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
                        ctx.emit_jump(join_block);
                    }
                } else {
                    ctx.emit_imm(result_place.clone(), Immediate::unit());
                    ctx.emit_jump(join_block);
                }
            }
        }

        Some(ElseBranch::ElseIf(else_if_expr)) => {
            let else_result = lower_expression(ctx, else_if_expr);
            if !ctx.is_block_terminated() {
                ctx.emit_assign_value(result_place.clone(), else_result);
                ctx.emit_jump(join_block);
            }
        }

        None => {
            // No else branch - result is unit
            ctx.emit_imm(result_place.clone(), Immediate::unit());
            ctx.emit_jump(join_block);
        }
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
            lower_if_let_condition(ctx, pattern, value, conditions, index, then_block, else_block);
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

        DecisionTree::Switch { path, ty, cases, default } => {
            emit_if_let_switch(ctx, path, ty, cases, default, scrutinee, conditions, index, then_block, else_block);
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
    cases: &[(kestrel_semantic_pattern_matching::Constructor, kestrel_semantic_pattern_matching::DecisionTree)],
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
            emit_if_let_bool_switch(ctx, &switch_place, cases, default, scrutinee, conditions, index, then_block, else_block);
        }

        TyKind::Enum { .. } => {
            emit_if_let_enum_switch(ctx, &switch_place, cases, default, scrutinee, conditions, index, then_block, else_block);
        }

        TyKind::Int(_) => {
            emit_if_let_int_switch(ctx, &switch_place, cases, default, scrutinee, conditions, index, then_block, else_block);
        }

        TyKind::String => {
            emit_if_let_string_switch(ctx, &switch_place, cases, default, scrutinee, conditions, index, then_block, else_block);
        }

        TyKind::Tuple(_) | TyKind::Struct { .. } => {
            // Single constructor types - just recurse into the case
            if let Some((_, subtree)) = cases.first() {
                emit_if_let_decision_tree(ctx, subtree, scrutinee, conditions, index, then_block, else_block);
            } else if let Some(default_tree) = default {
                emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
            } else {
                ctx.emit_jump(else_block);
            }
        }

        _ => {
            // For other types, try the default or first case
            if let Some(default_tree) = default {
                emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
            } else if let Some((_, tree)) = cases.first() {
                emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);
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
    cases: &[(kestrel_semantic_pattern_matching::Constructor, kestrel_semantic_pattern_matching::DecisionTree)],
    default: &Option<Box<kestrel_semantic_pattern_matching::DecisionTree>>,
    scrutinee: &Place,
    conditions: &[IfCondition],
    index: usize,
    then_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
    else_block: kestrel_execution_graph::Id<kestrel_execution_graph::Block>,
) {
    use kestrel_semantic_pattern_matching::Constructor;

    // Find true and false cases
    let true_tree = cases.iter().find(|(c, _)| matches!(c, Constructor::True)).map(|(_, t)| t);
    let false_tree = cases.iter().find(|(c, _)| matches!(c, Constructor::False)).map(|(_, t)| t);

    // Create blocks for each case
    let true_block = ctx.create_block();
    let false_block = ctx.create_block();

    // Emit branch
    ctx.emit_branch(Value::Place(switch_place.clone()), true_block, false_block);

    // Emit true case
    ctx.set_current_block(true_block);
    if let Some(tree) = true_tree {
        emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);
    } else if let Some(default_tree) = default {
        emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
    } else {
        ctx.emit_jump(else_block);
    }

    // Emit false case
    ctx.set_current_block(false_block);
    if let Some(tree) = false_tree {
        emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);
    } else if let Some(default_tree) = default {
        emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit enum switch for if-let.
fn emit_if_let_enum_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(kestrel_semantic_pattern_matching::Constructor, kestrel_semantic_pattern_matching::DecisionTree)],
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
        emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);
    }

    // Emit default case: if there's a default tree use it, otherwise go to else
    ctx.set_current_block(default_case_block);
    if let Some(default_tree) = default {
        emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit integer comparison chain for if-let.
fn emit_if_let_int_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(kestrel_semantic_pattern_matching::Constructor, kestrel_semantic_pattern_matching::DecisionTree)],
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
            emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
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
                emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);

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
                emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);

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
        emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
    } else {
        ctx.emit_jump(else_block);
    }
}

/// Emit string comparison chain for if-let.
fn emit_if_let_string_switch(
    ctx: &mut LoweringContext,
    switch_place: &Place,
    cases: &[(kestrel_semantic_pattern_matching::Constructor, kestrel_semantic_pattern_matching::DecisionTree)],
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
            emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
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
            emit_if_let_decision_tree(ctx, tree, scrutinee, conditions, index, then_block, else_block);

            ctx.set_current_block(next_block);
        }
    }

    // After all cases, check default or go to else
    if let Some(default_tree) = default {
        emit_if_let_decision_tree(ctx, default_tree, scrutinee, conditions, index, then_block, else_block);
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
    use crate::context::LoopInfo;

    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(LoopInfo {
        loop_id,
        header_block,
        exit_block,
    });

    // Jump to header
    ctx.emit_jump(header_block);

    // Header: check condition
    ctx.set_current_block(header_block);
    let cond_value = lower_expression(ctx, condition);
    ctx.emit_branch(cond_value, body_block, exit_block);

    // Body
    ctx.set_current_block(body_block);
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Jump back to header (if not terminated by break/return)
    if !ctx.is_block_terminated() {
        ctx.emit_jump(header_block);
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
    use crate::context::LoopInfo;

    // Create blocks: header (body entry) and exit
    // For infinite loops, header IS the body - no condition check
    let header_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    ctx.push_loop(LoopInfo {
        loop_id,
        header_block,
        exit_block,
    });

    // Jump to header (body entry)
    ctx.emit_jump(header_block);

    // Body
    ctx.set_current_block(header_block);
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Jump back to header (infinite loop) if not terminated by break/return
    if !ctx.is_block_terminated() {
        ctx.emit_jump(header_block);
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
    use crate::context::LoopInfo;

    // Create blocks
    let header_block = ctx.create_block();
    let body_block = ctx.create_block();
    let exit_block = ctx.create_block();

    // Push loop info for break/continue
    // Note: continue should jump to header_block to re-evaluate the condition
    ctx.push_loop(LoopInfo {
        loop_id,
        header_block,
        exit_block,
    });

    // Jump to header
    ctx.emit_jump(header_block);

    // Header: evaluate conditions
    // This is where we emit the pattern matching logic
    // If all conditions pass → body_block
    // If any condition fails → exit_block
    ctx.set_current_block(header_block);
    lower_while_let_condition_chain(ctx, conditions, 0, body_block, exit_block);

    // Body
    ctx.set_current_block(body_block);
    for stmt in body {
        lower_statement(ctx, stmt);
        if ctx.is_block_terminated() {
            break;
        }
    }

    // Jump back to header (if not terminated by break/return)
    if !ctx.is_block_terminated() {
        ctx.emit_jump(header_block);
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
                ctx,
                pattern,
                value,
                conditions,
                index,
                body_block,
                exit_block,
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
                ctx,
                path,
                ty,
                cases,
                default,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
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
                    ctx,
                    subtree,
                    scrutinee,
                    conditions,
                    index,
                    body_block,
                    exit_block,
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
                    ctx,
                    tree,
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
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
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
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
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
            ctx,
            tree,
            scrutinee,
            conditions,
            index,
            body_block,
            exit_block,
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
                    ctx,
                    tree,
                    scrutinee,
                    conditions,
                    index,
                    body_block,
                    exit_block,
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
                    ctx,
                    tree,
                    scrutinee,
                    conditions,
                    index,
                    body_block,
                    exit_block,
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
                ctx,
                tree,
                scrutinee,
                conditions,
                index,
                body_block,
                exit_block,
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
