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
use crate::decl::extern_ffi_safe::is_ffi_safe;
use crate::diagnostic::*;
use crate::util;
use kestrel_ast_builder::{Callable, Name, NodeKind};
use kestrel_hir::builtin::Builtin;
use crate::traits::{DeclCheck, Describe};
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E420",
    name: "fields_not_conforming_to_protocol",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ProtocolFieldConformanceAnalyzer;

impl Describe for ProtocolFieldConformanceAnalyzer {
    fn id(&self) -> &'static str {
        "protocol_field_conformance"
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
        // Check if this type conforms to FFISafe
        let Some(ffi_safe_entity) = cx.query.query(ResolveBuiltin {
            builtin: Builtin::FFISafe,
            root: cx.root,
        }) else {
            return vec![];
        };

        let conforming = cx.query.query(ConformingProtocols {
            entity: cx.entity,
            root: cx.root,
        });
        if !conforming.contains(&ffi_safe_entity) {
            return vec![];
        }

        // Check each field's type conforms to FFISafe
        let mut bad_fields = Vec::new();
        for &child in cx.query.children_of(cx.entity) {
            let Some(kind) = cx.query.get::<NodeKind>(child) else { continue };
            if *kind != NodeKind::Field { continue; }
            // Skip computed properties — only stored fields affect layout
            if cx.query.has::<Callable>(child) { continue; }

            let field_name = cx.query.get::<Name>(child)
                .map(|n| n.0.clone())
                .unwrap_or_else(|| "<unknown>".into());

            let Some(field_ty) = cx.query.query(LowerTypeAnnotation {
                entity: child,
                root: cx.root,
            }) else {
                continue;
            };

            if !is_ffi_safe(cx, &field_ty, ffi_safe_entity) {
                bad_fields.push(field_name);
            }
        }

        if bad_fields.is_empty() {
            return vec![];
        }

        let type_name = util::entity_name(cx.query, cx.entity);
        let span = util::entity_span(cx.query, cx.entity);

        vec![AnalyzeDiagnostic {
            descriptor_id: "E420",
            severity: Severity::Error,
            message: format!(
                "fields of '{}' do not conform to FFISafe: {}",
                type_name,
                bad_fields.join(", ")
            ),
            labels: vec![DiagLabel {
                span,
                message: "type conforms to FFISafe but has non-FFISafe fields".into(),
                is_primary: true,
            }],
            notes: vec![
                "all fields must conform to FFISafe for the type to conform".into(),
            ],
        }]
    }
}
