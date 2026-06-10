//! # Extern FFI Safe Analyzer
//!
//! Validates extern function constraints and FFISafe conformance:
//! - Parameter and return types must conform to FFISafe
//! - Cannot be generic (no stable ABI for generics)
//! - Cannot have a body (implemented externally)
//! - Parameters cannot use mutating access mode (must be consuming)
//! - Must specify a calling convention (e.g. @extern(.C))
//!
//! ## Diagnostics
//!
//! - E605: type does not conform to FFISafe
//! - E609: extern function cannot be generic
//! - E610: extern function cannot have a body
//! - E611: extern parameter must use consuming access mode
//! - E612: @extern requires a calling convention

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{AnalyzerId, DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Attributes, Body, Callable, Intrinsic, NodeKind, TypeParams};
use kestrel_hir::builtin::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerCallableTypes;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[
    DiagnosticDescriptor {
        id: "E605",
        name: "type_not_ffi_safe",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E609",
        name: "extern_cannot_be_generic",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E610",
        name: "extern_cannot_have_body",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E611",
        name: "extern_param_not_consuming",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
    DiagnosticDescriptor {
        id: "E612",
        name: "extern_requires_calling_convention",
        default_severity: Severity::Error,
        category: Category::Correctness,
    },
];

pub struct ExternFfiSafeAnalyzer;

impl Describe for ExternFfiSafeAnalyzer {
    fn id(&self) -> AnalyzerId {
        AnalyzerId::ExternFfiSafe
    }
    fn descriptors(&self) -> &'static [DiagnosticDescriptor] {
        DESCRIPTORS
    }
}

impl DeclCheck for ExternFfiSafeAnalyzer {
    fn target_kinds(&self) -> &'static [NodeKind] {
        &[NodeKind::Function]
    }

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic> {
        let Some(attrs) = cx.query.get::<Attributes>(cx.entity) else {
            return vec![];
        };
        let Some(extern_attr) = attrs.0.iter().find(|a| a.name == "extern") else {
            return vec![];
        };

        let span = util::entity_span(cx.query, cx.entity);
        let mut diags = Vec::new();

        // @extern requires a calling convention argument like @extern(.C)
        if extern_attr.args.is_empty() {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E612",
                severity: Severity::Error,
                message: "@extern requires a calling convention".into(),
                labels: vec![DiagLabel {
                    span: span.clone(),
                    message: "missing calling convention (e.g. @extern(.C))".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
        }

        // Extern functions cannot be generic — if generic, skip FFISafe checks
        // since type params can't be checked for conformance
        let is_generic = cx
            .query
            .get::<TypeParams>(cx.entity)
            .is_some_and(|tp| !tp.0.is_empty());
        if is_generic {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E609",
                severity: Severity::Error,
                message: "@extern functions cannot be generic".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "generic functions cannot have a stable ABI".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

        // Extern functions cannot have a body — skip FFISafe checks if so,
        // since the function is already structurally invalid
        if cx.query.get::<Body>(cx.entity).is_some() {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E610",
                severity: Severity::Error,
                message: "@extern functions cannot have a body".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "extern functions are implemented in external code".into(),
                    is_primary: true,
                }],
                notes: vec![],
            });
            return diags;
        }

        // Extern parameters cannot use mutating access mode (must be consuming)
        if let Some(callable) = cx.query.get::<Callable>(cx.entity) {
            for param in &callable.params {
                if param.is_mut && !param.is_consuming {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: "E611",
                        severity: Severity::Error,
                        message: format!(
                            "@extern function parameter '{}' must use consuming access mode",
                            param.name
                        ),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: "extern functions receive values, not references".into(),
                            is_primary: true,
                        }],
                        notes: vec![],
                    });
                }
            }
        }

        // Check parameter types conform to FFISafe
        let Some(ffi_safe_entity) = cx.query.query(ResolveBuiltin {
            builtin: Builtin::FFISafe,
            root: cx.root,
        }) else {
            return diags;
        };

        if let Some(param_types) = cx.query.query(LowerCallableTypes {
            entity: cx.entity,
            root: cx.root,
        }) {
            for (i, param_ty) in param_types.iter().enumerate() {
                let Some(hir_ty) = param_ty else { continue };
                if !is_ffi_safe(cx, hir_ty, ffi_safe_entity) {
                    let param_name = self.param_name(cx, i);
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: "E605",
                        severity: Severity::Error,
                        message: format!(
                            "parameter '{}' type does not conform to FFISafe",
                            param_name
                        ),
                        labels: vec![DiagLabel {
                            span: span.clone(),
                            message: "type is not FFI-safe".into(),
                            is_primary: true,
                        }],
                        notes: vec![
                            "only types conforming to FFISafe can cross FFI boundaries".into(),
                        ],
                    });
                }
            }
        }

        // Check return type (unit/void is always valid for extern)
        if let Some(ret_ty) = cx.query.query(kestrel_hir_lower::LowerTypeAnnotation {
            entity: cx.entity,
            root: cx.root,
        }) && !matches!(&ret_ty, HirTy::Tuple(elems, _) if elems.is_empty())
            && !is_ffi_safe(cx, &ret_ty, ffi_safe_entity)
        {
            diags.push(AnalyzeDiagnostic {
                descriptor_id: "E605",
                severity: Severity::Error,
                message: "return type does not conform to FFISafe".into(),
                labels: vec![DiagLabel {
                    span,
                    message: "type is not FFI-safe".into(),
                    is_primary: true,
                }],
                notes: vec!["only types conforming to FFISafe can cross FFI boundaries".into()],
            });
        }

        diags
    }
}

impl ExternFfiSafeAnalyzer {
    fn param_name(&self, cx: &DeclContext<'_>, index: usize) -> String {
        cx.query
            .get::<Callable>(cx.entity)
            .and_then(|c| c.params.get(index))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("#{}", index))
    }
}

/// Check if a type conforms to FFISafe.
pub fn is_ffi_safe(
    cx: &DeclContext<'_>,
    hir_ty: &HirTy,
    ffi_safe_entity: kestrel_hecs::Entity,
) -> bool {
    match hir_ty {
        // Tuples are FFI-safe if all elements are FFI-safe
        HirTy::Tuple(elems, _) => elems.iter().all(|e| is_ffi_safe(cx, e, ffi_safe_entity)),
        // Nominal types: check intrinsic status or protocol conformance
        HirTy::Struct { entity, .. }
        | HirTy::Enum { entity, .. }
        | HirTy::Protocol { entity, .. } => {
            cx.query.has::<Intrinsic>(*entity)
                || cx
                    .query
                    .query(ConformingProtocols {
                        entity: *entity,
                        root: cx.root,
                    })
                    .contains(&ffi_safe_entity)
        },
        // Functions, type params, unresolved alias uses, projections — not FFI-safe
        _ => false,
    }
}
