//! # Guard Divergence Analyzer
//!
//! Verifies that guard/guard-let else blocks always diverge (return, break,
//! continue). Both `guard <condition> else { ... }` and `guard let <pattern> = <expr> else { ... }`
//! are desugared to `if condition { } else { else_body }` in the HIR.
//! The `HirBody.guard_stmts` field marks which statements originated from
//! guard desugaring, so this analyzer can identify them reliably.
//!
//! ## Diagnostics
//!
//! ### E003 — `guard_let_else_must_diverge` (Error, Correctness)
//!
//! **Message:** "guard else block must diverge (return, break, continue, or throw)"
//!
//! **Labels:**
//! - Primary: the guard statement
//!   - Span source: `util::stmt_span` on the guard `HirStmtId`
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

pub struct GuardDivergenceAnalyzer;

impl Describe for GuardDivergenceAnalyzer {
    fn id(&self) -> &'static str {
        "guard_divergence"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for GuardDivergenceAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Check guard-originated If statements (non-let guards)
        for &stmt_id in &cx.hir.guard_stmts {
            let HirStmt::Expr { expr, .. } = &cx.hir.stmts[stmt_id] else {
                continue;
            };
            let HirExpr::If { else_body, .. } = &cx.hir.exprs[*expr] else {
                continue;
            };
            let Some(else_block) = else_body else {
                continue;
            };
            if !block_diverges(cx.hir, else_block) {
                let span = non_diverging_span(cx.hir, else_block)
                    .unwrap_or_else(|| util::stmt_span(cx.hir, stmt_id));
                diags.push(guard_diverge_diagnostic(span));
            }
        }

        // Check CPS guard-let Match expressions (MatchSource::GuardLet).
        // The wildcard arm (last arm) is the else body that must diverge.
        for (_, expr) in cx.hir.exprs.iter() {
            if let HirExpr::Match { arms, source: MatchSource::GuardLet, .. } = expr {
                if let Some(else_arm) = arms.last() {
                    if !expr_diverges(cx.hir, else_arm.body) {
                        let arm_span = util::expr_span(cx.hir, else_arm.body);
                        diags.push(guard_diverge_diagnostic(arm_span));
                    }
                }
            }
        }

        diags
    }
}

fn guard_diverge_diagnostic(span: kestrel_span::Span) -> AnalyzeDiagnostic {
    AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message: "guard else block must diverge (return, break, continue, or throw)".into(),
        labels: vec![DiagLabel {
            span,
            message: "else block does not diverge".into(),
            is_primary: true,
        }],
        notes: vec![],
    }
}

/// Span of the expression that *should* diverge but doesn't — i.e. the
/// block's tail expression, or the last statement-expression. Used to put
/// the E003 diagnostic under the offending value rather than the whole
/// guard-let statement.
fn non_diverging_span(hir: &HirBody, block: &HirBlock) -> Option<kestrel_span::Span> {
    if let Some(tail) = block.tail_expr {
        return Some(util::expr_span(hir, tail));
    }
    if let Some(&last_stmt) = block.stmts.last()
        && let HirStmt::Expr { expr, .. } = &hir.stmts[last_stmt]
    {
        return Some(util::expr_span(hir, *expr));
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
