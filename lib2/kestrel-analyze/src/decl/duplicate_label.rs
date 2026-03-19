//! # Duplicate Label Analyzer
//!
//! Checks that no two parameters within an enum case share the same label.
//! Inspects each EnumCase child's Callable component for duplicate labels.
//!
//! ## Diagnostics
//!
//! ### E428 -- `duplicate_enum_label` (Error, Correctness)
//!
//! **Message:** "duplicate label '{label}' in case '{case_name}'"
//!
//! **Labels:**
//! - Primary: the enum case containing the duplicate label
//!   - Span source: `util::entity_span` on the EnumCase child entity
//!   - Message: "duplicate label"
//!
//! **Notes:** (none)

use std::collections::HashSet;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E428",
    name: "duplicate_enum_label",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DuplicateLabelAnalyzer;

impl Describe for DuplicateLabelAnalyzer {
    fn id(&self) -> &'static str {
        "duplicate_enum_label"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DuplicateLabelAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        for &child in cx.query.children_of(cx.entity) {
            if !matches!(cx.query.get::<NodeKind>(child), Some(NodeKind::EnumCase)) {
                continue;
            }

            let Some(callable) = cx.query.get::<Callable>(child) else {
                continue;
            };

            let case_name = util::entity_name(cx.query, child);
            let mut seen = HashSet::new();

            for param in &callable.params {
                let Some(label) = &param.label else {
                    continue;
                };

                if !seen.insert(label.clone()) {
                    // Duplicate label found
                    let span = util::entity_span(cx.query, child);

                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "duplicate label '{}' in case '{}'",
                            label, case_name
                        ),
                        labels: vec![DiagLabel {
                            span,
                            message: "duplicate label".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                    // One diagnostic per case is enough
                    break;
                }
            }
        }

        diags
    }
}
