//! # Closure Analyzer
//!
//! Validates closure semantics: implicit `it` parameter arity, closure
//! parameter count and types vs expected function type, and assignment
//! restrictions on captured variables and closure parameters.
//!
//! ## Diagnostics
//!
//! ### E600 — `it_wrong_arity` (Error, Correctness)
//!
//! **Message:** "implicit 'it' parameter used in closure expecting {n} parameters"
//!
//! **Labels:**
//! - Primary: the closure expression
//!   - Span source: `util::expr_span` on the closure `HirExprId`
//!   - Message: "'it' requires exactly 1 parameter"
//!
//! **Notes:** (none)
//!
//! ### E601 — `closure_arity_mismatch` (Error, Correctness)
//!
//! **Message:** "closure has {actual} parameters, but expected {expected}"
//!
//! **Labels:**
//! - Primary: the closure expression
//!   - Span source: `util::expr_span` on the closure `HirExprId`
//!   - Message: "wrong number of parameters"
//!
//! **Notes:** (none)
//!
//! ### E602 — `closure_param_type_mismatch` (Error, Correctness)
//!
//! **Message:** "closure parameter type mismatch at position {index}"
//!
//! **Labels:**
//! - Primary: the closure parameter with wrong type
//!   - Span source: (closure span as fallback)
//!   - Message: "expected '{expected}', got '{actual}'"
//!
//! **Notes:** (none)
//!
//! ### E603 — `assign_to_captured_variable` (Error, Correctness)
//!
//! **Message:** "cannot assign to captured variable '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the assignment target `HirExprId`
//!   - Message: "captured variables are immutable in closures"
//!
//! **Notes:** (none)
//!
//! ### E604 — `assign_to_closure_parameter` (Error, Correctness)
//!
//! **Message:** "cannot assign to closure parameter '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the assignment target `HirExprId`
//!   - Message: "closure parameters are immutable"
//!
//! **Notes:** (none)

use std::collections::HashSet;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_hir::body::*;
use kestrel_hir::res::LocalId;
use kestrel_type_infer::result::ResolvedTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E600",
        name: "it_wrong_arity",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E601",
        name: "closure_arity_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E602",
        name: "closure_param_type_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E603",
        name: "assign_to_captured_variable",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E604",
        name: "assign_to_closure_parameter",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E605",
        name: "capturing_closure_escape",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E606",
        name: "cannot_infer_closure_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ClosureAnalyzer;

impl Describe for ClosureAnalyzer {
    fn id(&self) -> &'static str {
        "closure"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ClosureAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Captures come from the single source of truth — the post-inference
        // ClosureCaptures query (place-based). E603/E605 only need the set of
        // captured *root* locals.
        let capture_plan = cx.query.query(kestrel_type_infer::ClosureCaptures {
            entity: cx.entity,
            root: cx.root,
        });

        // Walk all expressions looking for closures
        for (expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Closure { params, body, .. } = expr else {
                continue;
            };

            let mut capture_roots: Vec<LocalId> =
                capture_plan.get(expr_id).iter().map(|c| c.key.root).collect();
            capture_roots.sort_by_key(|l| l.raw());
            capture_roots.dedup();
            let captures = &capture_roots;

            // Check closure arity and types against expected function type.
            if let Some(ty) = cx.typed.expr_types.get(&expr_id) {
                check_closure_type(cx, expr_id, params, ty, &mut diags);
            }

            // E606: closure param types could not be inferred (no context at all).
            // Only emit when inference produced NO other errors — unresolved params
            // with other errors are likely cascading failures, not missing context.
            if !params.is_empty() && cx.typed.errors.is_empty() {
                let has_unresolved = params.iter().any(|p| {
                    p.ty.is_none()
                        && cx
                            .typed
                            .local_types
                            .get(&p.local)
                            .is_some_and(|t| matches!(t, ResolvedTy::Error))
                });
                if has_unresolved {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[6].id,
                        severity: DESCRIPTORS[6].default_severity,
                        message: "could not infer type for closure parameter".into(),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, expr_id),
                            message: "closure needs type context".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                    continue;
                }
            }

            // E603: check for assignments to captured variables
            let diag_count_before = diags.len();
            if !captures.is_empty() {
                let capture_set: HashSet<LocalId> = captures.iter().copied().collect();
                check_capture_assignments(cx, body, &capture_set, &mut diags);
            }
            let has_capture_mutation = diags.len() > diag_count_before;

