//! # Cloneable Field Analyzer
//!
//! Checks that structs/enums containing Cloneable fields themselves conform
//! to Cloneable. When a type has all-copyable fields except some that are
//! Cloneable, the parent type must also conform to Cloneable.
//!
//! ## Diagnostics
//!
//! ### E502 -- `cloneable_field_requires_conformance` (Error, Correctness)
//!
//! **Message:** "{kind} '{type_name}' has Cloneable field '{field_name}' but does not conform to Cloneable"
//!
//! **Labels:**
//! - Primary: the field with the Cloneable type
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "this field has a Cloneable type"
//!
//! **Notes:**
//! - "types containing Cloneable fields must conform to Cloneable"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::NodeKind;
use kestrel_hir::builtin::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerTypeAnnotation;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};

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
        // Resolve builtin protocol entities
        let Some(cloneable_entity) = cx.query.query(ResolveBuiltin {
            builtin: Builtin::Cloneable,
            root: cx.root,
        }) else {
            return vec![];
        };
        let Some(copyable_entity) = cx.query.query(ResolveBuiltin {
            builtin: Builtin::Copyable,
            root: cx.root,
        }) else {
            return vec![];
        };

        // Check this type's conformances
        let conforming = cx.query.query(ConformingProtocols {
            entity: cx.entity,
            root: cx.root,
        });

        // Already conforms to Cloneable — no diagnostic needed
        if conforming.contains(&cloneable_entity) {
            return vec![];
        }

        // If the type is Copyable, no need for Cloneable
        if conforming.contains(&copyable_entity) {
            return vec![];
        }

        let type_name = util::entity_name(cx.query, cx.entity);
        let kind_str = match cx.kind {
            NodeKind::Struct => "struct",
            NodeKind::Enum => "enum",
            _ => "type",
        };

        // Walk children looking for fields/enum cases with Cloneable types
        for &child in cx.query.children_of(cx.entity) {
            let child_kind = cx.query.get::<NodeKind>(child);

            match child_kind {
                Some(NodeKind::Field) => {
                    if let Some(field_entity) = self.check_field_cloneable(
                        cx, child, cloneable_entity, copyable_entity,
                    ) {
                        let field_name = util::entity_name(cx.query, field_entity);
                        return vec![self.make_diagnostic(
                            &type_name, &field_name, kind_str, cx, field_entity,
                        )];
                    }
                }
                Some(NodeKind::EnumCase) => {
                    // Enum cases have Field children for associated values
                    for &case_child in cx.query.children_of(child) {
                        if cx.query.get::<NodeKind>(case_child) != Some(&NodeKind::Field) {
                            continue;
                        }
                        if let Some(field_entity) = self.check_field_cloneable(
                            cx, case_child, cloneable_entity, copyable_entity,
                        ) {
                            let field_name = util::entity_name(cx.query, field_entity);
                            return vec![self.make_diagnostic(
                                &type_name, &field_name, kind_str, cx, field_entity,
                            )];
                        }
                    }
                }
                _ => {}
            }
        }

        vec![]
    }
}

impl CloneableFieldAnalyzer {
    /// Check if a field's type conforms to Cloneable (but not Copyable).
    /// Returns the field entity if it does, None otherwise.
    fn check_field_cloneable(
        &self,
        cx: &DeclContext<'_>,
        field: kestrel_hecs::Entity,
        cloneable_entity: kestrel_hecs::Entity,
        copyable_entity: kestrel_hecs::Entity,
    ) -> Option<kestrel_hecs::Entity> {
        // Resolve field type to HirTy
        let hir_ty = cx.query.query(LowerTypeAnnotation {
            entity: field,
            root: cx.root,
        })?;

        // Extract the type entity from Named types
        let type_entity = match &hir_ty {
            HirTy::Named { entity, .. } => *entity,
            _ => return None,
        };

        // Check field type's conformances
        let field_conforming = cx.query.query(ConformingProtocols {
            entity: type_entity,
            root: cx.root,
        });

        // Field is Cloneable but not Copyable — parent needs Cloneable conformance
        if field_conforming.contains(&cloneable_entity)
            && !field_conforming.contains(&copyable_entity)
        {
            Some(field)
        } else {
            None
        }
    }

    fn make_diagnostic(
        &self,
        type_name: &str,
        field_name: &str,
        kind_str: &str,
        cx: &DeclContext<'_>,
        field_entity: kestrel_hecs::Entity,
    ) -> AnalyzeDiagnostic {
        AnalyzeDiagnostic {
            descriptor_id: DESCRIPTORS[0].id,
            severity: DESCRIPTORS[0].default_severity,
            message: format!(
                "{} '{}' has Cloneable field '{}' but does not conform to Cloneable",
                kind_str, type_name, field_name
            ),
            labels: vec![DiagLabel {
                span: util::entity_span(cx.query, field_entity),
                message: "this field has a Cloneable type".into(),
                is_primary: true,
            }],
            notes: vec![
                "types containing Cloneable fields must conform to Cloneable".into(),
            ],
        }
    }
}
