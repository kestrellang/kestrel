//! Closure semantic analysis.

mod diagnostics;

pub use diagnostics::*;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;
use kestrel_semantic_model::LocalName;
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::ty::TyKind;

pub struct ClosureAnalyzer;

impl ClosureAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ClosureAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ClosureAnalyzer {
    fn name(&self) -> &'static str {
        "closure"
    }

    fn visit_expression(&mut self, expr: &Expression, ctx: &mut AnalysisContext) {
        let ExprKind::Closure {
            params,
            body,
            tail_expr,
            captures,
            uses_it,
            ..
        } = &expr.kind
        else {
            return;
        };

        // Get the container symbol for looking up local names
        let container_id = ctx.current_symbol().map(|s| s.metadata().id());

        // Check for `it` usage with wrong arity
        // `it` is used when params is None (implicit syntax)
        // We need to check if the expected arity allows `it`
        if *uses_it {
            validate_it_usage(expr, ctx);
        }

        // Validate closure type matches expected type
        validate_closure_type(expr, params, tail_expr, ctx);

        // Check for assignment to captured variables
        validate_capture_assignments(body, tail_expr.as_deref(), captures, container_id, ctx);

        // Check for assignment to closure parameters
        validate_parameter_assignments(expr, params, container_id, ctx);
    }
}

/// Validate that `it` is only used when the closure has exactly 1 parameter.
fn validate_it_usage(expr: &Expression, ctx: &mut AnalysisContext) {
    // Check the closure's type to see what the expected arity is
    if let TyKind::Function { params, .. } = expr.ty.kind() {
        let expected_arity = params.len();

        // `it` can only be used when arity is exactly 1
        if expected_arity != 1 {
            ctx.report(ItUsedWithWrongArityError {
                span: expr.span.clone(),
                expected_arity,
            });
        }
    }
}

/// Validate that the closure type is compatible with the expected type.
fn validate_closure_type(
    expr: &Expression,
    params: &Option<Vec<kestrel_semantic_tree::expr::ClosureParam>>,
    tail_expr: &Option<Box<Expression>>,
    ctx: &mut AnalysisContext,
) {
    // Extract the closure's function type
    if let TyKind::Function {
        params: param_tys,
        return_type: return_ty,
    } = expr.ty.kind()
    {
        // Check parameter count matches
        if let Some(param_list) = params {
            let actual_count = param_list.len();
            let expected_count = param_tys.len();

            if actual_count != expected_count {
                ctx.report(ClosureArityMismatchError {
                    span: expr.span.clone(),
                    actual: actual_count,
                    expected: expected_count,
                });
                return; // Don't check param types if count mismatch
            }

            // Check parameter types match
            for (i, (param, expected_ty)) in param_list.iter().zip(param_tys.iter()).enumerate() {
                // Only check if the parameter has a concrete type (not Infer)
                if !param.ty.is_infer() && param.ty.id() != expected_ty.id() {
                    ctx.report(ClosureParamTypeMismatchError {
                        span: param.span.clone(),
                        index: i,
                        actual: param.ty.to_string(),
                        expected: expected_ty.to_string(),
                    });
                }
            }
        }

        // Return type compatibility is already validated by the type inference solver.
        // The solver generates constraints during type inference that ensure the tail
        // expression's type unifies with the closure's return type. This analyzer runs
        // after type inference completes, so we trust those results.
    }
}

/// Validate that captured variables are not assigned to.
fn validate_capture_assignments(
    body: &[kestrel_semantic_tree::stmt::Statement],
    tail_expr: Option<&Expression>,
    captures: &[kestrel_semantic_tree::expr::Capture],
    container_id: Option<semantic_tree::symbol::SymbolId>,
    ctx: &mut AnalysisContext,
) {
    if captures.is_empty() {
        return;
    }

    // Build a set of captured local IDs for quick lookup
    let captured_ids: std::collections::HashSet<_> = captures.iter().map(|c| c.local_id).collect();

    // Walk the closure body to find assignments to captured variables
    for stmt in body {
        walk_statement_for_assignments(stmt, &captured_ids, container_id, ctx);
    }

    // Check the tail expression too
    if let Some(tail) = tail_expr {
        find_assignments_to_locals(tail, &captured_ids, container_id, ctx);
    }
}

