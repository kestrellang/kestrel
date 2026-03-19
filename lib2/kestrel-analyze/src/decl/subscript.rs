//! # Subscript Analyzer
//!
//! Validates that subscript declarations are well-formed:
//! - Subscripts must have at least one parameter
//! - Subscripts must have a body (unless inside a protocol)
//!
//! ## Diagnostics
//!
//! ### E607 -- `subscript_missing_parameters` (Error, Correctness)
//!
//! **Message:** "subscript must have at least one parameter"
//!
//! **Labels:**
//! - Primary: the subscript declaration
//!   - Span source: `util::entity_span` on the subscript entity (declaration span)
//!   - Message: "add at least one parameter"
//!
//! **Notes:**
//! - "Subscripts provide indexed access and require parameters."
//! - "Use a computed property instead if no parameters are needed."
//!
//! ### E608 -- `subscript_missing_body` (Error, Correctness)
//!
//! **Message:** "subscript must have a body"
//!
//! **Labels:**
//! - Primary: the subscript declaration
//!   - Span source: `util::entity_span` on the subscript entity (declaration span)
//!   - Message: "add a body to this subscript"
//!
//! **Notes:**
//! - "Provide a body with { expr } or { get { } set { } } syntax."

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Body, Callable, NodeKind, Valued};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E607",
        name: "subscript_missing_parameters",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E608",
        name: "subscript_missing_body",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct SubscriptAnalyzer;

impl Describe for SubscriptAnalyzer {
    fn id(&self) -> &'static str {
        "subscript"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for SubscriptAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Subscript]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let span = util::entity_span(cx.query, cx.entity);

        // Check 1: subscript must have at least one parameter
        if let Some(callable) = cx.query.get::<Callable>(cx.entity) {
            if callable.params.is_empty() {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: "subscript must have at least one parameter".into(),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "add at least one parameter".into(),
                        is_primary: true,
                    }],
                    notes: vec![
                        "Subscripts provide indexed access and require parameters.".into(),
                        "Use a computed property instead if no parameters are needed.".into(),
                    ],
                });
            }
        }

        // Check 2: subscript must have a body (unless inside a protocol)
        if let Some(parent) = cx.query.parent_of(cx.entity) {
            if matches!(cx.query.get::<NodeKind>(parent), Some(NodeKind::Protocol)) {
                return diags;
            }
        }

        let has_body = cx.query.get::<Body>(cx.entity).is_some()
            || cx.query.get::<Valued>(cx.entity).is_some();

        if !has_body {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: "subscript must have a body".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "add a body to this subscript".into(),
                    is_primary: true,
                }],
                notes: vec![
                    "Provide a body with { expr } or { get { } set { } } syntax.".into(),
                ],
            });
        }

        diags
    }
}
