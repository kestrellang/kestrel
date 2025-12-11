//! Validator for exhaustive return analysis
//!
//! This validator ensures that functions with non-unit return types
//! return a value on all code paths.
//!
//! A function is valid if:
//! - It returns `()` (unit) - no explicit return needed
//! - All code paths end with a return expression
//! - All code paths end with a value expression (implicit return)
//! - All code paths diverge (infinite loop, etc.)

use std::sync::Arc;

use kestrel_reporting::{Diagnostic, DiagnosticContext, IntoDiagnostic, Label};
use kestrel_semantic_tree::behavior::executable::{CodeBlock, ExecutableBehavior};
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use kestrel_span::Span;
use semantic_tree::symbol::Symbol;

use crate::validation::{SymbolContext, Validator};

/// Validator for exhaustive return analysis
pub struct ExhaustiveReturnValidator;

impl ExhaustiveReturnValidator {
    const NAME: &'static str = "exhaustive_return";

    pub fn new() -> Self {
        Self
    }
}

impl Default for ExhaustiveReturnValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for ExhaustiveReturnValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check functions (not initializers - they return self implicitly)
        if kind != KestrelSymbolKind::Function {
            return;
        }

        // Get the function symbol to check return type
        let symbol_ref: &dyn Symbol<KestrelLanguage> = ctx.symbol.as_ref();
        let Some(func) = symbol_ref.as_any().downcast_ref::<FunctionSymbol>() else {
            return;
        };

        // Check if return type is unit - if so, no explicit return needed
        let return_ty = func.return_type();
        if is_unit_type(return_ty.kind()) {
            return;
        }

        // Get the executable behavior (body)
        let Some(body) = get_executable_body(ctx.symbol) else {
            return; // No body - handled by function_body validator
        };

        // Analyze the body for exhaustive returns
        let result = analyze_block(&body.statements, body.yield_expr.as_deref());

        // Check if all paths return/diverge
        if !result.definitely_returns() {
            let func_name = ctx.symbol.metadata().name().value.clone();
            let span = ctx.symbol.metadata().declaration_span().clone();

            ctx.diagnostics().get().add_diagnostic(
                MissingReturnError { span, func_name }.into_diagnostic(),
            );
        }
    }
}

/// Check if a type is unit
fn is_unit_type(kind: &TyKind) -> bool {
    match kind {
        TyKind::Unit => true,
        TyKind::Tuple(elements) => elements.is_empty(),
        _ => false,
    }
}

/// Error for missing return on some code path
struct MissingReturnError {
    span: Span,
    func_name: String,
}

impl IntoDiagnostic for MissingReturnError {
    fn into_diagnostic(&self) -> Diagnostic<usize> {
        Diagnostic::error()
            .with_message(format!(
                "function '{}' does not return a value on all code paths",
                self.func_name
            ))
            .with_labels(vec![
                Label::primary(self.span.file_id, self.span.range())
                    .with_message("this function has a non-unit return type")
            ])
            .with_notes(vec![
                "all code paths must end with a return statement or a value expression".to_string()
            ])
    }
}

/// Result of analyzing a code path for returns
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnState {
    /// Path ends with a return or value expression
    Returns,
    /// Path diverges (infinite loop, break, continue - never reaches end)
    Diverges,
    /// Path may fall through without returning
    MayFallThrough,
}

impl ReturnState {
    /// Check if this state means the function definitely returns (or diverges)
    fn definitely_returns(self) -> bool {
        matches!(self, ReturnState::Returns | ReturnState::Diverges)
    }

    /// Merge two states from different branches (e.g., if/else)
    /// Returns the "worst case" - if either branch may fall through, the result may fall through
    fn merge(self, other: ReturnState) -> ReturnState {
        match (self, other) {
            // If both return or diverge, we're good
            (ReturnState::Returns, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Returns, ReturnState::Diverges) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Diverges) => ReturnState::Diverges,
            // If either may fall through, result may fall through
            (ReturnState::MayFallThrough, _) => ReturnState::MayFallThrough,
            (_, ReturnState::MayFallThrough) => ReturnState::MayFallThrough,
        }
    }
}

/// Analyze a block of statements for exhaustive returns
fn analyze_block(statements: &[Statement], yield_expr: Option<&Expression>) -> ReturnState {
    let mut state = ReturnState::MayFallThrough;

    for stmt in statements {
        if state.definitely_returns() {
            // Already returned/diverged - rest is dead code (handled by dead_code validator)
            return state;
        }
        state = analyze_statement(stmt);
    }

    // If we haven't returned yet, check the yield expression
    if !state.definitely_returns() {
        if let Some(expr) = yield_expr {
            // A yield expression counts as returning a value
            let expr_state = analyze_expression(expr);
            if expr_state.definitely_returns() {
                return expr_state;
            }
            // The yield expression itself is a value - this counts as returning
            return ReturnState::Returns;
        }
    }

    state
}

/// Analyze a statement for returns
fn analyze_statement(stmt: &Statement) -> ReturnState {
    match &stmt.kind {
        StatementKind::Binding { value: Some(expr), .. } => {
            // If the initializer diverges, the binding never completes
            analyze_expression(expr)
        }
        StatementKind::Binding { value: None, .. } => ReturnState::MayFallThrough,
        StatementKind::Expr(expr) => analyze_expression(expr),
    }
}

