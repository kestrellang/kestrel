//! Validator for dead code detection
//!
//! This validator detects unreachable code:
//! - Code after `return`
//! - Code after `break` or `continue`
//! - Code after infinite `loop { }` (loop without break)
//!
//! These are reported as warnings, not errors.

use std::sync::Arc;

use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::validation::{SymbolContext, Validator};

/// Validator for detecting unreachable (dead) code
pub struct DeadCodeValidator;

impl DeadCodeValidator {
    const NAME: &'static str = "dead_code";

    pub fn new() -> Self {
        Self
    }
}

impl Default for DeadCodeValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for DeadCodeValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check functions and initializers
        if !matches!(
            kind,
            KestrelSymbolKind::Function | KestrelSymbolKind::Initializer
        ) {
            return;
        }

        // Get the executable behavior (body)
        let Some(body) = get_executable_body(ctx.symbol) else {
            return;
        };

        // Analyze the body for dead code
        let mut errors = Vec::new();
        analyze_block(&body.statements, body.yield_expr.as_deref(), &mut errors);

        // Report warnings
        for error in errors {
            ctx.diagnostics()
                .get()
                .add_diagnostic(error.into_diagnostic(ctx.file_id));
        }
    }
}

/// Warning for unreachable code
struct UnreachableCodeWarning {
    /// Span of the unreachable code
    span: Span,
    /// What caused the code to be unreachable
    reason: UnreachableReason,
}

#[derive(Clone, Copy)]
enum UnreachableReason {
    AfterReturn,
    AfterBreak,
    AfterContinue,
    AfterInfiniteLoop,
}

impl UnreachableReason {
    fn description(&self) -> &'static str {
        match self {
            UnreachableReason::AfterReturn => "after return statement",
            UnreachableReason::AfterBreak => "after break statement",
            UnreachableReason::AfterContinue => "after continue statement",
            UnreachableReason::AfterInfiniteLoop => "after infinite loop",
        }
    }
}

impl IntoDiagnostic for UnreachableCodeWarning {
    fn into_diagnostic(&self, file_id: usize) -> Diagnostic<usize> {
        Diagnostic::warning()
            .with_message(format!("unreachable code {}", self.reason.description()))
            .with_labels(vec![
                Label::primary(file_id, self.span.clone()).with_message("this code will never execute")
            ])
    }
}

/// Result of analyzing divergence in code
#[derive(Clone, Copy, PartialEq, Eq)]
enum Divergence {
    /// Code continues normally
    None,
    /// Code always returns
    Returns,
    /// Code always breaks (from current loop)
    Breaks,
    /// Code always continues (current loop)
    Continues,
    /// Code is an infinite loop
    InfiniteLoop,
}

impl Divergence {
    fn to_reason(self) -> Option<UnreachableReason> {
        match self {
            Divergence::None => None,
            Divergence::Returns => Some(UnreachableReason::AfterReturn),
            Divergence::Breaks => Some(UnreachableReason::AfterBreak),
            Divergence::Continues => Some(UnreachableReason::AfterContinue),
            Divergence::InfiniteLoop => Some(UnreachableReason::AfterInfiniteLoop),
        }
    }

    /// Check if this divergence means code after is unreachable
    fn diverges(self) -> bool {
        self != Divergence::None
    }
}

/// Analyze a block of statements for dead code
fn analyze_block(
    statements: &[Statement],
    yield_expr: Option<&Expression>,
    errors: &mut Vec<UnreachableCodeWarning>,
) -> Divergence {
    let mut divergence = Divergence::None;

    for (i, stmt) in statements.iter().enumerate() {
        if divergence.diverges() {
            // This statement is unreachable
            if let Some(reason) = divergence.to_reason() {
                errors.push(UnreachableCodeWarning {
                    span: stmt.span.clone(),
                    reason,
                });
            }
            // Don't report more errors for subsequent statements in this block
            // (one warning per unreachable section is enough)
            break;
        }

        divergence = analyze_statement(stmt, errors);
    }

    // Check yield expression
    if let Some(yield_expr) = yield_expr {
        if divergence.diverges() {
            if let Some(reason) = divergence.to_reason() {
                errors.push(UnreachableCodeWarning {
                    span: yield_expr.span.clone(),
                    reason,
                });
            }
        } else {
            divergence = analyze_expression(yield_expr, errors);
        }
    }

    divergence
}