/// Validate that closure parameters are not assigned to.
fn validate_parameter_assignments(
    expr: &Expression,
    params: &Option<Vec<kestrel_semantic_tree::expr::ClosureParam>>,
    container_id: Option<semantic_tree::symbol::SymbolId>,
    ctx: &mut AnalysisContext,
) {
    // Closure parameters are immutable, so we need to check for assignments to them
    // This would require walking the closure body and detecting assignments

    // Build a set of parameter names for detection
    if let Some(param_list) = params {
        let _param_names: Vec<_> = param_list.iter().map(|p| p.name.clone()).collect();

        // Similar to capture validation, we'd need to walk the closure body
        // and check for assignments to these parameters
        // When found, report CannotAssignToClosureParameterError

        // This is a placeholder for the full implementation
        let _ = (expr, container_id, ctx);
    }
}

/// Walk an expression tree to find assignments to specific local IDs.
fn find_assignments_to_locals(
    expr: &Expression,
    target_locals: &std::collections::HashSet<kestrel_semantic_tree::symbol::local::LocalId>,
    container_id: Option<semantic_tree::symbol::SymbolId>,
    ctx: &mut AnalysisContext,
) {
    match &expr.kind {
        ExprKind::Assignment { target, value } => {
            // Check if the target is a LocalRef that matches one of our target locals
            if let ExprKind::LocalRef(local_id) = &target.kind {
                if target_locals.contains(local_id) {
                    // Get the variable name
                    if let Some(cid) = container_id {
                        let name = ctx
                            .model
                            .query(LocalName {
                                container_id: cid,
                                local_id: *local_id,
                            })
                            .unwrap_or_else(|| "<unknown>".to_string());

                        ctx.report(CannotAssignToCapturedVariableError {
                            span: target.span.clone(),
                            name,
                        });
                    }
                }
            }

            // Continue walking the value expression
            find_assignments_to_locals(value, target_locals, container_id, ctx);
        }

        // Recursively check other expression kinds
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Process conditions
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        find_assignments_to_locals(expr, target_locals, container_id, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        find_assignments_to_locals(value, target_locals, container_id, ctx);
                    }
                }
            }

            for stmt in then_branch {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }

            if let Some(then_val) = then_value {
                find_assignments_to_locals(then_val, target_locals, container_id, ctx);
            }

            if let Some(else_br) = else_branch {
                match else_br {
                    kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
                        }
                        if let Some(val) = value {
                            find_assignments_to_locals(val, target_locals, container_id, ctx);
                        }
                    }
                    kestrel_semantic_tree::expr::ElseBranch::ElseIf(if_expr) => {
                        find_assignments_to_locals(if_expr, target_locals, container_id, ctx);
                    }
                }
            }
        }

        ExprKind::While {
            condition, body, ..
        } => {
            find_assignments_to_locals(condition, target_locals, container_id, ctx);
            for stmt in body {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }
        }

        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        find_assignments_to_locals(expr, target_locals, container_id, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        find_assignments_to_locals(value, target_locals, container_id, ctx);
                    }
                }
            }
            for stmt in body {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }
        }

        ExprKind::Loop { body, .. } => {
            for stmt in body {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }
        }

        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // Walk nested closure body
            for stmt in body {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }
            if let Some(tail) = tail_expr {
                find_assignments_to_locals(tail, target_locals, container_id, ctx);
            }
        }

        // Other expression kinds that contain sub-expressions
        ExprKind::Array(elements) | ExprKind::Tuple(elements) => {
            for elem in elements {
                find_assignments_to_locals(elem, target_locals, container_id, ctx);
            }
        }

        ExprKind::Grouping(inner) => {
            find_assignments_to_locals(inner, target_locals, container_id, ctx);
        }

        ExprKind::FieldAccess { object, .. } => {
            find_assignments_to_locals(object, target_locals, container_id, ctx);
        }

        ExprKind::TupleIndex { tuple, .. } => {
            find_assignments_to_locals(tuple, target_locals, container_id, ctx);
        }

        ExprKind::MethodRef { receiver, .. } => {
            find_assignments_to_locals(receiver, target_locals, container_id, ctx);
        }

        ExprKind::PrimitiveMethodRef { receiver, .. } => {
            find_assignments_to_locals(receiver, target_locals, container_id, ctx);
        }

        ExprKind::Call {
            callee, arguments, ..
        } => {
            find_assignments_to_locals(callee, target_locals, container_id, ctx);
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            find_assignments_to_locals(receiver, target_locals, container_id, ctx);
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            find_assignments_to_locals(receiver, target_locals, container_id, ctx);
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        ExprKind::DelegatingInit { arguments, .. } => {
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        ExprKind::Return { value } => {
            if let Some(val) = value {
                find_assignments_to_locals(val, target_locals, container_id, ctx);
            }
        }

        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
                }
            }
        }

        // Lang intrinsics - walk arguments
        ExprKind::LangIntrinsic { arguments, .. } => {
            for arg in arguments {
                find_assignments_to_locals(&arg.value, target_locals, container_id, ctx);
            }
        }

        // Leaf expressions - no sub-expressions to check
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Break { .. }
        | ExprKind::Continue { .. }
        | ExprKind::LangIntrinsicRef(_)
        | ExprKind::Error => {}

        ExprKind::Match { scrutinee, arms } => {
            find_assignments_to_locals(scrutinee, target_locals, container_id, ctx);
            for arm in arms {
                if let Some(guard) = &arm.guard {
                    find_assignments_to_locals(guard, target_locals, container_id, ctx);
                }
                find_assignments_to_locals(&arm.body, target_locals, container_id, ctx);
            }
        }

        ExprKind::Block { statements, value } => {
            for stmt in statements {
                walk_statement_for_assignments(stmt, target_locals, container_id, ctx);
            }
            if let Some(val) = value {
                find_assignments_to_locals(val, target_locals, container_id, ctx);
            }
        }
    }
}

