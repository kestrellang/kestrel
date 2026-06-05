//! # Static Context Analyzer
//!
//! Checks that the `static` modifier is only used on functions inside
//! struct, enum, protocol, or extension declarations. Static at module
//! level is invalid.
//!
//! ## Diagnostics
//!
//! ### E418 -- `static_in_wrong_context` (Error, Correctness)
//!
//! **Message:** "'{name}' cannot be static in this context"
//!
//! **Labels:**
//! - Primary: the function declaration with invalid `static`
//!   - Span source: `util::entity_span` on the function entity (declaration span)
//!   - Message: "static is not allowed at module level"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{NodeKind, Static};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E418",
    name: "static_in_wrong_context",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct StaticContextAnalyzer;

impl Describe for StaticContextAnalyzer {
    fn id(&self) -> &'static str {
        "static_context"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for StaticContextAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Only applies to functions with the Static marker component
        if cx.query.get::<Static>(cx.entity).is_none() {
            return vec![];
        }

        // Check parent: static is valid inside Struct, Enum, Protocol, Extension
        if let Some(parent) = cx.query.parent_of(cx.entity)
            && matches!(
                cx.query.get::<NodeKind>(parent),
                Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol | NodeKind::Extension)
            )
        {
            return vec![];
        }

        let name = util::entity_name(cx.query, cx.entity);
        let span = util::entity_span(cx.query, cx.entity);

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("'{}' cannot be static in this context", name),
            labels: vec![DiagLabel {
                span,
                message: "static is not allowed at module level".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}
