use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_model::ExecutableBodyFor;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression, IfCondition};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

mod diagnostics;
use diagnostics::{UnreachableCodeWarning, UnreachableReason};

pub struct DeadCodeAnalyzer;

impl DeadCodeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DeadCodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DeadCodeAnalyzer {
    fn name(&self) -> &'static str {
        "dead_code"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();
        if !matches!(
            kind,
            KestrelSymbolKind::Function | KestrelSymbolKind::Initializer
        ) {
            return;
        }

        let symbol_id = symbol.metadata().id();
        let Some(body) = ctx.model.query(ExecutableBodyFor { symbol_id }) else {
            return;
        };

        let mut warnings = Vec::new();
        analyze_block(&body.statements, body.yield_expr.as_deref(), &mut warnings);
        for w in warnings {
            ctx.report(w);
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Divergence {
    None,
    Returns,
    Breaks,
    Continues,
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
    fn diverges(self) -> bool {
        self != Divergence::None
    }
}

fn analyze_block(
    statements: &[Statement],
    yield_expr: Option<&Expression>,
    errors: &mut Vec<UnreachableCodeWarning>,
) -> Divergence {
    let mut divergence = Divergence::None;

    for stmt in statements {
        if divergence.diverges() {
            if let Some(reason) = divergence.to_reason() {
                errors.push(UnreachableCodeWarning {
                    span: stmt.span.clone(),
                    reason,
                });
            }
            break;
        }
        divergence = analyze_statement(stmt, errors);
    }

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

fn analyze_statement(stmt: &Statement, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &stmt.kind {
        StatementKind::Binding {
            value: Some(expr), ..
        } => analyze_expression(expr, errors),
        StatementKind::Binding { value: None, .. } => Divergence::None,
        StatementKind::Expr(expr) => analyze_expression(expr, errors),
        StatementKind::GuardLet {
            conditions,
            else_block,
        } => {
            // Analyze each condition
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        let div = analyze_expression(expr, errors);
                        if div != Divergence::None {
                            return div;
                        }
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        let div = analyze_expression(value, errors);
                        if div != Divergence::None {
                            return div;
                        }
                    }
                }
            }
            // The else block must diverge, but the guard-let itself doesn't diverge
            // (control continues after guard-let if the pattern matches)
            let _ = analyze_block(
                &else_block.statements,
                else_block.yield_expr.as_deref(),
                errors,
            );
            Divergence::None
        }
        StatementKind::Deinit { .. } => {
            // Deinit is a simple statement that doesn't diverge
            Divergence::None
        }
    }
}

