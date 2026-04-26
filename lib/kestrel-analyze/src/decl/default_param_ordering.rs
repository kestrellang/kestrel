//! # Default Parameter Ordering Analyzer
//!
//! Validates:
//! 1. Required parameters do not follow parameters with defaults.
//! 2. Default value expressions do not reference other parameters.
//!
//! ## Diagnostics
//!
//! ### E613 -- `required_param_after_default` (Error, Correctness)
//! **Message:** "required parameter '{name}' cannot follow parameter '{default_name}' which has a default value"
//!
//! ### E614 -- `default_references_param` (Error, Correctness)
//! **Message:** "default value cannot reference parameter '{name}'"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Callable, DefaultReferencesParam, NodeKind};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E613",
        name: "required_param_after_default",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E614",
        name: "default_references_param",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct DefaultParamOrderingAnalyzer;

impl Describe for DefaultParamOrderingAnalyzer {
    fn id(&self) -> &'static str {
        "default_param_ordering"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for DefaultParamOrderingAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[
            NodeKind::Function,
            NodeKind::Initializer,
            NodeKind::Subscript,
        ]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(callable) = cx.query.get::<Callable>(cx.entity) else {
            return vec![];
        };

        let span = util::entity_span(cx.query, cx.entity);
        let mut diags = Vec::new();

        // Check ordering: required params cannot follow defaulted params
        let mut first_default_name: Option<&str> = None;
        for param in &callable.params {
            if param.default_entity.is_some() {
                if first_default_name.is_none() {
                    first_default_name = Some(&param.name);
                }
            } else if let Some(default_name) = first_default_name {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: "E613",
                    severity: Severity::Error,
                    message: format!(
                        "required parameter '{}' cannot follow parameter '{}' which has a default value",
                        param.name, default_name
                    ),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "required parameter after default parameter".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
                break;
            }
        }

        // Check that default expressions don't reference sibling parameters.
        // The AST builder detects this and sets a DefaultReferencesParam marker.
        for param in &callable.params {
            let Some(default_entity) = param.default_entity else {
                continue;
            };
            if let Some(marker) = cx.query.get::<DefaultReferencesParam>(default_entity) {
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: "E614",
                    severity: Severity::Error,
                    message: format!("default value cannot reference parameter '{}'", marker.0),
                    labels: vec![DiagLabel {
                        span: span.clone(),
                        message: "default values are evaluated at each call site".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        diags
    }
}
