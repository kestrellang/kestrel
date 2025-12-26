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

        ExprKind::SymbolRef(_symbol_id) => {
            // TODO: Handle module-level functions and globals
            ctx.emit_error(LoweringError::unsupported_expr(
                "SymbolRef",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
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
                    Value::Immediate(Immediate::unit())
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
                    Value::Immediate(Immediate::unit())
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
                    return Value::Immediate(Immediate::unit());
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
            loop_id: _,
            label: _,
            conditions: _,
            body: _,
        } => {
            // TODO: Implement while-let loops
            ctx.emit_error(LoweringError::unsupported_expr(
                "while let expression",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

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
        ExprKind::Match { scrutinee: _, arms: _ } => {
            // TODO: Implement match lowering with switch and pattern matching
            ctx.emit_error(LoweringError::unsupported_expr(
                "match expression",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        // === Closures ===
        ExprKind::Closure {
            params: _,
            body: _,
            tail_expr: _,
            captures: _,
            uses_it: _,
            implicit_param: _,
        } => {
            // TODO: Implement closure lowering
            // This requires generating an environment struct and a call function
            ctx.emit_error(LoweringError::unsupported_expr(
                "closure expression",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

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
                    ctx.mir.ty_unit()
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
            Value::Immediate(Immediate::unit())
        }

        ExprKind::TypeRef(_) => {
            // Type references shouldn't appear as values
            ctx.emit_error(LoweringError::internal(
                "type reference as value",
                Some(expr.span.clone()),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::TypeParameterRef(_) => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "type parameter reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::AssociatedTypeRef => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "associated type reference",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::MethodRef {
            receiver: _,
            candidates: _,
            method_name,
        } => {
            // Method reference without a call - creates a bound method
            ctx.emit_error(LoweringError::unsupported_expr(
                format!("method reference '{}'", method_name),
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::EnumCase { case_id: _ } => {
            // TODO: Enum case construction
            ctx.emit_error(LoweringError::unsupported_expr(
                "enum case",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
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
            Value::Immediate(Immediate::unit())
        }

        ExprKind::Error => {
            // Error expression - return unit (error already reported)
            Value::Immediate(Immediate::unit())
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

        // === Binary Operations ===
        _ => {
            // Binary operations take one argument
            if arguments.is_empty() {
                ctx.emit_error(LoweringError::internal(
                    "binary primitive method with no arguments",
                    Some(expr.span.clone()),
                ));
                return Value::Immediate(Immediate::unit());
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

                // Methods that don't map to binary ops
                PrimitiveMethod::IntToString
                | PrimitiveMethod::IntAbs
                | PrimitiveMethod::StringLength
                | PrimitiveMethod::StringIsEmpty => {
                    // TODO: These need special handling (function calls)
                    ctx.emit_error(LoweringError::unsupported_expr(
                        format!("primitive method '{}'", method.name()),
                        expr.span.clone(),
                    ));
                    return Value::Immediate(Immediate::unit());
                }

                // Already handled as unary
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
                    
                    // Check if this is an initializer call
                    let is_initializer = sym.metadata().kind() == KestrelSymbolKind::Initializer;
                    
                    if is_initializer {
                        // Initializer call - need to allocate self and pass as first arg
                        // Initializers have signature: func Type.init(self: &var Type, params...) -> ()
                        let func_name = qualified_name_for_symbol(ctx, &sym);
                        
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
                        
                        // Call the init function
                        let mir_callee = if type_args.is_empty() {
                            Callee::direct(func_name)
                        } else {
                            Callee::direct_generic(func_name, type_args)
                        };
                        ctx.emit_call(unit_place, mir_callee, all_args);
                        
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
                    return Value::Immediate(Immediate::unit());
                }
            }
        }

        ExprKind::MethodRef {
            receiver,
            candidates,
            method_name,
        } => {
            // Method call - receiver becomes first argument
            let receiver_value = lower_expression(ctx, receiver);
            let mut all_args = vec![receiver_value];
            all_args.extend(arg_values);

            // For methods, we need to find the resolved method from candidates
            // During type inference, the correct candidate should have been selected
            if let Some(&method_id) = candidates.first() {
                let method_symbol = ctx.model.query(SymbolFor { id: method_id });
                match method_symbol {
                    Some(sym) => {
                        let func_name = qualified_name_for_symbol(ctx, &sym);
                        let mir_callee = if type_args.is_empty() {
                            Callee::direct(func_name)
                        } else {
                            Callee::direct_generic(func_name, type_args)
                        };
                        ctx.emit_call(result_place.clone(), mir_callee, all_args);
                    }
                    None => {
                        ctx.emit_error(LoweringError::internal(
                            format!("method symbol not found for '{}'", method_name),
                            Some(expr.span.clone()),
                        ));
                        return Value::Immediate(Immediate::unit());
                    }
                }
            } else {
                ctx.emit_error(LoweringError::internal(
                    format!("no method candidates for '{}'", method_name),
                    Some(expr.span.clone()),
                ));
                return Value::Immediate(Immediate::unit());
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
                    return Value::Immediate(Immediate::unit());
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
                    // Assume it's a thin function pointer for now
                    // TODO: Distinguish thin vs thick based on type
                    ctx.emit_call(result_place.clone(), Callee::Thin(callee_place), arg_values);
                }
                Value::Immediate(_) => {
                    ctx.emit_error(LoweringError::unsupported_expr(
                        "indirect call on immediate value",
                        expr.span.clone(),
                    ));
                    return Value::Immediate(Immediate::unit());
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

    // For now, only handle single boolean conditions
    if conditions.len() != 1 {
        ctx.emit_error(LoweringError::unsupported_expr(
            "if-let or condition chain",
            expr.span.clone(),
        ));
        return Value::Place(result_place);
    }

    let condition = match &conditions[0] {
        IfCondition::Expr(e) => e,
        IfCondition::Let { .. } => {
            ctx.emit_error(LoweringError::unsupported_expr(
                "if-let condition",
                expr.span.clone(),
            ));
            return Value::Place(result_place);
        }
    };

    // Create blocks
    let then_block = ctx.create_block();
    let else_block = ctx.create_block();
    let join_block = ctx.create_block();

    // Lower condition and emit branch
    let cond_value = lower_expression(ctx, condition);
    ctx.emit_branch(cond_value, then_block, else_block);

    // Lower then branch
    ctx.set_current_block(then_block);
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
