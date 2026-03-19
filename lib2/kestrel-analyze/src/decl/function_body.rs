//! # Function Body Analyzer
//!
//! Checks that non-protocol, non-extern functions have a body.
//! Functions inside protocols are declarations only (no body expected).
//! Extern functions have bodies provided externally.
//!
//! ## Diagnostics
//!
//! ### E606 -- `function_missing_body` (Error, Correctness)
//!
//! **Message:** "function '{name}' requires a body"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity (declaration span)
//!   - Message: "function declared without body"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Attributes, Body, Intrinsic, NodeKind, Valued};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E606",
    name: "function_missing_body",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct FunctionBodyAnalyzer;

impl Describe for FunctionBodyAnalyzer {
    fn id(&self) -> &'static str {
        "function_body"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for FunctionBodyAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Skip functions inside protocols -- they're declarations only
        if let Some(parent) = cx.query.parent_of(cx.entity) {
            if matches!(cx.query.get::<NodeKind>(parent), Some(NodeKind::Protocol)) {
                return vec![];
            }
        }

        // Skip intrinsic functions (lang module builtins with no implementation)
        if cx.query.has::<Intrinsic>(cx.entity) {
            return vec![];
        }

        // Skip extern and builtin functions
        if let Some(attrs) = cx.query.get::<Attributes>(cx.entity) {
            if attrs.0.iter().any(|a| a.name == "extern" || a.name == "builtin") {
                return vec![];
            }
        }

        // Function has a body or computed value -- no error
        if cx.query.get::<Body>(cx.entity).is_some()
            || cx.query.get::<Valued>(cx.entity).is_some()
        {
            return vec![];
        }

        let name = util::entity_name(cx.query, cx.entity);
        let span = util::entity_span(cx.query, cx.entity);

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("function '{}' requires a body", name),
            labels: vec![DiagLabel {
                span,
                message: "function declared without body".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}
