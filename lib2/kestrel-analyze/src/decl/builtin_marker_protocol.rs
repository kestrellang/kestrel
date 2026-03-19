//! # Builtin Marker Protocol Analyzer
//!
//! Validates that builtin protocols with `must_be_marker: true` (from BuiltinKind)
//! have no required members (no Function or TypeAlias children).
//!
//! Uses the `EntityBuiltin` query to look up whether this protocol entity is a
//! registered builtin, then checks the `must_be_marker` flag on its BuiltinKind.
//!
//! ## Diagnostics
//!
//! ### KS419 -- `builtin_must_be_marker` (Error, Correctness)
//!
//! **Message:** "@builtin(.{feature}) must be a marker protocol (no required methods or types)"
//!
//! **Labels:**
//! - Primary: the protocol declaration
//!   - Span source: `util::entity_span` on the protocol entity (declaration span)
//!   - Message: "protocol has required members"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_hir::builtin::BuiltinKind;
use kestrel_name_res::EntityBuiltin;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS419",
    name: "builtin_must_be_marker",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct BuiltinMarkerProtocolAnalyzer;

impl Describe for BuiltinMarkerProtocolAnalyzer {
    fn id(&self) -> &'static str {
        "builtin_marker_protocol"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for BuiltinMarkerProtocolAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Protocol]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Look up whether this protocol is a registered builtin
        let Some(builtin) = cx.query.query(EntityBuiltin { entity: cx.entity }) else {
            return vec![];
        };

        // Check if this builtin protocol requires marker status
        let BuiltinKind::Protocol { must_be_marker: true, .. } = builtin.kind() else {
            return vec![];
        };

        // A marker protocol must have no required members: no Function or TypeAlias children
        let has_required_members = cx.query.children_of(cx.entity).iter().any(|&child| {
            matches!(
                cx.query.get::<NodeKind>(child),
                Some(NodeKind::Function) | Some(NodeKind::TypeAlias)
            )
        });

        if !has_required_members {
            return vec![];
        }

        let span = util::entity_span(cx.query, cx.entity);

        vec![AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "@builtin(.{}) must be a marker protocol (no required methods or types)",
                builtin.name()
            ),
            labels: vec![DiagLabel {
                span,
                message: "protocol has required members".into(),
                is_primary: true,
            }],
            notes: vec![],
        }]
    }
}
