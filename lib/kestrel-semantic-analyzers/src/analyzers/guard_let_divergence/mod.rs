//! Guard-let divergence analyzer.
//!
//! Verifies that guard-let else blocks always diverge (return, break, continue).

use std::sync::Arc;

use kestrel_reporting::IntoDiagnostic;
use kestrel_semantic_model::ExecutableBodyFor;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::GuardLetElseMustDivergeError;

pub struct GuardLetDivergenceAnalyzer;

impl GuardLetDivergenceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GuardLetDivergenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for GuardLetDivergenceAnalyzer {
    fn name(&self) -> &'static str {
        "guard_let_divergence"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only check functions
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        // Downcast to FunctionSymbol
        let Ok(_func) = symbol.clone().downcast_arc::<FunctionSymbol>() else {
            return;
        };

        let symbol_id = symbol.metadata().id();
        let Some(body) = ctx.model.query(ExecutableBodyFor { symbol_id }) else {
            return;
        };

        // Check all statements in the body
        check_statements(&body.statements, ctx);
        
        // Check yield expression if present
        if let Some(yield_expr) = &body.yield_expr {
            check_expression(yield_expr, ctx);
        }
    }
}

fn check_statements(statements: &[Statement], ctx: &mut AnalysisContext) {
    for stmt in statements {
        check_statement(stmt, ctx);
    }
}

fn check_statement(stmt: &Statement, ctx: &mut AnalysisContext) {
    match &stmt.kind {
        StatementKind::GuardLet { else_block, .. } => {
            // Check if the else block diverges
            if !block_diverges(&else_block.statements, else_block.yield_expr.as_deref()) {
                ctx.report(GuardLetElseMustDivergeError {
                    span: stmt.span.clone(),
                });
            }
            
            // Also check nested statements in the else block
            check_statements(&else_block.statements, ctx);
            if let Some(yield_expr) = &else_block.yield_expr {
                check_expression(yield_expr, ctx);
            }
        }
        StatementKind::Binding { value: Some(expr), .. } => {
            check_expression(expr, ctx);
        }
        StatementKind::Binding { value: None, .. } => {}
        StatementKind::Expr(expr) => {
            check_expression(expr, ctx);
        }
    }
}

fn check_expression(expr: &kestrel_semantic_tree::expr::Expression, ctx: &mut AnalysisContext) {
    match &expr.kind {
        ExprKind::If { then_branch, then_value, else_branch, .. } => {
            check_statements(then_branch, ctx);
            if let Some(val) = then_value {
                check_expression(val, ctx);
            }
            if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        check_statements(statements, ctx);
                        if let Some(v) = value {
                            check_expression(v, ctx);
                        }
                    }
                    ElseBranch::ElseIf(if_expr) => {
                        check_expression(if_expr, ctx);
                    }
                }
            }
        }
        ExprKind::While { body, .. } | ExprKind::WhileLet { body, .. } => {
            check_statements(body, ctx);
        }
        ExprKind::Loop { body, .. } => {
            check_statements(body, ctx);
        }
        ExprKind::Closure { body, tail_expr, .. } => {
            check_statements(body, ctx);
            if let Some(tail) = tail_expr {
                check_expression(tail, ctx);
            }
        }
        ExprKind::Match { arms, .. } => {
            for arm in arms {
                check_expression(&arm.body, ctx);
            }
        }
        _ => {}
    }
}

/// Check if a block diverges (ends with return, break, continue).
fn block_diverges(
    statements: &[Statement],
    yield_expr: Option<&kestrel_semantic_tree::expr::Expression>,
) -> bool {
    // Check statements for divergence
    for stmt in statements {
        if statement_diverges(stmt) {
            return true;
        }
    }
    
    // Check yield expression
    if let Some(expr) = yield_expr {
        return expression_diverges(expr);
    }
    
    // Check if last statement is an expression statement that diverges
    if let Some(last) = statements.last() {
        if let StatementKind::Expr(expr) = &last.kind {
            return expression_diverges(expr);
        }
    }
    
    false
}

fn statement_diverges(stmt: &Statement) -> bool {
    match &stmt.kind {
        StatementKind::Expr(expr) => expression_diverges(expr),
        StatementKind::Binding { value: Some(expr), .. } => expression_diverges(expr),
        StatementKind::Binding { value: None, .. } => false,
        StatementKind::GuardLet { conditions, .. } => {
            // Any condition might diverge
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        if expression_diverges(expr) {
                            return true;
                        }
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        if expression_diverges(value) {
                            return true;
                        }
                    }
                }
            }
            // The else block must diverge (but the guard-let as a whole doesn't diverge
            // because control continues after it if the pattern matches)
            false
        }
    }
}

fn expression_diverges(expr: &kestrel_semantic_tree::expr::Expression) -> bool {
    match &expr.kind {
        ExprKind::Return { .. } => true,
        ExprKind::Break { .. } => true,
        ExprKind::Continue { .. } => true,
        ExprKind::If { then_branch, then_value, else_branch, .. } => {
            // If both branches diverge, the if diverges
            let then_diverges = block_diverges(then_branch, then_value.as_deref());
            let else_diverges = match else_branch {
                Some(ElseBranch::Block { statements, value }) => {
                    block_diverges(statements, value.as_deref())
                }
                Some(ElseBranch::ElseIf(if_expr)) => expression_diverges(if_expr),
                None => false,
            };
            then_diverges && else_diverges
        }
        ExprKind::Match { arms, .. } => {
            // Match diverges if all arms diverge
            !arms.is_empty() && arms.iter().all(|arm| expression_diverges(&arm.body))
        }
        ExprKind::Loop { body, .. } => {
            // Infinite loop without break diverges
            // But for simplicity, assume it might break
            // TODO: properly analyze break statements
            false
        }
        _ => false,
    }
}
