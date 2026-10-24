//! # Copyable Deinit Analyzer
//!
//! Checks that a plain Copyable (bitwise) struct/enum does not declare a
//! `deinit`. Copyable means "implicit memcpy at every use", so every copy
//! would run the deinit again (double-close, double-free). Types that own a
//! resource must be `: not Copyable` (move-only) or `: Cloneable`.
//!
//! Routes through `NominalCopySemantics` so the rule agrees with the copy
//! classifier: types that are already NotCopyable (explicitly or via a
//! non-Copyable field) or Cloneable are accepted.
//!
//! ## Diagnostics
//!
//! ### E507 -- `copyable_with_deinit` (Error, Correctness)
//!
//! **Message:** "Copyable type '{type_name}' cannot have a deinit"
//!
//! **Labels:**
//! - Primary: the deinit declaration
//!   - Span source: `util::entity_span` on the first Deinit child entity
//!   - Message: "this deinit would run on every implicit copy"
//!
//! **Notes:**
//! - "mark the type `: not Copyable` to make it move-only, or `: Cloneable` for explicit copies"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_semantics::{CopySemantics, NominalCopySemantics};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E507",
    name: "copyable_with_deinit",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct CopyableDeinitAnalyzer;

impl Describe for CopyableDeinitAnalyzer {
    fn id(&self) -> &'static str {
        "copyable_deinit"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for CopyableDeinitAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(deinit) = cx
            .query
            .children_of(cx.entity)
            .iter()
            .copied()
            .find(|&child| matches!(cx.query.get::<NodeKind>(child), Some(NodeKind::Deinit)))
        else {
            return vec![];
        };

        // Only plain bitwise-Copyable types are wrong; NotCopyable and
        // Cloneable types legitimately own resources.
        let info = cx.query.query(NominalCopySemantics {
            entity: cx.entity,
            root: cx.root,
        });
        if info.semantics != CopySemantics::Copyable {
            return vec![];
        }

        let type_name = util::entity_name(cx.query, cx.entity);
        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!("Copyable type '{}' cannot have a deinit", type_name),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, deinit),
                message: "this deinit would run on every implicit copy".into(),
                is_primary: true,
            }],
            notes: vec![
                "mark the type `: not Copyable` to make it move-only, or `: Cloneable` for explicit copies"
                    .into(),
            ],
        }]
    }
}
