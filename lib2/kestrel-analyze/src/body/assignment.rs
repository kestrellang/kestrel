//! # Assignment Validation Analyzer
//!
//! Checks that assignment targets are valid and mutable:
//! - Cannot assign to `let` bindings (immutable locals)
//! - Cannot assign to immutable fields (no `Settable` component)
//! - Cannot assign to arbitrary expressions (calls, literals, etc.)
//!
//! In initializers, assigning to `self.field` is allowed even for let fields,
//! since the initializer is constructing the instance.
//!
//! ## Diagnostics
//!
//! ### E200 — `assign_to_immutable` (Error, Correctness)
//!
//! **Message:** "cannot assign to immutable variable '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the target `HirExprId`
//!   - Message: "declared as 'let'"
//!
//! **Notes:** (none)
//!
//! ### E201 — `assign_to_immutable_field` (Error, Correctness)
//!
//! **Message:** "cannot assign to immutable field '{name}'"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the target `HirExprId`
//!   - Message: "field is not settable"
//!
//! **Notes:** (none)
//!
//! ### E202 — `assign_to_expression` (Error, Correctness)
//!
//! **Message:** "cannot assign to this expression"
//!
//! **Labels:**
//! - Primary: the assignment target
//!   - Span source: `util::expr_span` on the target `HirExprId`
//!   - Message: "not a valid assignment target"
//!
//! **Notes:** (none)

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{NodeKind, Settable};
use kestrel_hir::body::*;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E200",
        name: "assign_to_immutable",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E201",
        name: "assign_to_immutable_field",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E202",
        name: "assign_to_expression",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct AssignmentAnalyzer;

impl Describe for AssignmentAnalyzer {
    fn id(&self) -> &'static str {
        "assignment"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for AssignmentAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let is_initializer = cx
            .query
            .get::<NodeKind>(cx.entity)
            .is_some_and(|k| *k == NodeKind::Initializer);

        let mut diags = Vec::new();

        // Walk all expressions looking for Assign nodes
        for (_id, expr) in cx.hir.exprs.iter() {
            let HirExpr::Assign { target, .. } = expr else {
                continue;
            };
            check_target(cx, *target, is_initializer, &mut diags);
        }

        diags
    }
}

/// Validate an assignment target, emitting diagnostics for violations.
fn check_target(
    cx: &BodyContext<'_>,
    target: HirExprId,
    is_initializer: bool,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &cx.hir.exprs[target] {
        // Local variable: check is_mut
        HirExpr::Local(local_id, _) => {
            let local = &cx.hir.locals[*local_id];
            if !local.is_mut {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!(
                        "cannot assign to immutable variable '{}'",
                        local.name
                    ),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, target),
                        message: "declared as 'let'".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        // Field access: check Settable component on the resolved entity.
        // In an initializer, self.field assignments are always allowed.
        HirExpr::Field { base, name, .. } => {
            // Check if base is `self` in an initializer
            let is_self_in_init = is_initializer && is_self_local(cx, *base);
            if is_self_in_init {
                return;
            }

            // Look up the resolved field entity
            if let Some(&field_entity) = cx.typed.resolutions.get(&target) {
                let is_settable = cx.query.get::<Settable>(field_entity).is_some();
                if !is_settable {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!("cannot assign to immutable field '{}'", name),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, target),
                            message: "field is not settable".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
            // If no resolution, inference had an error — don't double-report
        }

        // Tuple index: check if the base is mutable
        HirExpr::TupleIndex { base, index, .. } => {
            let is_self_in_init = is_initializer && is_self_local(cx, *base);
            if is_self_in_init {
                return;
            }

            // Tuple elements are mutable if the local holding the tuple is mutable.
            // Check if the base is a mutable local.
            if !is_mutable_base(cx, *base) {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[1].id,
                    severity: DESCRIPTORS[1].default_severity,
                    message: format!("cannot assign to immutable tuple element '{}'", index),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, target),
                        message: "field is not settable".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        // Def references (e.g., module-level fields) — check Settable
        HirExpr::Def(entity, _, _) => {
            let is_field = cx
                .query
                .get::<NodeKind>(*entity)
                .is_some_and(|k| *k == NodeKind::Field);
            if is_field {
                let is_settable = cx.query.get::<Settable>(*entity).is_some();
                if !is_settable {
                    let name = util::entity_name(cx.query, *entity);
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!("cannot assign to immutable field '{}'", name),
                        labels: vec![DiagLabel {
                            span: util::expr_span(cx.hir, target),
                            message: "field is not settable".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            } else {
                // Not a field — invalid target
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[2].id,
                    severity: DESCRIPTORS[2].default_severity,
                    message: "cannot assign to this expression".into(),
                    labels: vec![DiagLabel {
                        span: util::expr_span(cx.hir, target),
                        message: "not a valid assignment target".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        // All other expressions are invalid assignment targets
        _ => {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[2].id,
                severity: DESCRIPTORS[2].default_severity,
                message: "cannot assign to this expression".into(),
                labels: vec![DiagLabel {
                    span: util::expr_span(cx.hir, target),
                    message: "not a valid assignment target".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }
    }
}

/// Check if an expression is a reference to `self` (first parameter, index 0).
fn is_self_local(cx: &BodyContext<'_>, expr_id: HirExprId) -> bool {
    if let HirExpr::Local(local_id, _) = &cx.hir.exprs[expr_id] {
        cx.hir.locals[*local_id].name == "self"
    } else {
        false
    }
}

/// Check if a base expression refers to a mutable local.
fn is_mutable_base(cx: &BodyContext<'_>, expr_id: HirExprId) -> bool {
    if let HirExpr::Local(local_id, _) = &cx.hir.exprs[expr_id] {
        cx.hir.locals[*local_id].is_mut
    } else {
        // Conservative: for non-local bases (fields, method returns, etc.),
        // we don't have enough info to determine mutability at this level.
        // The type inference phase handles these cases.
        true
    }
}
