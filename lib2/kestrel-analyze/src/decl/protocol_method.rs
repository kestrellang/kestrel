//! # Protocol Method Analyzer
//!
//! Checks that methods declared directly inside a protocol do NOT have bodies.
//! Protocol methods are declarations only; default implementations go in
//! extensions.
//!
//! ## Diagnostics
//!
//! ### E417 -- `protocol_method_has_body` (Error, Correctness)
//!
//! **Message:** "protocol method '{method}' in '{protocol}' cannot have a body"
//!
//! **Labels:**
//! - Primary: the method declaration that incorrectly has a body
//!   - Span source: `util::entity_span` on the Function child entity
//!   - Message: "body not allowed in protocol method"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Body, NodeKind, Valued};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E417",
    name: "protocol_method_has_body",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ProtocolMethodAnalyzer;

impl Describe for ProtocolMethodAnalyzer {
    fn id(&self) -> &'static str {
        "protocol_method"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ProtocolMethodAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Protocol]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let protocol_name = util::entity_name(cx.query, cx.entity);
        let mut diags = Vec::new();

        // Iterate children looking for Function entities with bodies
        for &child in cx.query.children_of(cx.entity) {
            if !matches!(cx.query.get::<NodeKind>(child), Some(NodeKind::Function)) {
                continue;
            }

            // Check if this method has a body or computed value
            let has_body = cx.query.get::<Body>(child).is_some()
                || cx.query.get::<Valued>(child).is_some();
            if !has_body {
                continue;
            }

            let method_name = util::entity_name(cx.query, child);
            let span = util::entity_span(cx.query, child);

            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "protocol method '{}' in '{}' cannot have a body",
                    method_name, protocol_name
                ),
                labels: vec![DiagLabel {
                    span,
                    message: "body not allowed in protocol method".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        diags
    }
}
