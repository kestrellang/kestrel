//! # Parent Protocol Conformance Analyzer
//!
//! Validates that when a struct/enum conforms to a protocol that inherits from
//! another protocol, the type also explicitly conforms to the parent protocol.
//!
//! For example, if protocol B inherits from A, and struct S conforms to B,
//! then S must also declare conformance to A (so that A's methods are provided).
//!
//! Uses `ResolveTypePath` to resolve conformance AstTypes to protocol entities,
//! then checks each protocol's own Conformances (protocol inheritance) to find
//! parent protocols that the struct/enum must also conform to.
//!
//! ## Diagnostics
//!
//! ### E421 -- `missing_parent_protocol_conformance` (Error, Correctness)
//!
//! **Message:** "'{type_name}' conforms to '{child_protocol}' but not its parent '{parent_protocol}'"
//!
//! **Labels:**
//! - Primary: the struct/enum declaration
//!   - Span source: `util::entity_span` on the struct/enum entity
//!   - Message: "missing conformance to '{parent_protocol}'"
//!
//! **Notes:**
//! - "'{child_protocol}' inherits from '{parent_protocol}', so conforming types must also conform to it"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast::ast_type::AstType;
use kestrel_ast_builder::{ConformanceItem, Conformances, NodeKind};
use kestrel_hecs::Entity;
use kestrel_name_res::{ConformingProtocols, ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E421",
    name: "missing_parent_protocol_conformance",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ParentProtocolConformanceAnalyzer;

impl Describe for ParentProtocolConformanceAnalyzer {
    fn id(&self) -> &'static str {
        "parent_protocol_conformance"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// Resolve an AstType::Named to an entity via ResolveTypePath.
/// Returns None for non-Named types or resolution failures.
fn resolve_conformance_type(cx: &DeclContext<'_>, ast_type: &AstType) -> Option<Entity> {
    let AstType::Named { segments, .. } = ast_type else {
        return None;
    };
    let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
    let result = cx.query.query(ResolveTypePath {
        segments: seg_names,
        context: cx.entity,
        root: cx.root,
    });
    match result {
        TypeResolution::Found(entity) => Some(entity),
        _ => None,
    }
}

/// Collect the positively conformed protocol entities declared *directly* on
/// `entity` (reads the `Conformances` component only). Does not walk
/// inheritance or `ExtensionsFor`: the outer E421 loop already iterates the
/// full transitive set via `ConformingProtocols`, so transitivity is handled
/// at the call site.
fn collect_positive_conformances(cx: &DeclContext<'_>, entity: Entity) -> Vec<Entity> {
    let Some(conformances) = cx.query.get::<Conformances>(entity) else {
        return vec![];
    };
    conformances
        .0
        .iter()
        .filter_map(|item| {
            let ConformanceItem::Positive(ast_type, _) = item else {
                return None;
            };
            resolve_conformance_type(cx, ast_type)
        })
        .collect()
}

impl DeclCheck for ParentProtocolConformanceAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(conformances) = cx.query.get::<Conformances>(cx.entity) else {
            return vec![];
        };

        // Use transitive conformance query — includes direct, extension, and inherited protocols
        let type_conformed = cx.query.query(ConformingProtocols {
            entity: cx.entity,
            root: cx.root,
        });

        if type_conformed.is_empty() {
            return vec![];
        }

        let type_name = util::entity_name(cx.query, cx.entity);
        let mut diags = Vec::new();

        // For each conformed protocol, check its own conformance list (protocol inheritance)
        for &proto_entity in &type_conformed {
            let parent_protocols = collect_positive_conformances(cx, proto_entity);

            for parent_entity in parent_protocols {
                // Check if the struct/enum also conforms to this parent protocol
                if type_conformed.contains(&parent_entity) {
                    continue;
                }

                let child_name = util::entity_name(cx.query, proto_entity);
                let parent_name = util::entity_name(cx.query, parent_entity);

                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!(
                        "'{}' conforms to '{}' but not its parent '{}'",
                        type_name, child_name, parent_name
                    ),
                    labels: vec![DiagLabel {
                        span: util::entity_span(cx.query, cx.entity),
                        message: format!("missing conformance to '{}'", parent_name),
                        is_primary: true,
                    }],
                    notes: vec![format!(
                        "'{}' inherits from '{}', so conforming types must also conform to it",
                        child_name, parent_name
                    )],
                });
            }
        }

        diags
    }
}