/// Walk a statement to find assignments.
fn walk_statement_for_assignments(
    stmt: &kestrel_semantic_tree::stmt::Statement,
    target_locals: &std::collections::HashSet<kestrel_semantic_tree::symbol::local::LocalId>,
    container_id: Option<semantic_tree::symbol::SymbolId>,
    ctx: &mut AnalysisContext,
) {
    match &stmt.kind {
        kestrel_semantic_tree::stmt::StatementKind::Binding { value, .. } => {
            if let Some(val) = value {
                find_assignments_to_locals(val, target_locals, container_id, ctx);
            }
        }
        kestrel_semantic_tree::stmt::StatementKind::Expr(expr) => {
            find_assignments_to_locals(expr, target_locals, container_id, ctx);
        }
        kestrel_semantic_tree::stmt::StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        find_assignments_to_locals(expr, target_locals, container_id, ctx);
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        find_assignments_to_locals(value, target_locals, container_id, ctx);
                    }
                }
            }
            for else_stmt in &else_block.statements {
                walk_statement_for_assignments(else_stmt, target_locals, container_id, ctx);
            }
            if let Some(yield_expr) = &else_block.yield_expr {
                find_assignments_to_locals(yield_expr, target_locals, container_id, ctx);
            }
        }
        kestrel_semantic_tree::stmt::StatementKind::Deinit { .. } => {
            // Deinit statement has no expressions that could contain assignments
        }
    }
}
