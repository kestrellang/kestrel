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
//!
//! ### E466 -- `some_in_field_type` (Error, Correctness)
//!
//! **Message:** "'some' (opaque type) is not allowed in a field type"
//!
//! `some P` is only valid as a function return type (opaque) or a parameter
//! type (generic sugar). In a stored/computed field it has no return-position
//! origin body, so it cannot be reified — mir-lower would panic resolving the
//! opaque origin (#168). Reject it at the front end with a clean diagnostic.
//!
//! **Labels:**
//! - Primary: the `some` annotation
//!   - Message: "opaque types can only appear in return position"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast::AstType;
use kestrel_ast_builder::{Computed, FieldMutability, NodeKind, Static, TypeAnnotation, TypeParams};

/// Span of the first `some` (opaque) type found anywhere in `ty`, or `None`.
/// Opaque is legal only in return position; a field annotation containing it
/// (top-level or nested, e.g. `[some P]`) is rejected.
fn opaque_span(ty: &AstType) -> Option<kestrel_span::Span> {
    match ty {
        AstType::Some { span, .. } => Some(span.clone()),
        AstType::Array(inner, _) | AstType::Optional(inner, _) => opaque_span(inner),
        AstType::Dictionary(k, v, _) => opaque_span(k).or_else(|| opaque_span(v)),
        AstType::Result { ok, err, .. } => opaque_span(ok).or_else(|| opaque_span(err)),
        AstType::Tuple(elems, _) => elems.iter().find_map(opaque_span),
        AstType::Function {
            params,
            return_type,
            ..
        } => params
            .iter()
            .find_map(opaque_span)
            .or_else(|| opaque_span(return_type)),
        AstType::Ref { inner, .. } => opaque_span(inner),
        AstType::Named { .. }
        | AstType::Unit(_)
        | AstType::Never(_)
        | AstType::Inferred(_) => None,
    }
}

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
    DiagnosticDescriptor {
        id: "E466",
        name: "some_in_field_type",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct FieldAnalyzer;

impl Describe for FieldAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::Field
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

        // `some P` (opaque) is only valid in return/parameter position; in a
        // field type it has no origin body and would panic mir-lower (#168).
        if let Some(TypeAnnotation(ty)) = cx.query.get::<TypeAnnotation>(cx.entity)
            && let Some(opaque) = opaque_span(ty)
        {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[4].id,
                severity: DESCRIPTORS[4].default_severity,
                message: "'some' (opaque type) is not allowed in a field type".into(),
                labels: vec![DiagLabel {
                    span: opaque,
                    message: "opaque types can only appear in return position".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

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
