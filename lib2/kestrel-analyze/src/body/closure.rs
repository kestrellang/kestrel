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

        // Walk all expressions looking for closures
        for (expr_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Closure { params, body, .. } = expr else {
                continue;
            };

            // Collect the closure's parameter locals
            let param_locals: HashSet<LocalId> = params.iter().map(|p| p.local).collect();

            // Check closure arity and types against expected function type.
            // The expected type comes from inference (the closure expr's type).
            if let Some(ty) = cx.typed.expr_types.get(&expr_id) {
                check_closure_type(cx, expr_id, params, ty, &mut diags);
            }

            // Check for assignments to closure parameters within the closure body
            check_param_assignments(cx, body, &param_locals, &mut diags);

            // TODO: Check for assignments to captured variables (E603).
            // This requires knowing which locals are captures vs parameters.
            // In lib2 HIR, closures don't have an explicit capture list — we'd
            // need to compare body locals against the enclosing function's locals.
            // Deferred to when capture tracking is added to the HIR.
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
        return;
    }

    // TODO: E602 — per-parameter type mismatch checking.
    // This requires comparing resolved param types against expected_params,
    // which needs the resolved type for each closure param's type annotation.
    // Type mismatch is already caught by the constraint solver, so this is
    // a nice-to-have for better error messages.
}

/// Walk a closure body looking for assignments to closure parameters.
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

fn check_stmt_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirStmtId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.stmts[id] {
        HirStmt::Expr { expr, .. } => {
            check_expr_for_param_assign(cx, *expr, param_locals, diags);
        }
        HirStmt::Let { value: Some(v), .. } => {
            check_expr_for_param_assign(cx, *v, param_locals, diags);
        }
        _ => {}
    }
}

fn check_expr_for_param_assign(
    cx: &BodyContext<'_>,
    id: HirExprId,
    param_locals: &HashSet<LocalId>,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[id] {
        HirExpr::Assign { target, value, .. } => {
            // Check if target is a closure parameter
            if let HirExpr::Local(local_id, _) = &cx.hir.exprs[*target] {
                if param_locals.contains(local_id) {
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
            }
            check_expr_for_param_assign(cx, *value, param_locals, diags);
        }

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
        }
        HirExpr::Loop { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        }
        HirExpr::Match { scrutinee, arms, .. } => {
            check_expr_for_param_assign(cx, *scrutinee, param_locals, diags);
            for arm in arms {
                if let Some(guard) = arm.guard {
                    check_expr_for_param_assign(cx, guard, param_locals, diags);
                }
                check_expr_for_param_assign(cx, arm.body, param_locals, diags);
            }
        }
        HirExpr::Block { body, .. } => {
            check_block_for_param_assign(cx, body, param_locals, diags);
        }
        HirExpr::Call { callee, args, .. } => {
            check_expr_for_param_assign(cx, *callee, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        }
        HirExpr::MethodCall {
            receiver, args, ..
        }
        | HirExpr::ProtocolCall {
            receiver, args, ..
        } => {
            check_expr_for_param_assign(cx, *receiver, param_locals, diags);
            for arg in args {
                check_expr_for_param_assign(cx, arg.value, param_locals, diags);
            }
        }
        HirExpr::Return { value, .. } => {
            if let Some(val) = value {
                check_expr_for_param_assign(cx, *val, param_locals, diags);
            }
        }
        HirExpr::Field { base, .. } | HirExpr::TupleIndex { base, .. } => {
            check_expr_for_param_assign(cx, *base, param_locals, diags);
        }
        HirExpr::Tuple { elements, .. } | HirExpr::Array { elements, .. } => {
            for &elem in elements {
                check_expr_for_param_assign(cx, elem, param_locals, diags);
            }
        }
        HirExpr::Dict { entries, .. } => {
            for entry in entries {
                check_expr_for_param_assign(cx, entry.key, param_locals, diags);
                check_expr_for_param_assign(cx, entry.value, param_locals, diags);
            }
        }
        // Don't recurse into nested closures — they have their own param scope
        HirExpr::Closure { .. } => {}

        // Leaf expressions
        _ => {}
    }
}

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
