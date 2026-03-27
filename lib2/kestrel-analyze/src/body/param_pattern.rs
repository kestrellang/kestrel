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
use kestrel_ast::ast_body::{AstExpr, AstPat};
use kestrel_ast_builder::{AstType, Body, Callable, ParamPattern};

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

        // Check closure parameters (walk AST body for Closure expressions)
        if let Some(body_comp) = cx.query.get::<Body>(cx.entity) {
            let ast_body = &body_comp.0;
            for (_id, expr) in ast_body.exprs.iter() {
                if let AstExpr::Closure { params, .. } = expr {
                    for param in params {
                        let pat = &ast_body.pats[param.pattern];
                        // Skip simple bindings and wildcards
                        if matches!(pat, AstPat::Binding { .. } | AstPat::Wildcard { .. }) {
                            continue;
                        }
                        let span = pat_span(pat);
                        // Duplicate bindings
                        let mut seen = HashMap::new();
                        check_duplicate_bindings_ast(pat, ast_body, &mut seen, &span, &mut diags);
                        // Structural type checks
                        if let Some(ref ty) = param.ty {
                            check_pattern_type_ast(pat, ast_body, ty, &span, &mut diags);
                        }
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
        }
        ParamPattern::Tuple { elements } => {
            for elem in elements {
                check_duplicate_bindings(elem, seen, span, diags);
            }
        }
        ParamPattern::Struct { fields, .. } => {
            for field in fields {
                check_duplicate_bindings(&field.pattern, seen, span, diags);
            }
        }
        ParamPattern::Wildcard => {}
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
            }
            _ => emit_tuple_on_non_tuple(span, diags),
        }
    }
}

// ===== AST-based checks (for closure params) =====

/// Walk an AstPat tree and flag duplicate binding names.
fn check_duplicate_bindings_ast(
    pat: &AstPat,
    body: &kestrel_ast::ast_body::AstBody,
    seen: &mut HashMap<String, bool>,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    match pat {
        AstPat::Binding { name, .. } => {
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
        }
        AstPat::Tuple { prefix, suffix, .. } => {
            for &elem_id in prefix.iter().chain(suffix.iter()) {
                check_duplicate_bindings_ast(&body.pats[elem_id], body, seen, span, diags);
            }
        }
        AstPat::Struct { fields, .. } => {
            for field in fields {
                if let Some(pat_id) = field.pattern {
                    check_duplicate_bindings_ast(&body.pats[pat_id], body, seen, span, diags);
                } else {
                    // Shorthand: field name is the binding
                    if seen.insert(field.field_name.clone(), true).is_some() {
                        diags.push(AnalyzeDiagnostic {
                            descriptor_id: DESCRIPTORS[0].id,
                            severity: DESCRIPTORS[0].default_severity,
                            message: format!("duplicate binding '{}' in parameter pattern", field.field_name),
                            labels: vec![DiagLabel {
                                span: span.clone(),
                                message: format!("'{}' bound more than once", field.field_name),
                                is_primary: true,
                            }],
                            notes: vec![],
                        });
                    }
                }
            }
        }
        _ => {}
    }
}

/// Check that an AstPat is structurally compatible with its declared type.
fn check_pattern_type_ast(
    pat: &AstPat,
    body: &kestrel_ast::ast_body::AstBody,
    ty: &AstType,
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    if let AstPat::Tuple { prefix, has_rest, suffix, .. } = pat {
        match ty {
            AstType::Tuple(type_elems, _) => {
                let pat_count = prefix.len() + suffix.len();
                if *has_rest {
                    // Rest absorbs middle elements — just check min arity
                    if pat_count > type_elems.len() {
                        emit_arity_mismatch(pat_count, type_elems.len(), span, diags);
                    }
                } else if pat_count != type_elems.len() {
                    emit_arity_mismatch(pat_count, type_elems.len(), span, diags);
                }
            }
            _ => emit_tuple_on_non_tuple(span, diags),
        }
    }
}

/// Get the span from an AstPat.
fn pat_span(pat: &AstPat) -> kestrel_span2::Span {
    match pat {
        AstPat::Wildcard { span }
        | AstPat::Binding { span, .. }
        | AstPat::Tuple { span, .. }
        | AstPat::Literal { span, .. }
        | AstPat::Range { span, .. }
        | AstPat::Enum { span, .. }
        | AstPat::Struct { span, .. }
        | AstPat::Array { span, .. }
        | AstPat::At { span, .. }
        | AstPat::Or { span, .. }
        | AstPat::Rest { span }
        | AstPat::Error { span } => span.clone(),
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

fn emit_tuple_on_non_tuple(
    span: &kestrel_span2::Span,
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
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
