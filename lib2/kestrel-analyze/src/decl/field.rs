//! # Field Analyzer
//!
//! Validates field properties according to Kestrel's semantics:
//! - Computed properties must use `var`, not `let`
//! - Enums cannot have non-static stored fields
//! - Static stored properties are not supported in generic types
//!
//! A field is "computed" if it has a `Valued` component but no `Settable`
//! without also having `Gettable` from a property accessor (i.e., get/set
//! blocks). In practice, computed = has Valued + no default-value pattern.
//! The simplest proxy: a field is computed if it has a `Callable` component
//! (computed getters have a Callable with Borrowing receiver).
//!
//! ## Diagnostics
//!
//! ### E413 -- `computed_property_must_be_var` (Error, Correctness)
//!
//! **Message:** "computed properties must use 'var'"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "computed property declared with 'let'"
//!
//! **Notes:** (none)
//!
//! ### E415 -- `enum_stored_field` (Error, Correctness)
//!
//! **Message:** "enums cannot have stored fields"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "stored field declared here"
//!
//! **Notes:** (none)
//!
//! ### E416 -- `generic_type_static_stored_property` (Error, Correctness)
//!
//! **Message:** "static stored properties not supported in generic types"
//!
//! **Labels:**
//! - Primary: the field declaration
//!   - Span source: `util::entity_span` on the field entity
//!   - Message: "static stored property in generic type '{type_name}'"
//!
//! **Notes:** (none)

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Computed, FieldMutability, NodeKind, Static, TypeParams};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E413",
        name: "computed_property_must_be_var",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E415",
        name: "enum_stored_field",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E416",
        name: "generic_type_static_stored_property",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E417",
        name: "global_property_already_static",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct FieldAnalyzer;

impl Describe for FieldAnalyzer {
    fn id(&self) -> &'static str {
        "field"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for FieldAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Field]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();
        let span = util::entity_span(cx.query, cx.entity);

        let is_static = cx.query.get::<Static>(cx.entity).is_some();

        let is_computed = cx.query.get::<Computed>(cx.entity).is_some();
        let has_var_keyword = matches!(
            cx.query.get::<FieldMutability>(cx.entity),
            Some(FieldMutability::Var)
        );

        // Check 1: computed properties must use 'var' (not 'let')
        if is_computed && !has_var_keyword {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: "computed properties must use 'var'".into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "computed property declared with 'let'".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

        // Get parent for context-dependent checks
        let Some(parent) = cx.query.parent_of(cx.entity) else {
            return diags;
        };
        let parent_kind = cx.query.get::<NodeKind>(parent);

        // Check: global-scope properties are already static
        if is_static && matches!(parent_kind, Some(NodeKind::Module)) {
            let msg = if is_computed {
                "computed properties in global context are already static"
            } else {
                "properties in global context are already static"
            };
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[3].id,
                severity: DESCRIPTORS[3].default_severity,
                message: msg.into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "'static' is redundant here".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

        // Check 2: enums cannot have non-static stored fields
        if matches!(parent_kind, Some(NodeKind::Enum)) && !is_static && !is_computed {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: "enums cannot have stored fields".into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "stored field declared here".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

        // Check 3: static stored properties not supported in generic types
        // Skip protocol fields — they're abstract declarations, not stored properties
        if is_static && !is_computed && !matches!(parent_kind, Some(NodeKind::Protocol)) {
            let parent_is_generic = cx
                .query
                .get::<TypeParams>(parent)
                .is_some_and(|tp| !tp.0.is_empty());

            if parent_is_generic {
                let type_name = util::entity_name(cx.query, parent);

                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[2].id,
                    severity: DESCRIPTORS[2].default_severity,
                    message: "static stored properties not supported in generic types".into(),
                    labels: vec![DiagLabel {
                        span,
                        message: format!("static stored property in generic type '{}'", type_name),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        diags
    }
}
