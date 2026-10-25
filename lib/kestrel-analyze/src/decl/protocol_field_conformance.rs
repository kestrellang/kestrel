//! # Protocol Field Conformance Analyzer
//!
//! Validates that when a struct conforms to a protocol with
//! `requires_fields_conform` (e.g. FFISafe), all stored fields also conform.
//!
//! ## Diagnostics
//!
//! ### E420 -- `fields_not_conforming_to_protocol` (Error, Correctness)
//!
//! **Message:** "fields of '{type_name}' do not conform to '{protocol}'"

use crate::context::DeclContext;
use crate::decl::extern_ffi_safe::{conforms_to_builtin_protocol, tuple_propagates};
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, Name, NodeKind};
use kestrel_hir::builtin::BuiltinKind;
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_name_res::{ConformingProtocols, EntityBuiltin};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E420",
    name: "fields_not_conforming_to_protocol",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ProtocolFieldConformanceAnalyzer;

impl Describe for ProtocolFieldConformanceAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ProtocolFieldConformance
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ProtocolFieldConformanceAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Struct, NodeKind::Enum]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // Which protocols does this type claim? For each, ask its BuiltinKind
        // whether conformance propagates to fields. This used to resolve
        // `Builtin::FFISafe` directly, which made `requires_fields_conform` a
        // flag with no readers — the module doc advertised a data-driven rule
        // that did not exist, and a second protocol setting the flag would
        // have been silently ignored.
        let conforming = cx.query.query(ConformingProtocols {
            entity: cx.entity,
            root: cx.root,
        });

        let mut diags = Vec::new();
        for &protocol in conforming.iter() {
            let Some(builtin) = cx.query.query(EntityBuiltin { entity: protocol }) else {
                continue;
            };
            let BuiltinKind::Protocol {
                requires_fields_conform: true,
                ..
            } = builtin.kind()
            else {
                continue;
            };
            let tuples_propagate = tuple_propagates(cx, protocol);
            let protocol_name = util::entity_name(cx.query, protocol);

            let mut bad_fields = Vec::new();
            for child in util::children_of_kind(cx.query, cx.entity, NodeKind::Field) {
                // Skip computed properties — only stored fields affect layout
                if cx.query.has::<Callable>(child) {
                    continue;
                }

                let field_name = cx
                    .query
                    .get::<Name>(child)
                    .map(|n| n.0.clone())
                    .unwrap_or_else(|| "<unknown>".into());

                let Some(field_ty) = cx.query.query(LowerTypeAnnotation {
                    entity: child,
                    root: cx.root,
                }) else {
                    continue;
                };

                if !conforms_to_builtin_protocol(cx, &field_ty, protocol, tuples_propagate) {
                    bad_fields.push(field_name);
                }
            }

            if bad_fields.is_empty() {
                continue;
            }

            let type_name = util::entity_name(cx.query, cx.entity);
            let span = util::entity_span(cx.query, cx.entity);

            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E420",
                severity: Severity::Error,
                message: format!(
                    "fields of '{}' do not conform to {}: {}",
                    type_name,
                    protocol_name,
                    bad_fields.join(", ")
                ),
                labels: vec![DiagLabel {
                    span,
                    message: format!(
                        "type conforms to {protocol_name} but has non-{protocol_name} fields"
                    ),
                    is_primary: true,
                }],
                notes: vec![format!(
                    "all fields must conform to {protocol_name} for the type to conform"
                )],
            });
        }

        diags
    }
}
