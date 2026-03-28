//! # Extension Validation Analyzer
//!
//! Validates extension declarations:
//!
//! 1. **Invalid extension target** — Extensions can only target structs, enums,
//!    and protocols. Primitives, type aliases, and unknown types are rejected.
//! 2. **Wrong type parameter count** — The number of type arguments on the
//!    extension target must match the target type's declared type parameters.
//!
//! ## Diagnostics
//!
//! ### E452 -- `invalid_extension_target` (Error, Correctness)
//!
//! **Message:** "cannot extend '{name}'"
//!
//! ### E453 -- `extension_type_param_count` (Error, Correctness)
//!
//! **Message:** "wrong number of type parameters for '{name}': expected {expected}, got {got}"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{ExtensionTarget, Intrinsic, NodeKind, TypeParams};
use kestrel_name_res::{ResolveTypePath, TypeResolution};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E452",
        name: "invalid_extension_target",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E453",
        name: "extension_type_param_count",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ExtensionValidationAnalyzer;

impl Describe for ExtensionValidationAnalyzer {
    fn id(&self) -> &'static str {
        "extension_validation"
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ExtensionValidationAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Extension]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let mut diags = Vec::new();

        let Some(target) = cx.query.get::<ExtensionTarget>(cx.entity) else {
            return diags;
        };

        // Extract the target type name and type args from the AstType
        let kestrel_ast::AstType::Named { segments, .. } = &target.0 else {
            return diags;
        };

        let seg_names: Vec<String> = segments.iter().map(|s| s.name.clone()).collect();
        let target_name = seg_names.join(".");
        let span = util::entity_span(cx.query, cx.entity);

        // Resolve the target type
        let context = cx.query.parent_of(cx.entity).unwrap_or(cx.root);
        let resolution = cx.query.query(ResolveTypePath {
            segments: seg_names,
            context,
            root: cx.root,
        });

        match resolution {
            TypeResolution::Found(entity) => {
                // Check 1: target must be a struct, enum, or protocol (not intrinsic)
                let kind = cx.query.get::<NodeKind>(entity);
                let is_intrinsic = cx.query.has::<Intrinsic>(entity);
                if is_intrinsic || !matches!(kind, Some(NodeKind::Struct | NodeKind::Enum | NodeKind::Protocol)) {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!("cannot extend '{}'", target_name),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: "only structs, enums, and protocols can be extended".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                    return diags;
                }

                // Check 2: type parameter count must match
                let expected_count = cx.query.get::<TypeParams>(entity)
                    .map(|tp| tp.0.len())
                    .unwrap_or(0);
                let got_count = segments.last()
                    .map(|s| s.type_args.len())
                    .unwrap_or(0);

                // Only check if type args were explicitly provided (got > 0)
                if got_count > 0 && got_count != expected_count {
                    let target_name = util::entity_name(cx.query, entity);
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[1].id,
                        severity: DESCRIPTORS[1].default_severity,
                        message: format!(
                            "wrong number of type parameters for '{}': expected {}, got {}",
                            target_name, expected_count, got_count,
                        ),
                        labels: vec![DiagLabel {
                            span,
                            message: format!("expected {} type parameter{}", expected_count, if expected_count == 1 { "" } else { "s" }),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
            _ => {
                // Target type not found
                diags.push(AnalyzeDiagnostic {
                    descriptor_id: DESCRIPTORS[0].id,
                    severity: DESCRIPTORS[0].default_severity,
                    message: format!("cannot extend '{}': unknown type", target_name),
                    labels: vec![DiagLabel {
                        span,
                        message: "type not found".into(),
                        is_primary: true,
                    }],
                    notes: vec![],
                });
            }
        }

        diags
    }
}