            // E605: capturing closure cannot escape its defining scope.
            // Skip if we already reported capture mutation (E603) — avoid double errors.
            if !captures.is_empty() && !has_capture_mutation {
                // Check if this closure is in return position of the function
                let is_func_tail = cx.hir.tail_expr == Some(expr_id);
                let is_returned = cx.hir.exprs.iter().any(
                    |(_, e)| matches!(e, HirExpr::Return { value: Some(v), .. } if *v == expr_id),
                );
                // Check if this closure is in return position of another closure
                let is_closure_tail = cx.hir.exprs.iter().any(|(_, e)| {
                    matches!(e, HirExpr::Closure { body: b, .. } if b.tail_expr == Some(expr_id))
                });
                if is_func_tail || is_returned || is_closure_tail {
                    let captured_names: Vec<String> = captures
                        .iter()
                        .map(|id| cx.hir.locals[*id].name.clone())
                        .collect();
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[5].id,
                        severity: DESCRIPTORS[5].default_severity,
                        message: "cannot return a closure that captures variables".into(),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, expr_id),
                            message: format!("captures: {}", captured_names.join(", ")),
                            is_primary: true,
                        }],
                        notes: vec![
                            "closures that capture variables cannot escape their defining function"
                                .into(),
                        ],
                    });
                }
            }
        }

        diags
    }
}

/// Check closure parameter count and types against the expected function type.
fn check_closure_type(
    cx: &BodyContext<'_>,
    expr_id: HirExprId,
    params: &[HirClosureParam],
    expected_ty: &ResolvedTy,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let ResolvedTy::Function {
        params: expected_params,
        ..
    } = expected_ty
    else {
        return;
    };

    let actual_count = params.len();
    let expected_count = expected_params.len();

    // Check if this is an implicit `it` closure (zero explicit params but
    // the body references a local named "it"). This is the E600 check.
    if actual_count == 0 && expected_count != 1 {
        // Check if any local in the body is named "it"
        let uses_it = cx.hir.locals.iter().any(|(_, local)| local.name == "it");
        if uses_it {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "implicit 'it' parameter used in closure expecting {} parameters",
                    expected_count
                ),
                labels: vec![DiagLabel {
                    span: util::expr_span(cx.hir, expr_id),
                    message: "'it' requires exactly 1 parameter".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return;
        }
    }

    // Arity mismatch (E601)
    if actual_count != expected_count && actual_count > 0 {
        diags.push(AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[1].id,
            severity: DESCRIPTORS[1].default_severity,
            message: format!(
                "closure has {} parameters, but expected {}",
                actual_count, expected_count
            ),
            labels: vec![DiagLabel {
                span: util::expr_span(cx.hir, expr_id),
                message: "wrong number of parameters".into(),
                is_primary: true,
            }],
            notes: vec![],
        });
        // Don't check types if counts differ
    }

    // TODO: E602 — per-parameter type mismatch checking.
    // This requires comparing resolved param types against expected_params,
    // which needs the resolved type for each closure param's type annotation.
    // Type mismatch is already caught by the constraint solver, so this is
    // a nice-to-have for better error messages.
}

/// Walk a closure body looking for assignments to closure parameters.
#[allow(dead_code)]
fn check_param_assignments(
    cx: &BodyContext<'_>,
    body: &HirBlock,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &body.stmts {
        check_stmt_for_param_assign(cx, stmt_id, param_locals, diags);
    }
    if let Some(tail) = body.tail_expr {
        check_expr_for_param_assign(cx, tail, param_locals, diags);
    }
}

#[allow(dead_code)]
fn check_stmt_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirStmtId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.stmts[id] {
        HirStmt::Expr { expr, .. } => {
            check_expr_for_param_assign(cx, *expr, param_locals, diags);
        },
        HirStmt::Let { value: Some(v), .. } => {
            check_expr_for_param_assign(cx, *v, param_locals, diags);
        },
        _ => {},
    }
}

