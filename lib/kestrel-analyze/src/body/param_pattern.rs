//! # Parameter Pattern Analyzer
//!
//! Validates destructured function/init parameters:
//! - Duplicate bindings in a pattern (e.g., `(a, a)`)
//! - Tuple arity mismatch (e.g., `(a, b, c): (Int, Int)`)
//! - Tuple pattern on non-tuple type (e.g., `(a, b): Int`)
//!
//! ## Diagnostics
//!
//! ### E110 — `duplicate_param_binding` (Error, Correctness)
//! ### E111 — `param_tuple_arity_mismatch` (Error, Correctness)
//! ### E112 — `param_tuple_on_non_tuple` (Error, Correctness)

use std::collections::HashMap;

use crate::context::BodyContext;
use crate::diagnostic::*;
use crate::traits::{BodyCheck, Describe};
use crate::util;
use kestrel_ast_builder::{AstType, Callable, ParamPattern};
use kestrel_hir::body::{HirBody, HirExpr, HirPat, HirPatId};
use kestrel_hir::ty::HirTy;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E110",
        name: "duplicate_param_binding",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E111",
        name: "param_tuple_arity_mismatch",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E112",
        name: "param_tuple_on_non_tuple",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ParamPatternAnalyzer;

impl Describe for ParamPatternAnalyzer {
    fn id(&self) -> &'static str {
        "param_pattern"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl BodyCheck for ParamPatternAnalyzer {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        // Check function/init parameters (from Callable component)
        if let Some(callable) = cx.query.get::<Callable>(cx.entity) {
            let span = util::entity_span(cx.query, cx.entity);
            for param in &callable.params {
                let Some(ref pattern) = param.pattern else {
                    continue;
                };
                let mut seen = HashMap::new();
                check_duplicate_bindings(pattern, &mut seen, &span, &mut diags);
                if let Some(ref ty) = param.ty {
                    check_pattern_type(pattern, ty, &span, &mut diags);
                }
            }
        }

        // Check closure parameters. HirClosureParam.pattern is Some only for
        // destructured params (tuple/struct); simple bindings desugar to a
        // local with no residual pattern.
        for (_id, expr) in cx.hir.exprs.iter() {
            if let HirExpr::Closure { params, .. } = expr {
                for param in params {
                    let Some(pat_id) = param.pattern else {
                        continue;
                    };
                    let pat = &cx.hir.pats[pat_id];
                    let span = hir_pat_span(pat).clone();
                    let mut seen = HashMap::new();
                    check_duplicate_bindings_hir(cx.hir, pat_id, &mut seen, &span, &mut diags);
                    if let Some(ref ty) = param.ty {
                        check_pattern_type_hir(cx.hir, pat_id, ty, &span, &mut diags);
                    }
                }
            }
        }

        diags
    }
}

/// Walk a pattern tree and flag duplicate binding names.
fn check_duplicate_bindings(
    pattern: &ParamPattern,
    seen: &mut HashMap<String, bool>,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match pattern {
        ParamPattern::Binding { name, .. } => {
            if name == "_" {
                return; // wildcards don't count
            }
            if seen.insert(name.clone(), true).is_some() {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate binding '{}' in parameter pattern", name),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: format!("'{}' bound more than once", name),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        },
        ParamPattern::Tuple { elements } => {
            for elem in elements {
                check_duplicate_bindings(elem, seen, span, diags);
            }
        },
        ParamPattern::Struct { fields, .. } => {
            for field in fields {
                check_duplicate_bindings(&field.pattern, seen, span, diags);
            }
        },
        ParamPattern::Wildcard => {},
    }
}

/// Check that a pattern is structurally compatible with its declared type.
fn check_pattern_type(
    pattern: &ParamPattern,
    ty: &AstType,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if let ParamPattern::Tuple { elements } = pattern {
        match ty {
            AstType::Tuple(type_elems, _) => {
                if elements.len() != type_elems.len() {
                    emit_arity_mismatch(elements.len(), type_elems.len(), span, diags);
                }
            },
            _ => emit_tuple_on_non_tuple(span, diags),
        }
    }
}

// ===== HIR-based checks (for closure params) =====

/// Walk an HirPat tree and flag duplicate binding names.
fn check_duplicate_bindings_hir(
    hir: &HirBody,
    pat_id: HirPatId,
    seen: &mut HashMap<String, bool>,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match &hir.pats[pat_id] {
        HirPat::Binding { local, .. } => {
            let name = &hir.locals[*local].name;
            if name == "_" {
                return;
            }
            if seen.insert(name.clone(), true).is_some() {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate binding '{}' in parameter pattern", name),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: format!("'{}' bound more than once", name),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        },
        HirPat::Tuple { prefix, suffix, .. } => {
            for &elem in prefix.iter().chain(suffix.iter()) {
                check_duplicate_bindings_hir(hir, elem, seen, span, diags);
            }
        },
        HirPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(sub) = field.pattern {
                    check_duplicate_bindings_hir(hir, sub, seen, span, diags);
                } else if seen.insert(field.field_name.clone(), true).is_some() {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "duplicate binding '{}' in parameter pattern",
                            field.field_name
                        ),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: format!("'{}' bound more than once", field.field_name),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
        },
        _ => {},
    }
}

/// Check that an HirPat is structurally compatible with its declared type.
fn check_pattern_type_hir(
    hir: &HirBody,
    pat_id: HirPatId,
    ty: &HirTy,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if let HirPat::Tuple {
        prefix,
        has_rest,
        suffix,
        ..
    } = &hir.pats[pat_id]
    {
        match ty {
            HirTy::Tuple(type_elems, _) => {
                let pat_count = prefix.len() + suffix.len();
                if *has_rest {
                    if pat_count > type_elems.len() {
                        emit_arity_mismatch(pat_count, type_elems.len(), span, diags);
                    }
                } else if pat_count != type_elems.len() {
                    emit_arity_mismatch(pat_count, type_elems.len(), span, diags);
                }
            },
            _ => emit_tuple_on_non_tuple(span, diags),
        }
    }
}

/// Get the span from an HirPat.
fn hir_pat_span(pat: &HirPat) -> &kestrel_span2::Span {
    match pat {
        HirPat::Wildcard { span }
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
        | HirPat::Error { span } => span,
    }
}

// ===== Shared diagnostic helpers =====

fn emit_arity_mismatch(
    pat_count: usize,
    type_count: usize,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    diags.push(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[1].id,
        severity: DESCRIPTORS[1].default_severity,
        message: format!(
            "tuple pattern has {} elements but type has {}",
            pat_count, type_count
        ),
        labels: vec![DiagLabel {
            span: span.clone(),
            message: "arity mismatch".into(),
            is_primary: true,
        }],
        notes: vec![],
    });
}

fn emit_tuple_on_non_tuple(span: &kestrel_span2::Span, diags: &mut Vec<AnalyzeDiagnostic>) {
    diags.push(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[2].id,
        severity: DESCRIPTORS[2].default_severity,
        message: "tuple pattern used with non-tuple type".into(),
        labels: vec![DiagLabel {
            span: span.clone(),
            message: "expected a tuple type".into(),
            is_primary: true,
        }],
        notes: vec![],
    });
}
