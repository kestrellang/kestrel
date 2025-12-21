use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::{AnalysisContext, reset_node_flags};
use crate::runner::AnalyzerId;

use kestrel_semantic_model::SemanticModel;
use kestrel_semantic_tree::behavior::executable::{ExecutableBehavior, ResolvedExecutableBehavior};
use kestrel_semantic_tree::expr::{ExprKind, Expression};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::stmt::{Statement, StatementKind};
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::Symbol;

pub(crate) fn walk_root(
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    let root = model.root().clone();
    walk_symbol(&root, analyzers, model, ctx);
}

fn walk_symbol(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Track current symbol on the stack for analyzers needing context
    ctx.push_symbol(symbol.clone());
    // Pre-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_symbol(symbol, ctx);
    }

    if ctx.stopped {
        return;
    }
    if !ctx.skip_children {
        // Visit body if function or initializer
        let kind = symbol.metadata().kind();
        if matches!(
            kind,
            KestrelSymbolKind::Function | KestrelSymbolKind::Initializer
        ) {
            if let Some(body) = get_executable_body(symbol) {
                for stmt in &body.statements {
                    walk_statement(stmt, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
                if let Some(yield_expr) = body.yield_expr() {
                    walk_expression(yield_expr, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
        }

        // Recurse into children
        for child in symbol.metadata().children() {
            reset_node_flags(ctx);
            walk_symbol(&child, analyzers, model, ctx);
            if ctx.stopped {
                return;
            }
        }
    }

    // Post-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_symbol_post(symbol, ctx);
    }
    // Pop current symbol after finishing this node
    ctx.pop_symbol();
}

fn walk_statement(
    stmt: &Statement,
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Pre-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_statement(stmt, ctx);
    }

    if ctx.stopped {
        return;
    }
    if !ctx.skip_children {
        match &stmt.kind {
            StatementKind::Binding { pattern, value } => {
                walk_pattern(pattern, analyzers, model, ctx);
                if let Some(value) = value {
                    walk_expression(value, analyzers, model, ctx);
                }
            }
            StatementKind::Expr(expr) => {
                walk_expression(expr, analyzers, model, ctx);
            }
        }
    }

    // Post-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_statement_post(stmt, ctx);
    }
}

fn walk_expression(
    expr: &Expression,
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Pre-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_expression(expr, ctx);
    }

    if ctx.stopped {
        return;
    }
    if !ctx.skip_children {
        match &expr.kind {
            ExprKind::Array(elements) => {
                for e in elements {
                    walk_expression(e, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::OverloadedRef(_) => { /* leaf */ }
            ExprKind::Loop { body, .. } => {
                for stmt in body {
                    walk_statement(stmt, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::Tuple(elements) => {
                for e in elements {
                    walk_expression(e, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::Grouping(inner) => {
                walk_expression(inner, analyzers, model, ctx);
            }
            ExprKind::FieldAccess { object, .. } => {
                walk_expression(object, analyzers, model, ctx);
            }
            ExprKind::TupleIndex { tuple, .. } => {
                walk_expression(tuple, analyzers, model, ctx);
            }
            ExprKind::MethodRef { receiver, .. } => {
                walk_expression(receiver, analyzers, model, ctx);
            }
            ExprKind::Call {
                callee, arguments, ..
            } => {
                walk_expression(callee, analyzers, model, ctx);
                for arg in arguments {
                    walk_expression(&arg.value, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::PrimitiveMethodCall {
                receiver,
                arguments,
                ..
            } => {
                walk_expression(receiver, analyzers, model, ctx);
                for arg in arguments {
                    walk_expression(&arg.value, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::ImplicitStructInit { arguments, .. } => {
                for arg in arguments {
                    walk_expression(&arg.value, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::Assignment { target, value } => {
                walk_expression(target, analyzers, model, ctx);
                walk_expression(value, analyzers, model, ctx);
            }
            ExprKind::If {
                condition,
                then_branch,
                then_value,
                else_branch,
            } => {
                walk_expression(condition, analyzers, model, ctx);
                for stmt in then_branch {
                    walk_statement(stmt, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
                if let Some(v) = then_value {
                    walk_expression(v, analyzers, model, ctx);
                }
                if let Some(else_br) = else_branch {
                    match else_br {
                        kestrel_semantic_tree::expr::ElseBranch::Block { statements, value } => {
                            for stmt in statements {
                                walk_statement(stmt, analyzers, model, ctx);
                                if ctx.stopped {
                                    return;
                                }
                            }
                            if let Some(v) = value {
                                walk_expression(v, analyzers, model, ctx);
                            }
                        }
                        kestrel_semantic_tree::expr::ElseBranch::ElseIf(e) => {
                            walk_expression(e, analyzers, model, ctx);
                        }
                    }
                }
            }
            ExprKind::While {
                condition, body, ..
            } => {
                walk_expression(condition, analyzers, model, ctx);
                for stmt in body {
                    walk_statement(stmt, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
            }
            ExprKind::Closure { body, tail_expr, .. } => {
                // Walk closure body statements
                for stmt in body {
                    walk_statement(stmt, analyzers, model, ctx);
                    if ctx.stopped {
                        return;
                    }
                }
                // Walk tail expression if present
                if let Some(tail) = tail_expr {
                    walk_expression(tail, analyzers, model, ctx);
                }
            }
            ExprKind::ImplicitMemberAccess { arguments, .. } => {
                if let Some(args) = arguments {
                    for arg in args {
                        walk_expression(&arg.value, analyzers, model, ctx);
                        if ctx.stopped {
                            return;
                        }
                    }
                }
            }
            // Leaf kinds or handled elsewhere
            ExprKind::Literal(_)
            | ExprKind::LocalRef(_)
            | ExprKind::SymbolRef(_)
            | ExprKind::TypeRef(_)
            | ExprKind::TypeParameterRef(_)
            | ExprKind::AssociatedTypeRef
            | ExprKind::EnumCase { .. }
            | ExprKind::Break { .. }
            | ExprKind::Continue { .. }
            | ExprKind::Return { value: None }
            | ExprKind::Error => {}
            ExprKind::Return { value: Some(v) } => {
                walk_expression(v, analyzers, model, ctx);
            }
        }
    }

    // Post-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_expression_post(expr, ctx);
    }
}

fn walk_pattern(
    pattern: &Pattern,
    analyzers: &mut [&mut dyn Analyzer],
    _model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Pre-visit
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_pattern(pattern, ctx);
    }
    // No recursive sub-patterns yet
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_pattern_post(pattern, ctx);
    }
}

#[allow(dead_code)]
fn walk_type(
    ty: &Ty,
    analyzers: &mut [&mut dyn Analyzer],
    _model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_type(ty, ctx);
    }
    for (i, a) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        a.visit_type_post(ty, ctx);
    }
}

fn get_executable_body(
    symbol: &Arc<dyn Symbol<KestrelLanguage>>,
) -> Option<kestrel_semantic_tree::behavior::executable::CodeBlock> {
    symbol
        .metadata()
        .get_behavior::<ExecutableBehavior>()
        .map(|exec| exec.body().clone())
}
