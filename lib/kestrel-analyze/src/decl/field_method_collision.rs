//! # Field/Method Collision Analyzer
//!
//! Detects an EXTENSION instance method declared with the same name as a
//! stored instance field on the extended struct, when the method is callable
//! with no arguments. Member resolution cannot disambiguate that pair —
//! `self.name` inside the method binds back to the method itself, producing
//! silent infinite recursion at runtime (#130) — so the collision is rejected
//! at declaration time. In-body collisions are already rejected by E475
//! (`duplicate_symbol.rs`), and a method requiring labeled arguments (the stdlib's
//! `slice` field + `slice(from:to:)` pattern) is disambiguated by its labels
//! at every use site, so neither is flagged here.
//!
//! ## Diagnostics
//!
//! ### E467 — `method_shadows_field` (Error, Correctness)
//!
//! **Message:** "method '{name}' shadows the stored field '{name}' of '{type}'"
//!
//! **Labels:**
//! - Primary: the colliding method declaration
//!   - Span source: `util::entity_span` on the method entity (declaration name)
//!   - Message: "method declared here"
//! - Secondary: the shadowed stored field
//!   - Span source: `util::entity_span` on the field entity (declaration name)
//!   - Message: "stored field declared here"
//!
//! **Notes:**
//! - "rename the method or the field: 'self.{name}' cannot refer to both"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, NodeKind, Static};
use kestrel_hecs::Entity;
use kestrel_name_res::ExtensionTargetEntity;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "E467",
    name: "method_shadows_field",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct FieldMethodCollisionAnalyzer;

impl Describe for FieldMethodCollisionAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::FieldMethodCollision
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

/// Stored instance fields of `type_entity`: `Field` children with no
/// `Callable` (computed properties carry one) and no `Static` marker.
fn stored_instance_fields(cx: &DeclContext<'_>, type_entity: Entity) -> Vec<(String, Entity)> {
    cx.query
        .children_of(type_entity)
        .iter()
        .copied()
        .filter(|&c| {
            cx.query.get::<NodeKind>(c) == Some(&NodeKind::Field)
                && cx.query.get::<Callable>(c).is_none()
                && cx.query.get::<Static>(c).is_none()
        })
        .map(|c| (util::entity_name(cx.query, c), c))
        .collect()
}

impl DeclCheck for FieldMethodCollisionAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Extension]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        // The type whose fields the extension's methods share a member
        // namespace with. In-body collisions are E475's job.
        let Some(field_owner) = cx.query.query(ExtensionTargetEntity {
            extension: cx.entity,
            root: cx.root,
        }) else {
            return vec![];
        };
        if cx.query.get::<NodeKind>(field_owner) != Some(&NodeKind::Struct) {
            return vec![];
        }

        let fields = stored_instance_fields(cx, field_owner);
        if fields.is_empty() {
            return vec![];
        }
        let type_name = util::entity_name(cx.query, field_owner);

        let mut diags = Vec::new();
        for &child in cx.query.children_of(cx.entity) {
            if cx.query.get::<NodeKind>(child) != Some(&NodeKind::Function)
                || cx.query.get::<Static>(child).is_some()
            {
                continue;
            }
            // Only a method callable with NO arguments collides with a bare
            // field access; a method requiring labeled args (the stdlib's
            // `slice` field + `slice(from:to:)` pattern) is disambiguated by
            // its labels at every use site.
            let callable_with_args = cx
                .query
                .get::<Callable>(child)
                .is_some_and(|c| c.params.iter().any(|p| p.default_entity.is_none()));
            if callable_with_args {
                continue;
            }
            let method_name = util::entity_name(cx.query, child);
            let Some((_, field)) = fields.iter().find(|(n, _)| *n == method_name) else {
                continue;
            };
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!(
                    "method '{method_name}' shadows the stored field '{method_name}' of '{type_name}'"
                ),
                labels: vec![
                    DiagLabel {
                        span: util::entity_span(cx.query, child),
                        message: "method declared here".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: util::entity_span(cx.query, *field),
                        message: "stored field declared here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec![format!(
                    "rename the method or the field: 'self.{method_name}' cannot refer to both"
                )],
            });
        }
        diags
    }
}
