//! # Dead Code Analyzer
//!
//! Detects unreachable statements after diverging expressions
//! (return, break, continue, infinite loops). Recurses into
//! nested blocks (if/else, loops, match arms) to find inner dead code.
//!
//! ## Diagnostics
//!
//! ### E002 — `unreachable_code` (Warning, Correctness)
//!
//! **Message:** "unreachable code"
//!
//! **Labels:**
//! - Primary: the first unreachable statement or expression after divergence
//!   - Span source: `util::stmt_span` on the unreachable `HirStmtId`, or
//!     `util::expr_span` on the unreachable tail `HirExprId`
//!   - Message: "this code will never execute"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E002",
    name: "unreachable_code",
    default_severity: Severity::Warning,
    category: Category::Correctness,
}];

pub struct DeadCodeAnalyzer;

impl Describe for DeadCodeAnalyzer {
    fn id(&self) -> &'static str {
        "dead_code"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for DeadCodeAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        check_block(cx.hir, &cx.hir.statements, cx.hir.tail_expr, &mut diags);
        diags
    }
}

/// Check a block for dead code: if a statement diverges, everything after is unreachable.
fn check_block(
    hir: &HirBody,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let mut diverged = false;

    for (i, &stmt_id) in stmts.iter().enumerate() {
        if diverged {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "unreachable code".into(),
                labels: vec![DiagLabel {
                    span: util::stmt_span(hir, stmt_id),
                    message: "this code will never execute".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            // Only report the first unreachable statement in a block
            break;
        }

        // Check if this statement diverges
        if stmt_diverges(hir, stmt_id) {
            if i + 1 < stmts.len() || tail.is_some() {
                diverged = true;
            }
        }

        // Recurse into sub-blocks within the statement
        check_stmt_inner(hir, stmt_id, diags);
    }

    // Check tail expression for inner dead code
    if let Some(tail) = tail {
        if diverged {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "unreachable code".into(),
                labels: vec![DiagLabel {
                    span: util::expr_span(hir, tail),
                    message: "this code will never execute".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        } else {
            check_expr_inner(hir, tail, diags);
        }
    }
}

fn check_stmt_inner(hir: &HirBody, id: HirStmtId, diags: &mut Vec<AnalyzeDiagnostic>) {
    if let HirStmt::Expr { expr, .. } = &hir.stmts[id] {
        check_expr_inner(hir, *expr, diags);
    }
}

/// Recurse into expressions that contain blocks to find inner dead code.
fn check_expr_inner(hir: &HirBody, id: HirExprId, diags: &mut Vec<AnalyzeDiagnostic>) {
    match &hir.exprs[id] {
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            check_block(hir, &then_body.stmts, then_body.tail_expr, diags);
            if let Some(else_block) = else_body {
                check_block(hir, &else_block.stmts, else_block.tail_expr, diags);
            }
        }
        HirExpr::Loop { body, .. } => {
            check_block(hir, &body.stmts, body.tail_expr, diags);
        }
        HirExpr::Match { arms, .. } => {
            for arm in arms {
                check_expr_inner(hir, arm.body, diags);
            }
        }
        HirExpr::Block { body, .. } => {
            check_block(hir, &body.stmts, body.tail_expr, diags);
        }
        // Recurse into closure bodies for inner dead code detection.
        // Closures don't cause outer divergence, but their bodies can have dead code.
        HirExpr::Closure { body, .. } => {
            check_block(hir, &body.stmts, body.tail_expr, diags);
        }
        _ => {}
    }
}

// ===== Divergence analysis (local to this analyzer) =====

fn stmt_diverges(hir: &HirBody, id: HirStmtId) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_diverges(hir, *expr),
        _ => false,
    }
}

fn expr_diverges(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Return { .. } | HirExpr::Break { .. } | HirExpr::Continue { .. } => true,

        HirExpr::Loop { body, .. } => {
            // If the loop body always returns, the loop returns
            if block_part_diverges(hir, body) {
                return true;
            }
            // If the loop has no break, it's an infinite loop — diverges
            // If it has a break, control can exit the loop normally
            !block_contains_break(hir, body)
        }

        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let then_div = block_part_diverges(hir, then_body);
            match else_body {
                Some(else_block) => then_div && block_part_diverges(hir, else_block),
                None => false,
            }
        }
        HirExpr::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_diverges(hir, arm.body))
        }
        HirExpr::Block { body, .. } => block_part_diverges(hir, body),
        _ => false,
    }
}

fn block_part_diverges(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if stmt_diverges(hir, stmt_id) {
            return true;
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_diverges(hir, tail);
    }
    false
}

// ===== Break detection for loop analysis =====

/// Check if a block contains a break targeting the enclosing loop.
/// Does NOT recurse into nested loops (their breaks target the inner loop).
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
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_contains_break(hir, then_body)
                || else_body
                    .as_ref()
                    .is_some_and(|e| block_contains_break(hir, e))
        }
        HirExpr::Match { arms, .. } => arms.iter().any(|arm| expr_contains_break(hir, arm.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),
        // Don't recurse into nested loops or closures
        HirExpr::Loop { .. } | HirExpr::Closure { .. } => false,
        _ => false,
    }
}
