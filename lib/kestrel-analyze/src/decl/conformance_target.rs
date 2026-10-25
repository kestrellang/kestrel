//! # Conformance Target Analyzer
//!
//! Each entry in a type's conformance list (`struct S: P`, `extend T: P`,
//! `protocol Q: P`) must name a protocol. A non-protocol type used to be
//! dropped silently, so the declaration compiled as if the entry were not
//! there (`struct S: S {}`). Unknown names are still dropped (see
//! `check_entry`).
//!
//! ## Diagnostics
//!
//! ### E630 -- `conformance_not_a_protocol` (Error, Correctness)
//!
//! **Message:** "'{name}' is a {kind}, not a protocol"
//!
//! **Labels:**
//! - Primary: the conformance entry
//!   - Span source: the entry's `AstType` span
//!   - Message: "expected a protocol"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use kestrel_ast::AstType;
use kestrel_ast_builder::{ConformanceItem, Conformances, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E630",
    name: "conformance_not_a_protocol",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ConformanceTargetAnalyzer;

impl Describe for ConformanceTargetAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ConformanceTarget
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ConformanceTargetAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::Protocol,
            NodeKind::Extension,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(conformances) = cx.query.get::<Conformances>(cx.entity) else {
            return vec![];
        };
        // Names resolve where the declaration sits (an extension's own scope
        // is its parent module).
        let context = match cx.query.get::<NodeKind>(cx.entity) {
            Some(NodeKind::Extension) => cx.query.parent_of(cx.entity).unwrap_or(cx.root),
            _ => cx.entity,
        };
        conformances
            .0
            .iter()
            .filter_map(|item| match item {
                ConformanceItem::Positive(ty, _) => check_entry(cx, ty, context),
                ConformanceItem::Negative(..) => None,
            })
            .collect()
    }
}

fn check_entry(
    cx: &DeclContext<'_>,
    ty: &AstType,
    _context: kestrel_hecs::Entity,
) -> Option<AnalyzeDiagnostic> {
    let AstType::Named { segments, span } = ty else {
        return None;
    };
    let name = segments
        .iter()
        .map(|s| s.name.as_str())
        .collect::<Vec<_>>()
        .join(".");
    // The same resolution the conformance graph uses (single source of truth).
    let resolved = kestrel_name_res::conformances::resolve_conformance_entity(
        cx.query, ty, cx.entity, cx.root,
    );
    // An unresolved name is not reported yet: existing code (notably
    // `stdlib: false` fixtures naming `Prelude.X`) relies on the entry being
    // dropped. Only a name that resolves to a non-protocol type is rejected.
    let message = match resolved {
        None => return None,
        Some(entity) => match cx.query.get::<NodeKind>(entity) {
            // An alias may name a protocol; its own validation covers it.
            Some(NodeKind::Protocol | NodeKind::TypeAlias) | None => return None,
            Some(kind) => format!("'{name}' is a {}, not a protocol", kind_name(kind)),
        },
    };
    Some(AnalyzeDiagnostic {
        descriptor_id: DESCRIPTORS[0].id,
        severity: DESCRIPTORS[0].default_severity,
        message,
        labels: vec![DiagLabel {
            span: span.clone(),
            message: "expected a protocol".into(),
            is_primary: true,
        }],
        notes: vec!["only protocols can appear in a conformance list".into()],
    })
}

fn kind_name(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Struct => "struct",
        NodeKind::Enum => "enum",
        NodeKind::TypeParameter => "type parameter",
        _ => "type",
    }
}
