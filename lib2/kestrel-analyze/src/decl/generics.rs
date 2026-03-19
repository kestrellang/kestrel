//! # Generics Analyzer
//!
//! Validates generic type parameter declarations and where clause bounds:
//!
//! 1. **Duplicate type parameter names** — Two parameters with the same name
//!    within a single generic declaration.
//! 2. **Default ordering** — Parameters with defaults must come after those
//!    without. A non-default parameter after a defaulted one is an error.
//! 3. **Where clause bounds must be protocols** — Not yet checkable because
//!    bound types are unresolved AstTypes. Shell for now.
//!
//! ## Diagnostics
//!
//! ### KS434 -- `duplicate_type_parameter` (Error, Correctness)
//!
//! **Message:** "duplicate type parameter name '{name}'"
//!
//! **Labels:**
//! - Primary: the duplicate type parameter
//!   - Span source: `util::entity_span` on the second TypeParameter child entity
//!   - Message: "duplicate type parameter"
//! - Secondary: the first type parameter with the same name
//!   - Span source: `util::entity_span` on the first TypeParameter child entity
//!   - Message: "first defined here"
//!
//! **Notes:** (none)
//!
//! ### KS435 -- `type_parameter_default_ordering` (Error, Correctness)
//!
//! **Message:** "type parameter '{without}' without default follows '{with_default}' which has a default"
//!
//! **Labels:**
//! - Primary: the non-default parameter after a defaulted one
//!   - Span source: `util::entity_span` on the non-default TypeParameter entity
//!   - Message: "parameter without default"
//! - Secondary: the first parameter with a default
//!   - Span source: `util::entity_span` on the defaulted TypeParameter entity
//!   - Message: "parameter with default"
//!
//! **Notes:**
//! - "type parameters with defaults must come after those without"
//!
//! ### KS436 -- `non_protocol_bound` (Error, Correctness)
//!
//! **Message:** "bound '{type_name}' is a {type_kind}, not a protocol"
//!
//! **Labels:**
//! - Primary: the where clause bound
//!   - Span source: the bound's syntax node span
//!   - Message: "expected a protocol"
//!
//! **Notes:**
//! - "only protocols can be used as type bounds in where clauses"
//!
//! ### KS437 -- `undeclared_type_parameter_in_where` (Error, Correctness)
//!
//! **Message:** "undeclared type parameter '{name}' in where clause"
//!
//! **Labels:**
//! - Primary: the undeclared type parameter reference
//!   - Span source: the subject's syntax node span
//!   - Message: "not a declared type parameter"
//!
//! **Notes:**
//! - "available type parameters: {list}"

use std::collections::HashMap;

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{NodeKind, TypeAnnotation, TypeParams};
use kestrel_span2::Span;

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "KS434",
        name: "duplicate_type_parameter",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS435",
        name: "type_parameter_default_ordering",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS436",
        name: "non_protocol_bound",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "KS437",
        name: "undeclared_type_parameter_in_where",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct GenericsAnalyzer;

impl Describe for GenericsAnalyzer {
    fn id(&self) -> &'static str {
        "generics"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for GenericsAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Function,
            NodeKind::Struct,
            NodeKind::Enum,
            NodeKind::Protocol,
            NodeKind::TypeAlias,
            NodeKind::Initializer,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(type_params) = cx.query.get::<TypeParams>(cx.entity) else {
            return vec![];
        };
        if type_params.0.is_empty() {
            return vec![];
        }

        let mut diags = Vec::new();

        check_duplicate_type_params(cx, &type_params.0, &mut diags);
        check_default_ordering(cx, &type_params.0, &mut diags);

        // TODO: Where clause bound validation (KS436, KS437) requires resolved types.
        // The WhereClause component contains AstType entries that need name resolution
        // to determine whether a bound is a protocol, struct, or type parameter.
        // Shell for now — those checks will be added once type resolution is available.

        diags
    }
}

/// Check for duplicate type parameter names (KS434).
fn check_duplicate_type_params(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    let mut seen: HashMap<String, Span> = HashMap::new();

    for &param_entity in params {
        let name = util::entity_name(cx.query, param_entity);
        let span = util::entity_span(cx.query, param_entity);

        if let Some(first_span) = seen.get(&name) {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[0].id,
                severity: DESCRIPTORS[0].default_severity,
                message: format!("duplicate type parameter name '{}'", name),
                labels: vec![
                    DiagLabel {
                        span,
                        message: "duplicate type parameter".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: first_span.clone(),
                        message: "first defined here".into(),
                        is_primary: false,
                    },
                ],
                notes: vec![],
            });
        } else {
            seen.insert(name, span);
        }
    }
}

/// Check that type parameters with defaults come after those without (KS435).
/// A TypeParameter has a default if it has a TypeAnnotation component.
fn check_default_ordering(
    cx: &DeclContext<'_>,
    params: &[kestrel_hecs::Entity],
    diags: &mut Vec<AnalyzeDiagnostic>,
) {
    // Track the first parameter that has a default
    let mut first_with_default: Option<(String, Span)> = None;

    for &param_entity in params {
        let has_default = cx.query.get::<TypeAnnotation>(param_entity).is_some();

        if has_default {
            if first_with_default.is_none() {
                let name = util::entity_name(cx.query, param_entity);
                let span = util::entity_span(cx.query, param_entity);
                first_with_default = Some((name, span));
            }
        } else if let Some((ref default_name, ref default_span)) = first_with_default {
            // Non-default parameter after one with a default — error
            let name = util::entity_name(cx.query, param_entity);
            let span = util::entity_span(cx.query, param_entity);
            diags.push(AnalyzeDiagnostic {
                descriptor_id: DESCRIPTORS[1].id,
                severity: DESCRIPTORS[1].default_severity,
                message: format!(
                    "type parameter '{}' without default follows '{}' which has a default",
                    name, default_name
                ),
                labels: vec![
                    DiagLabel {
                        span,
                        message: "parameter without default".into(),
                        is_primary: true,
                    },
                    DiagLabel {
                        span: default_span.clone(),
                        message: "parameter with default".into(),
                        is_primary: false,
                    },
                ],
                notes: vec!["type parameters with defaults must come after those without".into()],
            });
            // One diagnostic is enough — stop checking
            break;
        }
    }
}
