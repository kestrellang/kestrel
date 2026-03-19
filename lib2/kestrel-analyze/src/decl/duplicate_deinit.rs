//! # Duplicate Deinit Analyzer
//!
//! Checks that a struct has at most one `deinit` declaration. Multiple
//! deinits are invalid -- a struct can only have a single destructor.
//!
//! ## Diagnostics
//!
//! ### KS423 -- `duplicate_deinit` (Error, Correctness)
//!
//! **Message:** "struct `{name}` already has a deinit"
//!
//! **Labels:**
//! - Secondary: the first deinit declaration
//!   - Span source: `util::entity_span` on the first Deinit child entity
//!   - Message: "first deinit defined here"
//! - Primary: the duplicate deinit declaration
//!   - Span source: `util::entity_span` on the second Deinit child entity
//!   - Message: "duplicate deinit"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS423",
    name: "duplicate_deinit",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DuplicateDeinitAnalyzer;

impl Describe for DuplicateDeinitAnalyzer {
    fn id(&self) -> &'static str {
        "duplicate_deinit"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DuplicateDeinitAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Collect all Deinit children
        let deinits: Vec<_> = cx
            .query
            .children_of(cx.entity)
            .iter()
            .copied()
            .filter(|&child| matches!(cx.query.get::<NodeKind>(child), Some(NodeKind::Deinit)))
            .collect();

        if deinits.len() <= 1 {
            return vec![];
        }

        let struct_name = util::entity_name(cx.query, cx.entity);
        let first_span = util::entity_span(cx.query, deinits[0]);
        let dup_span = util::entity_span(cx.query, deinits[1]);

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("struct `{}` already has a deinit", struct_name),
            labels: vec![
                DiagLabel {
                    span: first_span,
                    message: "first deinit defined here".into(),
                    is_primary: false,
                },
                DiagLabel {
                    span: dup_span,
                    message: "duplicate deinit".into(),
                    is_primary: true,
                },
            ],
            notes: vec![],
        }]
    }
}
