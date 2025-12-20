//! Apply type inference solutions to code blocks.
//!
//! This module transforms a `CodeBlock` by replacing all `TyKind::Infer`
//! placeholders with their resolved types from the solution.

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::expr::{CallArgument, ElseBranch, ExprKind, Expression};
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::ty::{ParamInfo, Ty, TyKind};

use crate::solution::Solution;

/// Apply a solution to a code block, resolving all inference placeholders.
///
/// Returns a new `CodeBlock` where all `TyKind::Infer` types have been
/// replaced with their resolved concrete types.
pub fn apply_solution(block: &CodeBlock, solution: &Solution) -> CodeBlock {
    let statements = block
        .statements
        .iter()
        .map(|stmt| apply_to_statement(stmt, solution))
        .collect();

    let yield_expr = block
        .yield_expr
        .as_ref()
        .map(|expr| Box::new(apply_to_expression(expr, solution)));

    CodeBlock {
        statements,
        yield_expr,
    }
}

/// Apply solution to a statement.
fn apply_to_statement(stmt: &Statement, solution: &Solution) -> Statement {
    let kind = match &stmt.kind {
        StatementKind::Binding { pattern, value } => {
            let resolved_pattern = apply_to_pattern(pattern, solution);
            let resolved_value = value
                .as_ref()
                .map(|v| apply_to_expression(v, solution));
            StatementKind::Binding {
                pattern: resolved_pattern,
                value: resolved_value,
            }
        }
        StatementKind::Expr(expr) => {
            StatementKind::Expr(apply_to_expression(expr, solution))
        }
    };

    Statement::new(kind, stmt.span.clone())
}

/// Apply solution to a pattern.
fn apply_to_pattern(pattern: &Pattern, solution: &Solution) -> Pattern {
    let resolved_ty = resolve_type(&pattern.ty, solution);
    Pattern::new(pattern.kind.clone(), resolved_ty, pattern.span.clone())
}

/// Apply solution to an expression.
fn apply_to_expression(expr: &Expression, solution: &Solution) -> Expression {
    let resolved_ty = resolve_type(&expr.ty, solution);

    let kind = match &expr.kind {
        // Simple cases - just clone the kind
        ExprKind::Literal(lit) => ExprKind::Literal(lit.clone()),
        ExprKind::LocalRef(id) => ExprKind::LocalRef(*id),
        ExprKind::SymbolRef(id) => ExprKind::SymbolRef(*id),
        ExprKind::OverloadedRef(ids) => ExprKind::OverloadedRef(ids.clone()),
        ExprKind::TypeRef(id) => ExprKind::TypeRef(*id),
        ExprKind::TypeParameterRef(id) => ExprKind::TypeParameterRef(*id),
        ExprKind::EnumCase { case_id } => ExprKind::EnumCase { case_id: *case_id },
        ExprKind::ImplicitMemberAccess {
            member_name,
            arguments,
        } => ExprKind::ImplicitMemberAccess {
            member_name: member_name.clone(),
            arguments: arguments
                .as_ref()
                .map(|args| args.iter().map(|arg| apply_to_argument(arg, solution)).collect()),
        },
        ExprKind::Error => ExprKind::Error,
        ExprKind::Break { loop_id, label } => ExprKind::Break {
            loop_id: *loop_id,
            label: label.clone(),
        },
        ExprKind::Continue { loop_id, label } => ExprKind::Continue {
            loop_id: *loop_id,
            label: label.clone(),
        },

        // Compound expressions - recurse
        ExprKind::Array(elements) => {
            ExprKind::Array(elements.iter().map(|e| apply_to_expression(e, solution)).collect())
        }

        ExprKind::Tuple(elements) => {
            ExprKind::Tuple(elements.iter().map(|e| apply_to_expression(e, solution)).collect())
        }

        ExprKind::Grouping(inner) => {
            ExprKind::Grouping(Box::new(apply_to_expression(inner, solution)))
        }

        ExprKind::FieldAccess { object, field } => ExprKind::FieldAccess {
            object: Box::new(apply_to_expression(object, solution)),
            field: field.clone(),
        },

        ExprKind::TupleIndex { tuple, index } => ExprKind::TupleIndex {
            tuple: Box::new(apply_to_expression(tuple, solution)),
            index: *index,
        },

        ExprKind::MethodRef {
            receiver,
            candidates,
            method_name,
        } => ExprKind::MethodRef {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            candidates: candidates.clone(),
            method_name: method_name.clone(),
        },

        ExprKind::Call {
            callee,
            arguments,
            substitutions,
        } => ExprKind::Call {
            callee: Box::new(apply_to_expression(callee, solution)),
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
            substitutions: substitutions.clone(),
        },

        ExprKind::PrimitiveMethodCall {
            receiver,
            method,
            arguments,
        } => ExprKind::PrimitiveMethodCall {
            receiver: Box::new(apply_to_expression(receiver, solution)),
            method: *method,
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
        },

        ExprKind::ImplicitStructInit {
            struct_type,
            arguments,
        } => ExprKind::ImplicitStructInit {
            struct_type: resolve_type(struct_type, solution),
            arguments: arguments
                .iter()
                .map(|arg| apply_to_argument(arg, solution))
                .collect(),
        },

        ExprKind::Assignment { target, value } => ExprKind::Assignment {
            target: Box::new(apply_to_expression(target, solution)),
            value: Box::new(apply_to_expression(value, solution)),
        },

        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => ExprKind::If {
            condition: Box::new(apply_to_expression(condition, solution)),
            then_branch: then_branch
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
            then_value: then_value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
            else_branch: else_branch.as_ref().map(|eb| apply_to_else_branch(eb, solution)),
        },

        ExprKind::While {
            loop_id,
            label,
            condition,
            body,
        } => ExprKind::While {
            loop_id: *loop_id,
            label: label.clone(),
            condition: Box::new(apply_to_expression(condition, solution)),
            body: body.iter().map(|s| apply_to_statement(s, solution)).collect(),
        },

        ExprKind::Loop {
            loop_id,
            label,
            body,
        } => ExprKind::Loop {
            loop_id: *loop_id,
            label: label.clone(),
            body: body.iter().map(|s| apply_to_statement(s, solution)).collect(),
        },

        ExprKind::Return { value } => ExprKind::Return {
            value: value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
        },

        ExprKind::Closure {
            params,
            body,
            tail_expr,
            captures,
            uses_it,
            implicit_param,
        } => {
            // Apply solution to closure parameters
            let resolved_params = params.as_ref().map(|ps| {
                ps.iter()
                    .map(|p| kestrel_semantic_tree::expr::ClosureParam {
                        name: p.name.clone(),
                        ty: resolve_type(&p.ty, solution),
                        is_type_annotated: p.is_type_annotated,
                        span: p.span.clone(),
                    })
                    .collect()
            });

            // Apply solution to body statements
            let resolved_body = body.iter().map(|s| apply_to_statement(s, solution)).collect();

            // Apply solution to tail expression
            let resolved_tail = tail_expr
                .as_ref()
                .map(|e| Box::new(apply_to_expression(e, solution)));

            // Apply solution to captures
            let resolved_captures = captures
                .iter()
                .map(|c| kestrel_semantic_tree::expr::Capture {
                    local_id: c.local_id,
                    name: c.name.clone(),
                    ty: resolve_type(&c.ty, solution),
                    kind: c.kind,
                    span: c.span.clone(),
                })
                .collect();

            // Apply solution to implicit_param
            let resolved_implicit_param = implicit_param.as_ref().map(|(id, ty, span)| {
                (*id, resolve_type(ty, solution), span.clone())
            });

            ExprKind::Closure {
                params: resolved_params,
                body: resolved_body,
                tail_expr: resolved_tail,
                captures: resolved_captures,
                uses_it: *uses_it,
                implicit_param: resolved_implicit_param,
            }
        }
    };

    Expression::new(kind, resolved_ty, expr.span.clone(), expr.mutable)
}

