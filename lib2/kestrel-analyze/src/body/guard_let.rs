//! # Guard-Let Divergence Analyzer
//!
//! Verifies that guard-let else blocks always diverge (return, break, continue).
//! Guard-let is desugared to `if condition { } else { else_body }` in the HIR.
//! The `HirBody.guard_let_stmts` field marks which statements originated from
//! guard-let desugaring, so this analyzer can identify them reliably.
//!
//! ## Diagnostics
//!
//! ### E003 — `guard_let_else_must_diverge` (Error, Correctness)
//!
//! **Message:** "guard-let else block must diverge (return, break, continue, or throw)"
//!
//! **Labels:**
//! - Primary: the guard-let statement
//!   - Span source: `util::stmt_span` on the guard-let `HirStmtId`
//!   - Message: "else block does not diverge"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E003",
    name: "guard_let_else_must_diverge",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct GuardLetDivergenceAnalyzer;

impl Describe for GuardLetDivergenceAnalyzer {
    fn id(&self) -> &'static str {
        "guard_let_divergence"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for GuardLetDivergenceAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Check each guard-let-originated statement
        for &stmt_id in &cx.hir.guard_let_stmts {
            // The desugared form is: HirStmt::Expr { HirExpr::If { then: empty, else: body } }
            let HirStmt::Expr { expr, .. } = &cx.hir.stmts[stmt_id] else {
                continue;
            };
            let HirExpr::If { else_body, .. } = &cx.hir.exprs[*expr] else {
                continue;
            };
            let Some(else_block) = else_body else {
                continue;
            };

            // Check if the else block diverges
            if !block_diverges(cx.hir, else_block) {
                // Point at the non-diverging value expression inside the else
                // block (the tail expression or last statement), not the whole
                // guard-let statement — this lines the diagnostic up with the
                // code the user needs to change.
                let span = non_diverging_span(cx.hir, else_block)
                    .unwrap_or_else(|| util::stmt_span(cx.hir, stmt_id));
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message:
                        "guard-let else block must diverge (return, break, continue, or throw)"
                            .into(),
                    labels: vec![DiagLabel {
                        span,
                        message: "else block does not diverge".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        diags
    }
}

/// Span of the expression that *should* diverge but doesn't — i.e. the
/// block's tail expression, or the last statement-expression. Used to put
/// the E003 diagnostic under the offending value rather than the whole
/// guard-let statement.
fn non_diverging_span(hir: &HirBody, block: &HirBlock) -> Option<kestrel_span2::Span> {
    if let Some(tail) = block.tail_expr {
        return Some(util::expr_span(hir, tail));
    }
    if let Some(&last_stmt) = block.stmts.last() {
        if let HirStmt::Expr { expr, .. } = &hir.stmts[last_stmt] {
            return Some(util::expr_span(hir, *expr));
        }
    }
    None
}

// ===== Divergence analysis (private to this analyzer) =====

/// Check if a block definitely diverges.
fn block_diverges(hir: &HirBody, block: &HirBlock) -> bool {
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

fn stmt_diverges(hir: &HirBody, id: HirStmtId) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_diverges(hir, *expr),
        _ => false,
    }
}

fn expr_diverges(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Return { .. } | HirExpr::Break { .. } | HirExpr::Continue { .. } => true,
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            let then_div = block_diverges(hir, then_body);
            match else_body {
                Some(else_block) => then_div && block_diverges(hir, else_block),
                None => false,
            }
        },
        HirExpr::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_diverges(hir, arm.body))
        },
        // Infinite loop (no break) diverges
        HirExpr::Loop { .. } => true,
        HirExpr::Block { body, .. } => block_diverges(hir, body),
        _ => false,
    }
}