/// Analyze a statement for divergence
fn analyze_statement(stmt: &Statement, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &stmt.kind {
        StatementKind::Binding { value: Some(expr), .. } => {
            analyze_expression(expr, errors)
        }
        StatementKind::Binding { value: None, .. } => Divergence::None,
        StatementKind::Expr(expr) => analyze_expression(expr, errors),
    }
}

/// Analyze an expression for divergence
fn analyze_expression(expr: &Expression, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &expr.kind {
        ExprKind::Return { value } => {
            // Analyze the return value for nested dead code
            if let Some(val) = value {
                analyze_expression(val, errors);
            }
            Divergence::Returns
        }

        ExprKind::Break { .. } => Divergence::Breaks,

        ExprKind::Continue { .. } => Divergence::Continues,

        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Analyze condition
            let cond_div = analyze_expression(condition, errors);
            if cond_div.diverges() {
                return cond_div;
            }

            // Analyze then branch
            let then_div = analyze_block(then_branch, then_value.as_deref(), errors);

            // Analyze else branch
            let else_div = if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        analyze_block(statements, value.as_deref(), errors)
                    }
                    ElseBranch::ElseIf(if_expr) => analyze_expression(if_expr, errors),
                }
            } else {
                // No else branch - doesn't diverge on the else path
                Divergence::None
            };

            // If expression diverges only if BOTH branches diverge with same type
            // For simplicity, we only propagate if both return (the common case)
            if then_div == Divergence::Returns && else_div == Divergence::Returns {
                Divergence::Returns
            } else {
                Divergence::None
            }
        }

        ExprKind::While { condition, body, .. } => {
            // Analyze condition
            let cond_div = analyze_expression(condition, errors);
            if cond_div.diverges() {
                return cond_div;
            }

            // Analyze body for nested dead code
            // Track divergence within the loop body to detect dead code
            let mut body_divergence = Divergence::None;
            for stmt in body {
                if body_divergence.diverges() {
                    // Dead code inside the while loop body
                    if let Some(reason) = body_divergence.to_reason() {
                        errors.push(UnreachableCodeWarning {
                            span: stmt.span.clone(),
                            reason,
                        });
                    }
                    break;
                }
                body_divergence = analyze_statement(stmt, errors);
            }

            // While loops don't propagate divergence to code after the loop
            // (the loop body might not execute at all)
            Divergence::None
        }

        ExprKind::Loop { body, .. } => {
            // Analyze body
            let mut body_divergence = Divergence::None;
            let mut has_break = false;

            for stmt in body {
                if body_divergence.diverges() {
                    // Dead code inside loop
                    if let Some(reason) = body_divergence.to_reason() {
                        errors.push(UnreachableCodeWarning {
                            span: stmt.span.clone(),
                            reason,
                        });
                    }
                    break;
                }

                body_divergence = analyze_statement(stmt, errors);

                // Check if this statement contains a break
                if statement_contains_break(&stmt.kind) {
                    has_break = true;
                }
            }

            // If all paths return, propagate that
            if body_divergence == Divergence::Returns {
                return Divergence::Returns;
            }

            // If no breaks, it's an infinite loop
            if !has_break && body_divergence != Divergence::Returns {
                Divergence::InfiniteLoop
            } else {
                Divergence::None
            }
        }

        // Expressions that may contain nested code
        ExprKind::Call { callee, arguments } => {
            let div = analyze_expression(callee, errors);
            if div.diverges() {
                return div;
            }
            for arg in arguments {
                let div = analyze_expression(&arg.value, errors);
                if div.diverges() {
                    return div;
                }
            }
            Divergence::None
        }

        ExprKind::Assignment { target, value } => {
            let div = analyze_expression(value, errors);
            if div.diverges() {
                return div;
            }
            analyze_expression(target, errors)
        }

        ExprKind::Grouping(inner) => analyze_expression(inner, errors),

        ExprKind::Array(elements) => {
            for elem in elements {
                let div = analyze_expression(elem, errors);
                if div.diverges() {
                    return div;
                }
            }
            Divergence::None
        }

        ExprKind::Tuple(elements) => {
            for elem in elements {
                let div = analyze_expression(elem, errors);
                if div.diverges() {
                    return div;
                }
            }
            Divergence::None
        }

        ExprKind::FieldAccess { object, .. } => analyze_expression(object, errors),

        ExprKind::TupleIndex { tuple, .. } => analyze_expression(tuple, errors),

        ExprKind::MethodRef { receiver, .. } => analyze_expression(receiver, errors),

        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            let div = analyze_expression(receiver, errors);
            if div.diverges() {
                return div;
            }
            for arg in arguments {
                let div = analyze_expression(&arg.value, errors);
                if div.diverges() {
                    return div;
                }
            }
            Divergence::None
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                let div = analyze_expression(&arg.value, errors);
                if div.diverges() {
                    return div;
                }
            }
            Divergence::None
        }

        // Leaf expressions - no divergence
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::Error => Divergence::None,
    }
}

