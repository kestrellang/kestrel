use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::expr::{ElseBranch, ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

mod diagnostics;
use diagnostics::{UnreachableCodeWarning, UnreachableReason};

pub struct DeadCodeAnalyzer;

impl DeadCodeAnalyzer {
    pub fn new() -> Self { Self }
}

impl Default for DeadCodeAnalyzer { fn default() -> Self { Self::new() } }

impl Analyzer for DeadCodeAnalyzer {
    fn name(&self) -> &'static str { "dead_code" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
        let kind = symbol.metadata().kind();
        if !matches!(kind, KestrelSymbolKind::Function | KestrelSymbolKind::Initializer) {
            return;
        }

        if let Some(body) = get_executable_body(symbol) {
            let mut warnings = Vec::new();
            analyze_block(&body.statements, body.yield_expr.as_deref(), &mut warnings);
            for w in warnings { ctx.report(w); }
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
    fn diverges(self) -> bool { self != Divergence::None }
}

fn analyze_block(statements: &[Statement], yield_expr: Option<&Expression>, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    let mut divergence = Divergence::None;

    for stmt in statements {
        if divergence.diverges() {
            if let Some(reason) = divergence.to_reason() {
                errors.push(UnreachableCodeWarning { span: stmt.span.clone(), reason });
            }
            break;
        }
        divergence = analyze_statement(stmt, errors);
    }

    if let Some(yield_expr) = yield_expr {
        if divergence.diverges() {
            if let Some(reason) = divergence.to_reason() {
                errors.push(UnreachableCodeWarning { span: yield_expr.span.clone(), reason });
            }
        } else {
            divergence = analyze_expression(yield_expr, errors);
        }
    }

    divergence
}

fn analyze_statement(stmt: &Statement, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &stmt.kind {
        StatementKind::Binding { value: Some(expr), .. } => analyze_expression(expr, errors),
        StatementKind::Binding { value: None, .. } => Divergence::None,
        StatementKind::Expr(expr) => analyze_expression(expr, errors),
    }
}

fn analyze_expression(expr: &Expression, errors: &mut Vec<UnreachableCodeWarning>) -> Divergence {
    match &expr.kind {
        ExprKind::Return { value } => {
            if let Some(val) = value { let _ = analyze_expression(val, errors); }
            Divergence::Returns
        }
        ExprKind::Break { .. } => Divergence::Breaks,
        ExprKind::Continue { .. } => Divergence::Continues,
        ExprKind::If { condition, then_branch, then_value, else_branch } => {
            let cond_div = analyze_expression(condition, errors);
            if cond_div.diverges() { return cond_div; }

            let then_div = analyze_block(then_branch, then_value.as_deref(), errors);
            let else_div = if let Some(else_b) = else_branch {
                match else_b {
                    ElseBranch::Block { statements, value } => analyze_block(statements, value.as_deref(), errors),
                    ElseBranch::ElseIf(if_expr) => analyze_expression(if_expr, errors),
                }
            } else { Divergence::None };

            if then_div == Divergence::Returns && else_div == Divergence::Returns { Divergence::Returns } else { Divergence::None }
        }
        ExprKind::While { condition, body, .. } => {
            let cond_div = analyze_expression(condition, errors);
            if cond_div.diverges() { return cond_div; }
            let mut body_div = Divergence::None;
            for stmt in body {
                if body_div.diverges() {
                    if let Some(reason) = body_div.to_reason() {
                        errors.push(UnreachableCodeWarning { span: stmt.span.clone(), reason });
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
                        errors.push(UnreachableCodeWarning { span: stmt.span.clone(), reason });
                    }
                    break;
                }
                body_div = analyze_statement(stmt, errors);
                if statement_contains_break(&stmt.kind) { has_break = true; }
            }
            if body_div == Divergence::Returns { return Divergence::Returns; }
            if !has_break && body_div != Divergence::Returns { Divergence::InfiniteLoop } else { Divergence::None }
        }
        ExprKind::Call { callee, arguments, .. } => {
            let div = analyze_expression(callee, errors);
            if div.diverges() { return div; }
            for arg in arguments {
                let div = analyze_expression(&arg.value, errors);
                if div.diverges() { return div; }
            }
            Divergence::None
        }
        ExprKind::Assignment { target, value } => {
            let div = analyze_expression(value, errors);
            if div.diverges() { return div; }
            analyze_expression(target, errors)
        }
        ExprKind::Grouping(inner) => analyze_expression(inner, errors),
        ExprKind::Array(elements) => {
            for e in elements { let d = analyze_expression(e, errors); if d.diverges() { return d; } }
            Divergence::None
        }
        ExprKind::Tuple(elements) => {
            for e in elements { let d = analyze_expression(e, errors); if d.diverges() { return d; } }
            Divergence::None
        }
        ExprKind::FieldAccess { object, .. } => analyze_expression(object, errors),
        ExprKind::TupleIndex { tuple, .. } => analyze_expression(tuple, errors),
        ExprKind::MethodRef { receiver, .. } => analyze_expression(receiver, errors),
        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            let d = analyze_expression(receiver, errors); if d.diverges() { return d; }
            for arg in arguments { let d = analyze_expression(&arg.value, errors); if d.diverges() { return d; } }
            Divergence::None
        }
        ExprKind::ImplicitStructInit { arguments, .. } => {
            for arg in arguments { let d = analyze_expression(&arg.value, errors); if d.diverges() { return d; } }
            Divergence::None
        }
        // Leaf expressions
        ExprKind::Literal(_)
        | ExprKind::LocalRef(_)
        | ExprKind::SymbolRef(_)
        | ExprKind::OverloadedRef(_)
        | ExprKind::TypeRef(_)
        | ExprKind::TypeParameterRef(_)
        | ExprKind::Error => Divergence::None,
    }
}

fn statement_contains_break(stmt: &StatementKind) -> bool {
    match stmt {
        StatementKind::Expr(Expression { kind: ExprKind::Break { .. }, .. }) => true,
        StatementKind::Expr(Expression { kind: ExprKind::If { then_branch, else_branch, .. }, .. }) => {
            then_branch.iter().any(|s| statement_contains_break(&s.kind))
                || else_branch.as_ref().map(|b| match b {
                    ElseBranch::Block { statements, .. } => statements.iter().any(|s| statement_contains_break(&s.kind)),
                    ElseBranch::ElseIf(e) => expression_contains_break(e),
                }).unwrap_or(false)
        }
        StatementKind::Expr(Expression { kind: ExprKind::While { body, .. }, .. })
        | StatementKind::Expr(Expression { kind: ExprKind::Loop { body, .. }, .. }) => {
            body.iter().any(|s| statement_contains_break(&s.kind))
        }
        _ => false,
    }
}

fn expression_contains_break(expr: &Expression) -> bool {
    match &expr.kind {
        ExprKind::Break { .. } => true,
        ExprKind::If { then_branch, else_branch, .. } => {
            then_branch.iter().any(|s| statement_contains_break(&s.kind))
                || else_branch.as_ref().map(|b| match b {
                    ElseBranch::Block { statements, .. } => statements.iter().any(|s| statement_contains_break(&s.kind)),
                    ElseBranch::ElseIf(e) => expression_contains_break(e),
                }).unwrap_or(false)
        }
        ExprKind::While { body, .. } | ExprKind::Loop { body, .. } => body.iter().any(|s| statement_contains_break(&s.kind)),
        ExprKind::Grouping(inner) => expression_contains_break(inner),
        ExprKind::Array(elements) | ExprKind::Tuple(elements) => elements.iter().any(expression_contains_break),
        ExprKind::FieldAccess { object, .. } => expression_contains_break(object),
        ExprKind::TupleIndex { tuple, .. } => expression_contains_break(tuple),
        ExprKind::MethodRef { receiver, .. } => expression_contains_break(receiver),
        ExprKind::Call { callee, arguments, .. } => {
            expression_contains_break(callee) || arguments.iter().any(|a| expression_contains_break(&a.value))
        }
        ExprKind::PrimitiveMethodCall { receiver, arguments, .. } => {
            expression_contains_break(receiver) || arguments.iter().any(|a| expression_contains_break(&a.value))
        }
        ExprKind::ImplicitStructInit { arguments, .. } => arguments.iter().any(|a| expression_contains_break(&a.value)),
        ExprKind::Assignment { target, value } => expression_contains_break(target) || expression_contains_break(value),
        _ => false,
    }
}

fn get_executable_body(symbol: &Arc<dyn Symbol<KestrelLanguage>>) -> Option<kestrel_semantic_tree::behavior::executable::CodeBlock> {
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

