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
        check_block(
            cx.hir,
            &cx.hir.statements,
            cx.hir.tail_expr,
            false,
            &mut diags,
        );
        diags
    }
}

/// Check a block for dead code: if a statement diverges, everything after is unreachable.
/// `in_loop` tracks whether we're inside a loop (break/continue only diverge inside loops).
fn check_block(
    hir: &HirBody,
    stmts: &[HirStmtId],
    tail: Option<HirExprId>,
    in_loop: bool,
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
                    span: util::stmt_span(hir, stmt_id),
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
        if stmt_diverges(hir, stmt_id, in_loop) {
            if i + 1 < stmts.len() || tail.is_some() {
                diverged = true;
            }
        }

        // Recurse into sub-blocks within the statement
        check_stmt_inner(hir, stmt_id, in_loop, diags);
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
                        span: util::expr_span(hir, tail),
                        message: "this code will never execute".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        } else {
            check_expr_inner(hir, tail, in_loop, diags);
        }
    }
}

fn check_stmt_inner(
    hir: &HirBody,
    id: HirStmtId,
    in_loop: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if let HirStmt::Expr { expr, .. } = &hir.stmts[id] {
        check_expr_inner(hir, *expr, in_loop, diags);
    }
}

/// Recurse into expressions that contain blocks to find inner dead code.
fn check_expr_inner(
    hir: &HirBody,
    id: HirExprId,
    in_loop: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &hir.exprs[id] {
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            check_block(hir, &then_body.stmts, then_body.tail_expr, in_loop, diags);
            if let Some(else_block) = else_body {
                check_block(hir, &else_block.stmts, else_block.tail_expr, in_loop, diags);
            }
        },
        HirExpr::Loop { body, .. } => {
            // Inside a loop body, break/continue are valid divergence points
            check_block(hir, &body.stmts, body.tail_expr, true, diags);
        },
        HirExpr::Match { arms, .. } => {
            for arm in arms {
                check_expr_inner(hir, arm.body, in_loop, diags);
            }
        },
        HirExpr::Block { body, .. } => {
            check_block(hir, &body.stmts, body.tail_expr, in_loop, diags);
        },
        HirExpr::Closure { body, .. } => {
            // Closures start a new context — break/continue aren't valid
            check_block(hir, &body.stmts, body.tail_expr, false, diags);
        },
        _ => {},
    }
}

// ===== Divergence analysis (local to this analyzer) =====

fn stmt_diverges(hir: &HirBody, id: HirStmtId, in_loop: bool) -> bool {
    match &hir.stmts[id] {
        HirStmt::Expr { expr, .. } => expr_diverges(hir, *expr, in_loop),
        _ => false,
    }
}

fn expr_diverges(hir: &HirBody, id: HirExprId, in_loop: bool) -> bool {
    match &hir.exprs[id] {
        HirExpr::Return { .. } => true,
        // break/continue only diverge when inside a loop — outside a loop
        // they're invalid (already reported as errors), not divergence points.
        // Unlabeled break/continue inside a loop always diverge.
        // Labeled break/continue are conservatively treated as non-diverging
        // for dead code purposes — the label might target a non-enclosing loop.
        HirExpr::Break { label, .. } | HirExpr::Continue { label, .. } => {
            in_loop && label.is_none()
        },

        HirExpr::Loop { body, .. } => {
            // A loop that can exit via `break` does NOT diverge — even if another
            // path inside the body returns. This matches lib1 and also handles
            // desugared `while cond { ... }` whose loop body contains an implicit
            // break (from the condition check).
            if block_contains_break(hir, body) {
                return false;
            }
            // No break: if the body always returns, the loop diverges by returning.
            if block_always_returns(hir, body) {
                return true;
            }
            // Otherwise it's an infinite loop — also diverges.
            true
        },

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
        },
        HirExpr::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_diverges(hir, arm.body, in_loop))
        },
        HirExpr::Block { body, .. } => block_part_diverges(hir, body),
        _ => false,
    }
}

fn block_part_diverges(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if stmt_diverges(hir, stmt_id, true) {
            return true;
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_diverges(hir, tail, true);
    }
    false
}

/// Check if any Loop in the HIR body has the given label.
/// Used to suppress divergence for break/continue with invalid labels.
fn body_has_loop_label(hir: &HirBody, label: &str) -> bool {
    for (_, expr) in hir.exprs.iter() {
        if let HirExpr::Loop { label: Some(l), .. } = expr {
            if l == label {
                return true;
            }
        }
    }
    false
}

/// Check if a block always returns (via `return`), ignoring break/continue.
/// Used to determine if a loop body always exits the function, making
/// code after the loop unreachable.
fn block_always_returns(hir: &HirBody, block: &HirBlock) -> bool {
    for &stmt_id in &block.stmts {
        if let HirStmt::Expr { expr, .. } = &hir.stmts[stmt_id] {
            if expr_always_returns(hir, *expr) {
                return true;
            }
        }
    }
    if let Some(tail) = block.tail_expr {
        return expr_always_returns(hir, tail);
    }
    false
}

/// Check if an expression always returns from the function.
/// Only `return` counts — break/continue exit the loop, not the function.
fn expr_always_returns(hir: &HirBody, id: HirExprId) -> bool {
    match &hir.exprs[id] {
        HirExpr::Return { .. } => true,
        HirExpr::If {
            then_body,
            else_body,
            ..
        } => {
            block_always_returns(hir, then_body)
                && else_body
                    .as_ref()
                    .is_some_and(|e| block_always_returns(hir, e))
        },
        HirExpr::Match { arms, .. } => {
            !arms.is_empty() && arms.iter().all(|arm| expr_always_returns(hir, arm.body))
        },
        HirExpr::Block { body, .. } => block_always_returns(hir, body),
        _ => false,
    }
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
        },
        HirExpr::Match { arms, .. } => arms.iter().any(|arm| expr_contains_break(hir, arm.body)),
        HirExpr::Block { body, .. } => block_contains_break(hir, body),
        // Don't recurse into nested loops or closures
        HirExpr::Loop { .. } | HirExpr::Closure { .. } => false,
        _ => false,
    }
}
