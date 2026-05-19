//! Shared utilities for analyzers.
//!
//! Span extraction helpers for HIR nodes and entity info helpers.
//! When writing a new analyzer, use these instead of creating local versions.
//! If you need a new utility, add it here and update AGENTS.md.

use kestrel_ast_builder::{DeclSpan, Name, Valued};
use kestrel_hecs::{Entity, QueryContext};
use kestrel_hir::body::*;
use kestrel_span::Span;
use kestrel_syntax_tree::{SyntaxElement, SyntaxKind};

// ===== Span extraction =====

/// Extract the span from any HirExpr variant.
pub fn expr_span(hir: &HirBody, id: HirExprId) -> Span {
    match &hir.exprs[id] {
        HirExpr::Literal { span, .. }
        | HirExpr::Local(_, span)
        | HirExpr::Def(_, _, span)
        | HirExpr::OverloadSet { span, .. }
        | HirExpr::Field { span, .. }
        | HirExpr::TupleIndex { span, .. }
        | HirExpr::ImplicitMember { span, .. }
        | HirExpr::Call { span, .. }
        | HirExpr::MethodCall { span, .. }
        | HirExpr::ProtocolCall { span, .. }
        | HirExpr::If { span, .. }
        | HirExpr::Loop { span, .. }
        | HirExpr::Match { span, .. }
        | HirExpr::Break { span, .. }
        | HirExpr::Continue { span, .. }
        | HirExpr::Return { span, .. }
        | HirExpr::Assign { span, .. }
        | HirExpr::Tuple { span, .. }
        | HirExpr::Array { span, .. }
        | HirExpr::Dict { span, .. }
        | HirExpr::Closure { span, .. }
        | HirExpr::Block { span, .. }
        | HirExpr::Error { span }
        | HirExpr::Sugar { span, .. } => span.clone(),
    }
}

/// Extract the span from any HirStmt variant.
pub fn stmt_span(hir: &HirBody, id: HirStmtId) -> Span {
    match &hir.stmts[id] {
        HirStmt::Let { span, .. } | HirStmt::Expr { span, .. } | HirStmt::Deinit { span, .. } => {
            span.clone()
        },
    }
}

/// Extract the span from any HirPat variant.
pub fn pat_span(hir: &HirBody, id: HirPatId) -> Span {
    match &hir.pats[id] {
        HirPat::Wildcard { span, .. }
        | HirPat::Binding { span, .. }
        | HirPat::Tuple { span, .. }
        | HirPat::Literal { span, .. }
        | HirPat::Range { span, .. }
        | HirPat::Variant { span, .. }
        | HirPat::ImplicitVariant { span, .. }
        | HirPat::Struct { span, .. }
        | HirPat::Array { span, .. }
        | HirPat::Or { span, .. }
        | HirPat::At { span, .. }
        | HirPat::Error { span, .. } => span.clone(),
    }
}

// ===== Entity info =====

/// Get a human-readable name for an entity from its Name component.
/// Falls back to "<anonymous>" if no Name is set.
pub fn entity_name(ctx: &QueryContext<'_>, entity: Entity) -> String {
    ctx.get::<Name>(entity)
        .map(|n| n.0.clone())
        .unwrap_or_else(|| "<anonymous>".into())
}

/// Get the declaration span for an entity from its DeclSpan component.
/// Falls back to a synthetic span if no DeclSpan is set.
pub fn entity_span(ctx: &QueryContext<'_>, entity: Entity) -> Span {
    ctx.get::<DeclSpan>(entity)
        .map(|s| s.0.clone())
        .unwrap_or_else(|| Span::synthetic(0))
}

/// Span of the closing `}` of an entity's block body (from the `Valued`
/// component's CodeBlock CST node). Returns `None` for expression bodies
/// or entities without a `Valued` component.
pub fn body_close_brace_span(ctx: &QueryContext<'_>, entity: Entity) -> Option<Span> {
    let valued = ctx.get::<Valued>(entity)?;
    let node = &valued.0;
    if node.kind() != SyntaxKind::CodeBlock {
        return None;
    }
    let file_id = ctx
        .get::<DeclSpan>(entity)
        .map(|s| s.0.file_id)
        .unwrap_or(0);
    for elem in node.children_with_tokens() {
        if let SyntaxElement::Token(tok) = elem
            && tok.kind() == SyntaxKind::RBrace
        {
            let r = tok.text_range();
            return Some(Span::new(
                file_id,
                usize::from(r.start())..usize::from(r.end()),
            ));
        }
    }
    None
}
