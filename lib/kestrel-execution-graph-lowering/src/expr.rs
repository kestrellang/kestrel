//! Expression lowering - converts semantic expressions to MIR values.
//!
//! This is the core of the lowering pass. Each expression is converted to
//! a MIR Value (either a Place or an Immediate), potentially generating
//! statements and new basic blocks along the way.

use kestrel_execution_graph::{BinOp, Immediate, Place, Rvalue, UnOp, Value};
use kestrel_semantic_tree::expr::{
    CallArgument, ElseBranch, ExprKind, Expression, IfCondition, LiteralValue, PrimitiveMethod,
};


use crate::context::LoweringContext;
use crate::error::LoweringError;

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
            substitutions: _,
        } => lower_call(ctx, callee, arguments, expr),

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
            loop_id: _,
            label: _,
            body: _,
        } => {
            // TODO: Implement infinite loop with break/continue
            ctx.emit_error(LoweringError::unsupported_expr(
                "loop expression",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

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

        ExprKind::Break { loop_id: _, label: _ } => {
            // TODO: Jump to loop exit block
            ctx.emit_error(LoweringError::unsupported_expr(
                "break expression",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::Continue { loop_id: _, label: _ } => {
            // TODO: Jump to loop header block
            ctx.emit_error(LoweringError::unsupported_expr(
                "continue expression",
                expr.span.clone(),
            ));
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
        ExprKind::Array(_elements) => {
            // TODO: Array literal
            ctx.emit_error(LoweringError::unsupported_expr(
                "array literal",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
        }

        ExprKind::Tuple(_elements) => {
            // TODO: Tuple literal - needs construct statement
            ctx.emit_error(LoweringError::unsupported_expr(
                "tuple literal",
                expr.span.clone(),
            ));
            Value::Immediate(Immediate::unit())
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
    expr: &Expression,
) -> Value {
    // Lower arguments first
    let _arg_values: Vec<Value> = arguments
        .iter()
        .map(|arg| lower_expression(ctx, &arg.value))
        .collect();

    // Determine the callee
    let _mir_callee = match &callee.kind {
        ExprKind::SymbolRef(_symbol_id) => {
            // Direct function call
            // TODO: Look up the symbol to get its qualified name
            // For now, emit an error
            ctx.emit_error(LoweringError::unsupported_expr(
                "function call via SymbolRef",
                expr.span.clone(),
            ));
            return Value::Immediate(Immediate::unit());
        }

        ExprKind::MethodRef {
            receiver: _,
            candidates: _,
            method_name,
        } => {
            // Method call - add receiver as first argument
            ctx.emit_error(LoweringError::unsupported_expr(
                format!("method call '{}'", method_name),
                expr.span.clone(),
            ));
            return Value::Immediate(Immediate::unit());
        }

        ExprKind::TypeRef(_symbol_id) => {
            // Calling a type (initializer call)
            ctx.emit_error(LoweringError::unsupported_expr(
                "initializer call via TypeRef",
                expr.span.clone(),
            ));
            return Value::Immediate(Immediate::unit());
        }

        _ => {
            // Other callee expressions (closures, etc.)
            ctx.emit_error(LoweringError::unsupported_expr(
                "indirect call",
                expr.span.clone(),
            ));
            return Value::Immediate(Immediate::unit());
        }
    };
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