/// Analyze an expression for returns
fn analyze_expression(expr: &Expression) -> ReturnState {
    match &expr.kind {
        ExprKind::Return { .. } => ReturnState::Returns,

        ExprKind::Break { .. } | ExprKind::Continue { .. } => ReturnState::Diverges,

        ExprKind::If {
            condition,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Check condition first
            let cond_state = analyze_expression(condition);
            if cond_state.definitely_returns() {
                return cond_state;
            }

            // Analyze then branch
            let then_state = analyze_block(then_branch, then_value.as_deref());

            // Analyze else branch
            let else_state = if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        analyze_block(statements, value.as_deref())
                    }
                    ElseBranch::ElseIf(if_expr) => analyze_expression(if_expr),
                }
            } else {
                // No else branch - the "else" path falls through
                ReturnState::MayFallThrough
            };

            // Merge both branches
            then_state.merge(else_state)
        }

        ExprKind::While { condition, body, .. } => {
            // Check condition
            let cond_state = analyze_expression(condition);
            if cond_state.definitely_returns() {
                return cond_state;
            }

            // While loop body might not execute at all, so we can't rely on returns inside
            // But we still need to analyze it in case it always returns when it does execute
            // For exhaustive return purposes, while loops don't guarantee a return
            let _body_state = analyze_block(body, None);

            // While loops may not execute, so they don't guarantee a return
            ReturnState::MayFallThrough
        }

        ExprKind::Loop { body, .. } => {
            // Analyze the loop body
            let mut body_state = ReturnState::MayFallThrough;
            let mut has_break = false;

            for stmt in body {
                if body_state.definitely_returns() {
                    break;
                }
                body_state = analyze_statement(stmt);

                // Check if this statement contains a break
                if statement_contains_break(&stmt.kind) {
                    has_break = true;
                }
            }

            // If all paths in the body return, the loop always returns
            if body_state == ReturnState::Returns {
                return ReturnState::Returns;
            }

            // If no breaks and body doesn't return, it's an infinite loop (diverges)
            if !has_break && body_state != ReturnState::Returns {
                return ReturnState::Diverges;
            }

            // Loop has breaks - code after loop is reachable, and we may fall through
            ReturnState::MayFallThrough
        }

        // Expressions that may contain nested returns
        ExprKind::Call { callee, arguments, .. } => {
            let state = analyze_expression(callee);
            if state.definitely_returns() {
                return state;
            }
            for arg in arguments {
                let state = analyze_expression(&arg.value);
                if state.definitely_returns() {
                    return state;
                }
            }
            ReturnState::MayFallThrough
        }

        ExprKind::Assignment { target, value } => {
            let state = analyze_expression(value);
            if state.definitely_returns() {
                return state;
            }
            analyze_expression(target)
        }

        ExprKind::Grouping(inner) => analyze_expression(inner),

        ExprKind::Array(elements) => {
            for elem in elements {
                let state = analyze_expression(elem);
                if state.definitely_returns() {
                    return state;
                }
            }
            ReturnState::MayFallThrough
        }

        ExprKind::Tuple(elements) => {
            for elem in elements {
                let state = analyze_expression(elem);
                if state.definitely_returns() {
                    return state;
                }
            }
            ReturnState::MayFallThrough
        }

        ExprKind::FieldAccess { object, .. } => analyze_expression(object),

        ExprKind::TupleIndex { tuple, .. } => analyze_expression(tuple),

        ExprKind::MethodRef { receiver, .. } => analyze_expression(receiver),

        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            let state = analyze_expression(receiver);
            if state.definitely_returns() {
                return state;
            }
            for arg in arguments {
                let state = analyze_expression(&arg.value);
                if state.definitely_returns() {
                    return state;
                }
            }
            ReturnState::MayFallThrough
        }

        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                let state = analyze_expression(&arg.value);
                if state.definitely_returns() {
                    return state;
                }
            }
            ReturnState::MayFallThrough
        }

        // Leaf expressions - don't return, don't diverge
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::Error => ReturnState::MayFallThrough,
    }
}

/// Check if a statement contains a break at top level (not in nested loop)
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
        // Don't recurse into nested loops
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
    use kestrel_span::Span;
    use super::*;

    #[test]
    fn test_return_state_merge() {
        // Both return = returns
        assert_eq!(
            ReturnState::Returns.merge(ReturnState::Returns),
            ReturnState::Returns
        );

        // Return + diverge = returns
        assert_eq!(
            ReturnState::Returns.merge(ReturnState::Diverges),
            ReturnState::Returns
        );
        assert_eq!(
            ReturnState::Diverges.merge(ReturnState::Returns),
            ReturnState::Returns
        );

        // Both diverge = diverges
        assert_eq!(
            ReturnState::Diverges.merge(ReturnState::Diverges),
            ReturnState::Diverges
        );

        // Any + fall through = fall through
        assert_eq!(
            ReturnState::Returns.merge(ReturnState::MayFallThrough),
            ReturnState::MayFallThrough
        );
        assert_eq!(
            ReturnState::MayFallThrough.merge(ReturnState::Returns),
            ReturnState::MayFallThrough
        );
    }

    #[test]
    fn test_definitely_returns() {
        assert!(ReturnState::Returns.definitely_returns());
        assert!(ReturnState::Diverges.definitely_returns());
        assert!(!ReturnState::MayFallThrough.definitely_returns());
    }
}