fn analyze_expression(expr: &Expression, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &expr.kind {
        ExprKind::Return { value } => {
            if let Some(val) = value {
                let _ = analyze_expression(val, errors);
            }
            Divergence::Returns
        }
        ExprKind::Break { .. } => Divergence::Breaks,
        ExprKind::Continue { .. } => Divergence::Continues,
        ExprKind::If {
            conditions,
            then_branch,
            then_value,
            else_branch,
        } => {
            // Analyze all conditions
            for condition in conditions {
                let cond_div = match condition {
                    IfCondition::Expr(expr) => analyze_expression(expr, errors),
                    IfCondition::Let { value, .. } => analyze_expression(value, errors),
                };
                if cond_div.diverges() {
                    return cond_div;
                }
            }

            let then_div = analyze_block(then_branch, then_value.as_deref(), errors);
            let else_div = if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => {
                        analyze_block(statements, value.as_deref(), errors)
                    }
                    ElseBranch::ElseIf(if_expr) => analyze_expression(if_expr, errors),
                }
            } else {
                Divergence::None
            };

            if then_div == Divergence::Returns && else_div == Divergence::Returns {
                Divergence::Returns
            } else {
                Divergence::None
            }
        }
        ExprKind::While {
            condition, body, ..
        } => {
            let cond_div = analyze_expression(condition, errors);
            if cond_div.diverges() {
                return cond_div;
            }
            let mut body_div = Divergence::None;
            for stmt in body {
                if body_div.diverges() {
                    if let Some(reason) = body_div.to_reason() {
                        errors.push(UnreachableCodeWarning {
                            span: stmt.span.clone(),
                            reason,
                        });
                    }
                    break;
                }
                body_div = analyze_statement(stmt, errors);
            }
            Divergence::None
        }
        ExprKind::WhileLet {
            conditions, body, ..
        } => {
            for condition in conditions {
                match condition {
                    kestrel_semantic_tree::expr::IfCondition::Expr(expr) => {
                        let div = analyze_expression(expr, errors);
                        if div.diverges() {
                            return div;
                        }
                    }
                    kestrel_semantic_tree::expr::IfCondition::Let { value, .. } => {
                        let div = analyze_expression(value, errors);
                        if div.diverges() {
                            return div;
                        }
                    }
                }
            }
            let mut body_div = Divergence::None;
            for stmt in body {
                if body_div.diverges() {
                    if let Some(reason) = body_div.to_reason() {
                        errors.push(UnreachableCodeWarning {
                            span: stmt.span.clone(),
                            reason,
                        });
                    }
                    break;
                }
                body_div = analyze_statement(stmt, errors);
            }
            Divergence::None
        }
        ExprKind::Loop { body, .. } => {
            let mut body_div = Divergence::None;
            let mut has_break = false;
            for stmt in body {
                if body_div.diverges() {
                    if let Some(reason) = body_div.to_reason() {
                        errors.push(UnreachableCodeWarning {
                            span: stmt.span.clone(),
                            reason,
                        });
                    }
                    break;
                }
                body_div = analyze_statement(stmt, errors);
                if statement_contains_break(&stmt.kind) {
                    has_break = true;
                }
            }
            if body_div == Divergence::Returns {
                return Divergence::Returns;
            }
            if !has_break && body_div != Divergence::Returns {
                Divergence::InfiniteLoop
            } else {
                Divergence::None
            }
        }
        ExprKind::Call {
            callee, arguments, ..
        } => {
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
            for e in elements {
                let d = analyze_expression(e, errors);
                if d.diverges() {
                    return d;
                }
            }
            Divergence::None
        }
        ExprKind::Tuple(elements) => {
            for e in elements {
                let d = analyze_expression(e, errors);
                if d.diverges() {
                    return d;
                }
            }
            Divergence::None
        }
        ExprKind::FieldAccess { object, .. } => analyze_expression(object, errors),
        ExprKind::TupleIndex { tuple, .. } => analyze_expression(tuple, errors),
        ExprKind::MethodRef { receiver, .. } => analyze_expression(receiver, errors),
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            let d = analyze_expression(receiver, errors);
            if d.diverges() {
                return d;
            }
            for arg in arguments {
                let d = analyze_expression(&arg.value, errors);
                if d.diverges() {
                    return d;
                }
            }
            Divergence::None
        }
        ExprKind::DeferredMethodCall {
            receiver,
            arguments,
            ..
        } => {
            let d = analyze_expression(receiver, errors);
            if d.diverges() {
                return d;
            }
            for arg in arguments {
                let d = analyze_expression(&arg.value, errors);
                if d.diverges() {
                    return d;
                }
            }
            Divergence::None
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments {
                let d = analyze_expression(&arg.value, errors);
                if d.diverges() {
                    return d;
                }
            }
            Divergence::None
        }
        ExprKind::Closure {
            body, tail_expr, ..
        } => {
            // Analyze closure body for dead code
            for stmt in body {
                let d = analyze_statement(stmt, errors);
                if d.diverges() {
                    // If the closure diverges, subsequent statements are unreachable
                    // but the closure expression itself doesn't make outer code unreachable
                    break;
                }
            }
            if let Some(tail) = tail_expr {
                let _ = analyze_expression(tail, errors);
            }
            // Closures don't cause divergence in the enclosing scope
            Divergence::None
        }
        // Implicit member access - check arguments if present
        ExprKind::ImplicitMemberAccess { arguments, .. } => {
            if let Some(args) = arguments {
                for arg in args {
                    let d = analyze_expression(&arg.value, errors);
                    if d.diverges() {
                        return d;
                    }
                }
            }
            Divergence::None
        }
        // Leaf expressions
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::AssociatedTypeRef
        | ExprKind::EnumCase { .. }
        | ExprKind::Error => Divergence::None,

        // Match expressions - all arms must diverge for the match to diverge
        ExprKind::Match { scrutinee, arms } => {
            // Analyze scrutinee for any errors
            let _ = analyze_expression(scrutinee, errors);

            if arms.is_empty() {
                Divergence::None
            } else {
                // Check if all arms diverge
                let mut all_diverge = true;
                for arm in arms {
                    if let Some(guard) = &arm.guard {
                        let _ = analyze_expression(guard, errors);
                    }
                    let body_divergence = analyze_expression(&arm.body, errors);
                    if !body_divergence.diverges() {
                        all_diverge = false;
                    }
                }
                if all_diverge {
                    Divergence::Returns
                } else {
                    Divergence::None
                }
            }
        }
    }
}

fn statement_contains_break(stmt: &StatementKind) -> bool {
    match stmt {
        StatementKind::Expr(Expression {
            kind: ExprKind::Break { .. },
            ..
        }) => true,
        StatementKind::Expr(e) => expression_contains_break(e),
        _ => false,
    }
}

fn expression_contains_break(expr: &Expression) -> bool {
    match &expr.kind {
        ExprKind::Break { .. } => true,
        ExprKind::If {
            then_branch,
            else_branch,
            ..
        } => {
            then_branch
                .iter()
                .any(|s| statement_contains_break(&s.kind))
                || else_branch
                    .as_ref()
                    .map(|b| match b {
                        ElseBranch::Block { statements, .. } => {
                            statements.iter().any(|s| statement_contains_break(&s.kind))
                        }
                        ElseBranch::ElseIf(e) => expression_contains_break(e),
                    })
                    .unwrap_or(false)
        }
        ExprKind::While { body, .. }
        | ExprKind::WhileLet { body, .. }
        | ExprKind::Loop { body, .. } => body.iter().any(|s| statement_contains_break(&s.kind)),
        ExprKind::Grouping(inner) => expression_contains_break(inner),
        ExprKind::Array(elements) | ExprKind::Tuple(elements) => {
            elements.iter().any(expression_contains_break)
        }
        ExprKind::FieldAccess { object, .. } => expression_contains_break(object),
        ExprKind::TupleIndex { tuple, .. } => expression_contains_break(tuple),
        ExprKind::MethodRef { receiver, .. } => expression_contains_break(receiver),
        ExprKind::Call {
            callee, arguments, ..
        } => {
            expression_contains_break(callee)
                || arguments
                    .iter()
                    .any(|a| expression_contains_break(&a.value))
        }
        ExprKind::PrimitiveMethodCall {
            receiver,
            arguments,
            ..
        } => {
            expression_contains_break(receiver)
                || arguments
                    .iter()
                    .any(|a| expression_contains_break(&a.value))
        }
        ExprKind::ImplicitStructInit { arguments, .. } => arguments
            .iter()
            .any(|a| expression_contains_break(&a.value)),
        ExprKind::Assignment { target, value } => {
            expression_contains_break(target) || expression_contains_break(value)
        }
        _ => false,
    }
}
