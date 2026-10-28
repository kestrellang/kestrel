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

use crate::body::control_flow;
use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, BodyCheck, Describe};
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
    fn id(&self) -> AnalyzerId {
        AnalyzerId::DeadCode
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for DeadCodeAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        check_block(cx, &cx.hir.statements, cx.hir.tail_expr, &mut diags);
        diags
    }
}

/// Check a block for dead code: if a statement diverges, everything after is
/// unreachable.
///
/// Divergence itself comes from `control_flow` (G12) — this walk only decides
/// *where* to report. There is no `in_loop` flag any more: `break`/`continue`
/// are unconditionally `Never`-typed whether or not they stand in a loop, so
/// the shared predicate answers correctly without loop context. That also
/// retires the old "labeled break/continue are conservatively non-diverging"
/// carve-out, which suppressed a legitimate warning after `break outer;`.
fn check_block(
    cx: &BodyContext<'_>,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let mut diverged = false;
    let mut reported_in_block = false;

    for (i, &stmt_id) in stmts.iter().enumerate() {
        if diverged {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "unreachable code".into(),
                labels: vec![DiagLabel {
                    span: util::stmt_span(cx.hir, stmt_id),
                    message: "this code will never execute".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            reported_in_block = true;
            // Only report the first unreachable statement in a block
            break;
        }

        // Check if this statement diverges
        if control_flow::stmt_diverges(cx, stmt_id) && (i + 1 < stmts.len() || tail.is_some()) {
            diverged = true;
        }

        // Recurse into sub-blocks within the statement
        check_stmt_inner(cx, stmt_id, diags);
    }

    // Check tail expression for inner dead code
    if let Some(tail) = tail {
        if diverged {
            // Suppress the tail warning if we already reported an unreachable
            // statement in this block — the whole trailing chain is dead; one
            // warning per block is enough.
            if !reported_in_block {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: "unreachable code".into(),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, tail),
                        message: "this code will never execute".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        } else {
            check_expr_inner(cx, tail, diags);
        }
    }
}

fn check_stmt_inner(cx: &BodyContext<'_>, id: HirStmtId, diags: &mut Vec<AnalyzeDiagnostic>) {
    if let HirStmt::Expr { expr, .. } = &cx.hir.stmts[id] {
        check_expr_inner(cx, *expr, diags);
    }
}

/// Recurse into expressions that contain blocks to find inner dead code.
fn check_expr_inner(cx: &BodyContext<'_>, id: HirExprId, diags: &mut Vec<AnalyzeDiagnostic>) {
    match &cx.hir.exprs[id] {
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            check_block(cx, &then_body.stmts, then_body.tail_expr, diags);
            if let Some(else_block) = else_body {
                check_block(cx, &else_block.stmts, else_block.tail_expr, diags);
            }
        },
        HirExpr::Loop { body, .. } => {
            check_block(cx, &body.stmts, body.tail_expr, diags);
        },
        HirExpr::Match { arms, .. } => {
            for arm in arms {
                check_expr_inner(cx, arm.body, diags);
            }
        },
        HirExpr::Block { body, .. } => {
            check_block(cx, &body.stmts, body.tail_expr, diags);
        },
        HirExpr::Closure { body, .. } => {
            check_block(cx, &body.stmts, body.tail_expr, diags);
        },
        // `Sugar` is a transparent wrapper (see `HirExpr::Sugar` in
        // `kestrel-hir::body` — all consumers must recurse into `inner`).
        // Without this arm the whole desugared subtree was invisible here,
        // so E002 was structurally blind inside every `for` body: `for`
        // lowers to `Sugar{ForLoop} → Block → Loop → Match → user body`,
        // whereas `while` lowers to a bare `Loop` and worked. (G11)
        HirExpr::Sugar { inner, .. } => check_expr_inner(cx, *inner, diags),
        _ => {},
    }
}
