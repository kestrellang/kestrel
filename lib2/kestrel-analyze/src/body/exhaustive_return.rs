//! # Exhaustive Return Analyzer
//!
//! Checks that all code paths in non-unit functions return a value.
//! Skips unit-return functions, empty bodies (protocol decls), and
//! bodies with a tail expression.
//!
//! ## Diagnostics
//!
//! ### E001 — `missing_return` (Error, Correctness)
//!
//! **Message:** "function '{name}' does not return a value on all code paths"
//!
//! **Labels:**
//! - Primary: the last statement in the function body
//!   - Span source: `util::stmt_span` on the last `HirStmtId`
//!   - Message: "missing return"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{NodeKind, TypeAnnotation};
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E001",
    name: "missing_return",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ExhaustiveReturnAnalyzer;

impl Describe for ExhaustiveReturnAnalyzer {
    fn id(&self) -> &'static str {
        "exhaustive_return"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ExhaustiveReturnAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Only check functions (not inits, deinits, etc.)
        let kind = cx.query.get::<NodeKind>(cx.entity);
        if !matches!(kind, Some(NodeKind::Function)) {
            return vec![];
        }

        // Skip if no return type annotation (unit-return functions)
        if cx.query.get::<TypeAnnotation>(cx.entity).is_none() {
            return vec![];
        }

        // Skip empty bodies (protocol declarations, extern functions)
        if cx.hir.statements.is_empty() && cx.hir.tail_expr.is_none() {
            return vec![];
        }

        // A tail expression means the body always produces a value
        if cx.hir.tail_expr.is_some() {
            return vec![];
        }

        // Check if the statement list definitely diverges
        if block_diverges(cx.hir, &cx.hir.statements) {
            return vec![];
        }

        let func_name = util::entity_name(cx.query, cx.entity);

        let span = cx
            .hir
            .statements
            .last()
            .map(|&id| util::stmt_span(cx.hir, id))
            .unwrap_or_else(|| kestrel_span2::Span::synthetic(0));

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "function '{}' does not return a value on all code paths",
                func_name
            ),
            labels: vec![DiagLabel {
                span,
                message: "missing return".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}

// ===== Control flow divergence analysis =====
//
// Uses a 3-state ReturnState to distinguish:
// - Returns: all paths return a value (via `return` or throw)
// - Diverges: control flow exits abnormally (break/continue/infinite loop)
// - MayFallThrough: control may reach the end without returning
//
// For the exhaustive return check, both Returns and Diverges count as
// "definitely returns" — the function won't silently fall off the end.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnState {
    Returns,
    Diverges,
    MayFallThrough,
}

impl ReturnState {
    fn definitely_returns(self) -> bool {
        matches!(self, ReturnState::Returns | ReturnState::Diverges)
    }

    /// Merge two branches (if/else, match arms).
    /// Both branches must definitely return for the whole to definitely return.
    fn merge(self, other: ReturnState) -> ReturnState {
        match (self, other) {
            (ReturnState::Returns, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Returns, ReturnState::Diverges)
            | (ReturnState::Diverges, ReturnState::Returns) => ReturnState::Returns,
            (ReturnState::Diverges, ReturnState::Diverges) => ReturnState::Diverges,
            _ => ReturnState::MayFallThrough,
        }
    }
}

/// Whether a block of statements definitely returns.
fn block_state(hir: &HirBody, stmts: &[HirStmtId], tail: Option<HirExprId>) -> ReturnState {
    for &stmt_id in stmts {
        let state = stmt_state(hir, stmt_id);
        if state.definitely_returns() {
            return state;
        }
    }
    if let Some(tail) = tail {
        let state = expr_state(hir, tail);
        if state.definitely_returns() {
            return state;
        }
        // A tail expression means the block produces a value
        return ReturnState::Returns;
    }
    ReturnState::MayFallThrough
}

/// Whether a block of statements definitely diverges (for the top-level check).
fn block_diverges(hir: &HirBody, stmts: &[HirStmtId]) -> bool {
    block_state(hir, stmts, None).definitely_returns()
}

fn stmt_state(hir: &HirBody, id: HirStmtId) -> ReturnState {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_state(hir, *expr),
        HirStmt::Let { value: Some(v), .. } => expr_state(hir, *v),
        _ => ReturnState::MayFallThrough,
    }
}

fn expr_state(hir: &HirBody, id: HirExprId) -> ReturnState {
    match &hir.exprs[id] {
        HirExpr::Return { .. } => ReturnState::Returns,
        HirExpr::Break { .. } | HirExpr::Continue { .. } => ReturnState::Diverges,

        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let then_s = block_part_state(hir, then_body);
            match else_body {
                Some(else_block) => then_s.merge(block_part_state(hir, else_block)),
                None => ReturnState::MayFallThrough,
            }
        }

        HirExpr::Match { arms, .. } => {
            if arms.is_empty() {
                return ReturnState::MayFallThrough;
            }
            // All arms must definitely return for the match to definitely return
            let mut combined = expr_state(hir, arms[0].body);
            for arm in &arms[1..] {
                combined = combined.merge(expr_state(hir, arm.body));
            }
            combined
        }

        HirExpr::Loop { body, .. } => {
            // Check if the body always returns before any break could execute
            let body_state = block_part_state(hir, body);
            if body_state == ReturnState::Returns {
                return ReturnState::Returns;
            }
            // If the loop body has a break, control can exit the loop
            if block_contains_break(hir, body) {
                ReturnState::MayFallThrough
            } else {
                // Infinite loop with no break — diverges (never falls through)
                ReturnState::Diverges
            }
        }

        HirExpr::Block { body, .. } => block_part_state(hir, body),

        // Closures don't cause the enclosing function to return
        HirExpr::Closure { .. } => ReturnState::MayFallThrough,

        _ => ReturnState::MayFallThrough,
    }
}

fn block_part_state(hir: &HirBody, block: &HirBlock) -> ReturnState {
    block_state(hir, &block.stmts, block.tail_expr)
}

// ===== Break detection for loop analysis =====
//
// Checks whether a block contains a `break` that would exit the enclosing loop.
// Does NOT recurse into nested loops (their breaks target the inner loop).

fn block_contains_break(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if stmt_contains_break(hir, stmt_id) {
            return true;
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_contains_break(hir, tail);
    }
    false
}

fn stmt_contains_break(hir: &HirBody, id: HirStmtId) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_contains_break(hir, *expr),
        HirStmt::Let { value: Some(v), .. } => expr_contains_break(hir, *v),
        _ => false,
    }
}

fn expr_contains_break(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Break { .. } => true,

        // Recurse into if/else and match — breaks inside target the outer loop
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_contains_break(hir, then_body)
                || else_body.as_ref().is_some_and(|e| block_contains_break(hir, e))
        }
        HirExpr::Match { arms, .. } => arms.iter().any(|arm| expr_contains_break(hir, arm.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),

        // Do NOT recurse into nested loops — their breaks target the inner loop
        HirExpr::Loop { .. } => false,

        // Do NOT recurse into closures — their breaks are local
        HirExpr::Closure { .. } => false,

        _ => false,
    }
}