#[allow(dead_code)]
fn check_expr_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirExprId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[id] {
        HirExpr::Assign { target, value, .. } => {
            // Check if target is a closure parameter
            if let HirExpr::Local(local_id, _) = &cx.hir.exprs[*target]
                && param_locals.contains(local_id)
            {
                let name = cx.hir.locals[*local_id].name.clone();
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[4].id,
                    severity: DESCRIPTORS[4].default_severity,
                    message: format!("cannot assign to closure parameter '{}'", name),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, *target),
                        message: "closure parameters are immutable".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
            check_expr_for_param_assign(cx, *value, param_locals, diags);
        },

        // Recurse into sub-expressions
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            check_expr_for_param_assign(cx, *condition, param_locals, diags);
            check_block_for_param_assign(cx, then_body, param_locals, diags);
            if let Some(else_block) = else_body {
                check_block_for_param_assign(cx, else_block, param_locals, diags);
            }
        },
        HirExpr::Loop { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        },
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            check_expr_for_param_assign(cx, *scrutinee, param_locals, diags);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    check_expr_for_param_assign(cx, guard, param_locals, diags);
                }
                check_expr_for_param_assign(cx, arm.body, param_locals, diags);
            }
        },
        HirExpr::Block { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        },
        HirExpr::Call { callee, args, .. } => {
            check_expr_for_param_assign(cx, *callee, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            check_expr_for_param_assign(cx, *receiver, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        },
        HirExpr::Return {
            value: Some(val), ..
        } => {
            check_expr_for_param_assign(cx, *val, param_locals, diags);
        },
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            check_expr_for_param_assign(cx, *base, param_locals, diags);
        },
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &elem in elements {
                check_expr_for_param_assign(cx, elem, param_locals, diags);
            }
        },
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                check_expr_for_param_assign(cx, entry.key, param_locals, diags);
                check_expr_for_param_assign(cx, entry.value, param_locals, diags);
            }
        },
        // Don't recurse into nested closures — they have their own param scope
        HirExpr::Closure { .. } => {},

        // Leaf expressions
        _ => {},
    }
}

#[allow(dead_code)]
fn check_block_for_param_assign(
    cx: &BodyContext<'_>,
    block: &HirBlock,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &block.stmts {
        check_stmt_for_param_assign(cx, stmt_id, param_locals, diags);
    }
    if let Some(tail) = block.tail_expr {
        check_expr_for_param_assign(cx, tail, param_locals, diags);
    }
}

/// Walk a closure body looking for assignments to captured variables (E603).
fn check_capture_assignments(
    cx: &BodyContext<'_>,
    block: &HirBlock,
    capture_set: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    for &stmt_id in &block.stmts {
        match &cx.hir.stmts[stmt_id] {
            HirStmt::Expr { expr, .. } => {
                walk_for_capture_assign(cx, *expr, capture_set, diags);
            },
            HirStmt::Let { value: Some(v), .. } => {
                walk_for_capture_assign(cx, *v, capture_set, diags);
            },
            _ => {},
        }
    }
    if let Some(tail) = block.tail_expr {
        walk_for_capture_assign(cx, tail, capture_set, diags);
    }
}

fn walk_for_capture_assign(
    cx: &BodyContext<'_>,
    id: HirExprId,
    capture_set: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[id] {
        HirExpr::Assign { target, value, .. } => {
            if let HirExpr::Local(local_id, _) = &cx.hir.exprs[*target]
                && capture_set.contains(local_id)
            {
                let name = cx.hir.locals[*local_id].name.clone();
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[3].id,
                    severity: DESCRIPTORS[3].default_severity,
                    message: format!("cannot assign to captured variable '{}'", name),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, *target),
                        message: "captured variables are immutable in closures".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
            walk_for_capture_assign(cx, *value, capture_set, diags);
        },
        HirExpr::If {
            condition,
            then_body,
            else_body,
            ..
        } => {
            walk_for_capture_assign(cx, *condition, capture_set, diags);
            check_capture_assignments(cx, then_body, capture_set, diags);
            if let Some(eb) = else_body {
                check_capture_assignments(cx, eb, capture_set, diags);
            }
        },
        HirExpr::Loop { body, .. } | HirExpr::Block { body, .. } => {
            check_capture_assignments(cx, body, capture_set, diags);
        },
        HirExpr::Match {
            scrutinee, arms, ..
        } => {
            walk_for_capture_assign(cx, *scrutinee, capture_set, diags);
            for arm in arms {
                if let Some(g) = arm.guard {
                    walk_for_capture_assign(cx, g, capture_set, diags);
                }
                walk_for_capture_assign(cx, arm.body, capture_set, diags);
            }
        },
        HirExpr::Call { callee, args, .. } => {
            walk_for_capture_assign(cx, *callee, capture_set, diags);
            for arg in args {
                walk_for_capture_assign(cx, arg.value, capture_set, diags);
            }
        },
        HirExpr::MethodCall { receiver, args, .. }
        | HirExpr::ProtocolCall { receiver, args, .. } => {
            walk_for_capture_assign(cx, *receiver, capture_set, diags);
            for arg in args {
                walk_for_capture_assign(cx, arg.value, capture_set, diags);
            }
        },
        HirExpr::Return { value: Some(v), .. } => {
            walk_for_capture_assign(cx, *v, capture_set, diags);
        },
        HirExpr::Closure { .. } => {}, // nested closure has own scope
        _ => {},
    }
}