/// Check if a statement contains a break (at top level, not in nested loop)
fn statement_contains_break(kind: &StatementKind) -> bool {
    match kind {
        StatementKind::Expr(expr) => expr_contains_break(&expr.kind),
        StatementKind::Binding { value: Some(expr), .. } => expr_contains_break(&expr.kind),
        StatementKind::Binding { value: None, .. } => false,
    }
}

/// Check if an expression contains a break at top level
fn expr_contains_break(kind: &ExprKind) -> bool {
    match kind {
        ExprKind::Break { .. } => true,
        ExprKind::If { then_branch, then_value, else_branch, .. } => {
            // Check then branch
            for stmt in then_branch {
                if statement_contains_break(&stmt.kind) {
                    return true;
                }
            }
            if let Some(val) = then_value {
                if expr_contains_break(&val.kind) {
                    return true;
                }
            }
            // Check else branch
            if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        for stmt in statements {
                            if statement_contains_break(&stmt.kind) {
                                return true;
                            }
                        }
                        if let Some(val) = value {
                            if expr_contains_break(&val.kind) {
                                return true;
                            }
                        }
                    }
                    ElseBranch::ElseIf(if_expr) => {
                        if expr_contains_break(&if_expr.kind) {
                            return true;
                        }
                    }
                }
            }
            false
        }
        // Don't recurse into nested loops - breaks there don't affect outer loop
        ExprKind::While { .. } | ExprKind::Loop { .. } => false,
        _ => false,
    }
}

/// Get the executable body from a symbol
fn get_executable_body(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<CodeBlock> {
    let behaviors = symbol.metadata().behaviors();
    for b in behaviors.iter() {
        if matches!(b.kind(), KestrelBehaviorKind::Executable) {
            if let Some(exec) = b.as_ref().downcast_ref::<ExecutableBehavior>() {
                return Some(exec.body().clone());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_divergence_to_reason() {
        assert!(Divergence::None.to_reason().is_none());
        assert!(matches!(
            Divergence::Returns.to_reason(),
            Some(UnreachableReason::AfterReturn)
        ));
        assert!(matches!(
            Divergence::Breaks.to_reason(),
            Some(UnreachableReason::AfterBreak)
        ));
        assert!(matches!(
            Divergence::Continues.to_reason(),
            Some(UnreachableReason::AfterContinue)
        ));
        assert!(matches!(
            Divergence::InfiniteLoop.to_reason(),
            Some(UnreachableReason::AfterInfiniteLoop)
        ));
    }

    #[test]
    fn test_divergence_diverges() {
        assert!(!Divergence::None.diverges());
        assert!(Divergence::Returns.diverges());
        assert!(Divergence::Breaks.diverges());
        assert!(Divergence::Continues.diverges());
        assert!(Divergence::InfiniteLoop.diverges());
    }
}
