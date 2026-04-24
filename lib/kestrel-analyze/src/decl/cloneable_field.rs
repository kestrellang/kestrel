//! # Cloneable Field Analyzer
//!
//! Checks that structs/enums whose copy-semantics are forced to
//! `NotCopyable` by a Cloneable child opt in to Cloneable themselves.
//!
//! Routes through `NominalCopySemantics` (in `kestrel-semantics`), which
//! already distinguishes *explicit* Cloneable conformance from transitive
//! Copyable inheritance — a transitive check would never fire because
//! `Cloneable: Copyable` makes every Cloneable type also Copyable.
//!
//! ## Diagnostics
//!
//! ### E502 -- `cloneable_field_requires_conformance` (Error, Correctness)
//!
//! **Message:** "{kind} '{type_name}' has Cloneable field '{field_name}' but does not conform to Cloneable"
//!
//! **Labels:**
//! - Primary: the container declaration (struct/enum)
//!   - Span source: `util::entity_span` on the container entity
//!   - Message: "this type needs to conform to Cloneable"
//!
//! **Notes:**
//! - "types containing Cloneable fields must conform to Cloneable"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_semantics::{CopySemanticsReason, NominalCopySemantics};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E502",
    name: "cloneable_field_requires_conformance",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct CloneableFieldAnalyzer;

impl Describe for CloneableFieldAnalyzer {
    fn id(&self) -> &'static str {
        "cloneable_field"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for CloneableFieldAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let info = cx.query.query(NominalCopySemantics {
            entity: cx.entity,
            root: cx.root,
        });

        let CopySemanticsReason::CloneableChildRequiresConformance(child) = info.reason else {
            return vec![];
        };

        let type_name = util::entity_name(cx.query, cx.entity);
        let field_name = util::entity_name(cx.query, child);
        let kind_str = match cx.kind {
            NodeKind::Struct => "struct",
            NodeKind::Enum => "enum",
            _ => "type",
        };

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "{} '{}' has Cloneable field '{}' but does not conform to Cloneable",
                kind_str, type_name, field_name
            ),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, cx.entity),
                message: "this type needs to conform to Cloneable".into(),
                is_primary: true,
            }],
            notes: vec!["types containing Cloneable fields must conform to Cloneable".into()],
        }]
    }
}
