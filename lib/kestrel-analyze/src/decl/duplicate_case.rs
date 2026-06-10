//! # Duplicate Case Analyzer
//!
//! Checks that no two cases within an enum share the same name.
//!
//! ## Diagnostics
//!
//! ### E427 -- `duplicate_enum_case` (Error, Correctness)
//!
//! **Message:** "duplicate enum case '{case_name}'"
//!
//! **Labels:**
//! - Primary: the duplicate case declaration
//!   - Span source: `util::entity_span` on the second EnumCase child entity
//!   - Message: "duplicate case defined here"
//! - Secondary: the first case declaration with the same name
//!   - Span source: `util::entity_span` on the first EnumCase child entity
//!   - Message: "first defined here"
//!
//! **Notes:** (none)

use std::collections::HashMap;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_span::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E427",
    name: "duplicate_enum_case",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DuplicateCaseAnalyzer;

impl Describe for DuplicateCaseAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::DuplicateEnumCase
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DuplicateCaseAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut seen: HashMap<String, Span> = HashMap::new();
        let mut diags = Vec::new();

        for child in util::children_of_kind(cx.query, cx.entity, NodeKind::EnumCase) {
            let name = util::entity_name(cx.query, child);
            let span = util::entity_span(cx.query, child);

            if let Some(first_span) = seen.get(&name) {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("duplicate enum case '{}'", name),
                    labels: vec![
                        DiagLabel {
                            span,
                            message: "duplicate case defined here".into(),
                            is_primary: true,
                        },
                        DiagLabel {
                            span: first_span.clone(),
                            message: "first defined here".into(),
                            is_primary: false,
                        },
                    ],
                    notes: vec![],
                });
            } else {
                seen.insert(name, span);
            }
        }

        diags
    }
}