/// Apply solution to a call argument.
fn apply_to_argument(arg: &CallArgument, solution: &Solution) -> CallArgument {
    CallArgument {
        label: arg.label.clone(),
        value: apply_to_expression(&arg.value, solution),
        span: arg.span.clone(),
    }
}

/// Apply solution to an else branch.
fn apply_to_else_branch(branch: &ElseBranch, solution: &Solution) -> ElseBranch {
    match branch {
        ElseBranch::Block { statements, value } => ElseBranch::Block {
            statements: statements
                .iter()
                .map(|s| apply_to_statement(s, solution))
                .collect(),
            value: value
                .as_ref()
                .map(|v| Box::new(apply_to_expression(v, solution))),
        },
        ElseBranch::ElseIf(if_expr) => {
            ElseBranch::ElseIf(Box::new(apply_to_expression(if_expr, solution)))
        }
    }
}

/// Resolve a type using the solution.
/// If the type is an inference placeholder, look it up in the solution.
/// Recursively resolves nested types.
fn resolve_type(ty: &Ty, solution: &Solution) -> Ty {
    // If this type is an inference placeholder, look it up
    if matches!(ty.kind(), TyKind::Infer) {
        if let Some(resolved) = solution.get_type(ty.id()) {
            return resolve_type(resolved, solution);
        }
    }

    // For compound types, recursively resolve components
    match ty.kind() {
        TyKind::Tuple(elements) => {
            let resolved_elements: Vec<_> = elements
                .iter()
                .map(|e| resolve_type(e, solution))
                .collect();
            Ty::tuple(resolved_elements, ty.span().clone())
        }
        TyKind::Array(elem) => {
            let resolved_elem = resolve_type(elem, solution);
            Ty::array(resolved_elem, ty.span().clone())
        }
        TyKind::Function {
            params,
            return_type,
        } => {
            let resolved_params: Vec<_> = params
                .iter()
                .map(|p| resolve_type(p, solution))
                .collect();
            let resolved_return = resolve_type(return_type, solution);
            Ty::function(resolved_params, resolved_return, ty.span().clone())
        }
        TyKind::UnresolvedFunction {
            param_info,
            return_type,
        } => {
            let resolved_return = resolve_type(return_type, solution);
            let resolved_param_info = match param_info {
                ParamInfo::Unconstrained => ParamInfo::Unconstrained,
                ParamInfo::ImplicitIt { it_type } => ParamInfo::ImplicitIt {
                    it_type: Box::new(resolve_type(it_type, solution)),
                },
                ParamInfo::Explicit { param_types } => ParamInfo::Explicit {
                    param_types: param_types
                        .iter()
                        .map(|p| resolve_type(p, solution))
                        .collect(),
                },
            };
            Ty::unresolved_function(resolved_param_info, resolved_return, ty.span().clone())
        }
        // Nominal types with substitutions would need recursive resolution too,
        // but for now just return the original type
        _ => ty.clone(),
    }
}

#[cfg(test)]
mod tests {
    // Integration tests would require setting up expressions and solutions
}
