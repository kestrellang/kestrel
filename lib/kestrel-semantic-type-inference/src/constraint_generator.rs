//! Constraint generation from code blocks.
//!
//! This module walks a `CodeBlock` and generates type inference constraints
//! for the expressions and statements within it.

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::ty::{Ty, TyKind};

use crate::context::InferenceContext;

/// Generate type inference constraints for a code block.
///
/// This walks all statements and expressions in the block, generating
/// constraints that the solver will use to infer types.
///
/// # Arguments
/// * `ctx` - The inference context to add constraints to
/// * `block` - The code block to generate constraints for
/// * `return_type` - The expected return type (for yield expression)
pub fn generate_constraints(
    ctx: &mut InferenceContext<'_>,
    block: &CodeBlock,
    return_type: Option<&Ty>,
) {
    // Process all statements
    for stmt in &block.statements {
        generate_statement_constraints(ctx, stmt);
    }

    // Process yield expression if present
    if let Some(yield_expr) = block.yield_expr() {
        generate_expression_constraints(ctx, yield_expr);

        // If we have an expected return type, equate yield type with it
        if let Some(ret_ty) = return_type {
            ctx.register_type(ret_ty);
            ctx.register_type(&yield_expr.ty);
            ctx.equate(yield_expr.ty.id(), ret_ty.id(), yield_expr.span.clone());
        }
    }
}

/// Generate constraints for a statement.
fn generate_statement_constraints(ctx: &mut InferenceContext<'_>, stmt: &Statement) {
    match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            // Register the pattern type
            ctx.register_type(&pattern.ty);

            // If there's an initializer, equate its type with the pattern type
            if let Some(init) = value {
                generate_expression_constraints(ctx, init);
                ctx.register_type(&init.ty);
                ctx.equate(pattern.ty.id(), init.ty.id(), stmt.span.clone());
            }
        }
        StatementKind::Expr(expr) => {
            generate_expression_constraints(ctx, expr);
        }
    }
}

/// Generate constraints for an expression.
fn generate_expression_constraints(ctx: &mut InferenceContext<'_>, expr: &Expression) {
    // Register this expression's type
    ctx.register_type(&expr.ty);

    match &expr.kind {
        // Literals have concrete types - no constraints needed
        ExprKind::Literal(_) => {}

        // Arrays: all elements must have the same type
        ExprKind::Array(elements) => {
            if let TyKind::Array(elem_ty) = expr.ty.kind() {
                ctx.register_type(elem_ty);
                for elem in elements {
                    generate_expression_constraints(ctx, elem);
                    ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                }
            }
        }

        // Tuples: each element has its corresponding type
        ExprKind::Tuple(elements) => {
            if let TyKind::Tuple(elem_tys) = expr.ty.kind() {
                for (elem, elem_ty) in elements.iter().zip(elem_tys.iter()) {
                    generate_expression_constraints(ctx, elem);
                    ctx.register_type(elem_ty);
                    ctx.equate(elem.ty.id(), elem_ty.id(), elem.span.clone());
                }
            }
        }

        // Grouping: just process the inner expression
        ExprKind::Grouping(inner) => {
            generate_expression_constraints(ctx, inner);
            ctx.equate(inner.ty.id(), expr.ty.id(), expr.span.clone());
        }

        // References: type is already set during binding
        ExprKind::LocalRef(_) | ExprKind::SymbolRef(_) | ExprKind::TypeRef(_) => {}
        ExprKind::OverloadedRef(_) | ExprKind::TypeParameterRef(_) => {}

        // Field access: type is the field type
        ExprKind::FieldAccess { object, .. } => {
            generate_expression_constraints(ctx, object);
        }

        // Tuple index: type is the element type
        ExprKind::TupleIndex { tuple, .. } => {
            generate_expression_constraints(ctx, tuple);
        }

        // Method reference: process receiver
        ExprKind::MethodRef { receiver, .. } => {
            generate_expression_constraints(ctx, receiver);
        }

        // Calls
        ExprKind::Call {
            callee, arguments, ..
        } => {
            generate_expression_constraints(ctx, callee);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
            // TODO: Generate constraints for argument types matching parameter types
        }

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            generate_expression_constraints(ctx, receiver);
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                generate_expression_constraints(ctx, &arg.value);
            }
            // TODO: Generate constraints for field types
        }

        // Assignment
        ExprKind::Assignment { target, value } => {
            generate_expression_constraints(ctx, target);
            generate_expression_constraints(ctx, value);
            ctx.equate(target.ty.id(), value.ty.id(), expr.span.clone());
        }

        // Control flow
        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            generate_expression_constraints(ctx, condition);
            // Condition must be Bool
            let bool_ty = Ty::bool(condition.span.clone());
            ctx.register_type(&bool_ty);
            ctx.equate(condition.ty.id(), bool_ty.id(), condition.span.clone());

            // Process then branch
            for stmt in then_branch {
                generate_statement_constraints(ctx, stmt);
            }
            if let Some(then_val) = then_value {
                generate_expression_constraints(ctx, then_val);
            }

            // Process else branch
            if let Some(else_br) = else_branch {
                match else_br {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            generate_statement_constraints(ctx, stmt);
                        }
                        if let Some(else_val) = value {
                            generate_expression_constraints(ctx, else_val);
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(else_if) => {
                        generate_expression_constraints(ctx, else_if);
                    }
                }
            }
        }

        ExprKind::While {
            condition, body, ..
        } => {
            generate_expression_constraints(ctx, condition);
            // Condition must be Bool
            let bool_ty = Ty::bool(condition.span.clone());
            ctx.register_type(&bool_ty);
            ctx.equate(condition.ty.id(), bool_ty.id(), condition.span.clone());

            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        }

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                generate_statement_constraints(ctx, stmt);
            }
        }

        ExprKind::Break { .. } | ExprKind::Continue { .. } => {}

        ExprKind::Return { value } => {
            if let Some(val) = value {
                generate_expression_constraints(ctx, val);
            }
            // TODO: Equate return value type with function return type
        }

        ExprKind::Error => {}
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require setting up a full semantic model
}
