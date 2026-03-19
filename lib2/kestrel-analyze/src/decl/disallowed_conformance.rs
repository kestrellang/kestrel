//! # Disallowed Conformance Analyzer
//!
//! Checks that enums do not conform to protocols that disallow enum
//! conformance (e.g., `FFISafe` has `disallow_enum_conformance: true`).
//!
//! ## Diagnostics
//!
//! ### E422 -- `disallowed_enum_conformance` (Error, Correctness)
//!
//! **Message:** "enum '{enum_name}' cannot conform to protocol '{protocol_name}'"
//!
//! **Labels:**
//! - Primary: the enum declaration
//!   - Span source: `util::entity_span` on the enum entity
//!   - Message: "enums cannot conform to this protocol"
//!
//! **Notes:**
//! - "'{protocol_name}' only allows struct conformance"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Conformances, ConformanceItem, NodeKind};
use kestrel_hir::builtin::BuiltinKind;
use kestrel_name_res::{EntityBuiltin, ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E422",
    name: "disallowed_enum_conformance",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct DisallowedConformanceAnalyzer;

impl Describe for DisallowedConformanceAnalyzer {
    fn id(&self) -> &'static str {
        "disallowed_conformance"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DisallowedConformanceAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(conformances) = cx.query.get::<Conformances>(cx.entity) else {
            return vec![];
        };

        let enum_name = util::entity_name(cx.query, cx.entity);
        let mut diags = Vec::new();

        for item in &conformances.0 {
            let ConformanceItem::Positive(ast_type, _) = item else {
                continue;
            };

            // Resolve the conformance type to an entity
            let kestrel_ast::ast_type::AstType::Named { segments, .. } = ast_type else {
                continue;
            };
            let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
            let result = cx.query.query(ResolveTypePath {
                segments: seg_names,
                context: cx.entity,
                root: cx.root,
            });
            let TypeResolution::Found(proto_entity) = result else {
                continue;
            };

            // Check if this protocol has disallow_enum_conformance
            let Some(builtin) = cx.query.query(EntityBuiltin { entity: proto_entity }) else {
                continue;
            };
            if let BuiltinKind::Protocol { disallow_enum_conformance: true, .. } = builtin.kind() {
                let proto_name = util::entity_name(cx.query, proto_entity);
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!(
                        "enum '{}' cannot conform to protocol '{}'",
                        enum_name, proto_name
                    ),
                    labels: vec![DiagLabel {
                        span: util::entity_span(cx.query, cx.entity),
                        message: "enums cannot conform to this protocol".into(),
                        is_primary: true,
                    }],
                    notes: vec![format!("'{}' only allows struct conformance", proto_name)],
                });
            }
        }

        diags
    }
}
