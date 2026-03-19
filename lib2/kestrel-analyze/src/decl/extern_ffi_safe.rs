//! # Extern FFI Safe Analyzer
//!
//! Validates that extern functions' parameter types and return type conform
//! to FFISafe. Extern functions are identified by the `@extern` attribute.
//!
//! ## Diagnostics
//!
//! ### KS605 -- `type_not_ffi_safe` (Error, Correctness)
//!
//! **Message:** "{context} type '{ty}' does not conform to FFISafe"
//!
//! **Labels:**
//! - Primary: the function declaration
//!   - Span source: `util::entity_span` on the function entity
//!   - Message: "type is not FFI-safe"
//!
//! **Notes:**
//! - "only types conforming to FFISafe can cross FFI boundaries"

use crate::context::DeclContext;
use crate::diagnostic::*;
use crate::traits::{DeclCheck, Describe};
use crate::util;
use kestrel_ast_builder::{Attributes, NodeKind};
use kestrel_hir::builtin::Builtin;
use kestrel_hir::ty::HirTy;
use kestrel_hir_lower::LowerCallableTypes;
use kestrel_name_res::{ConformingProtocols, ResolveBuiltin};

static DESCRIPTORS: &[DiagnosticDescriptor] = &[DiagnosticDescriptor {
    id: "KS605",
    name: "type_not_ffi_safe",
    default_severity: Severity::Error,
    category: Category::Correctness,
}];

pub struct ExternFfiSafeAnalyzer;

impl Describe for ExternFfiSafeAnalyzer {
    fn id(&self) -> &'static str {
        "extern_ffi_safe"
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
        // Only check functions with @extern attribute
        let Some(attrs) = cx.query.get::<Attributes>(cx.entity) else {
            return vec![];
        };
        if !attrs.0.iter().any(|a| a.name == "extern") {
            return vec![];
        }

        // Resolve FFISafe protocol entity
        let Some(ffi_safe_entity) = cx.query.query(ResolveBuiltin {
            builtin: Builtin::FFISafe,
            root: cx.root,
        }) else {
            return vec![];
        };

        let mut diags = Vec::new();

        // Check parameter types
        if let Some(param_types) = cx.query.query(LowerCallableTypes {
            entity: cx.entity,
            root: cx.root,
        }) {
            for (i, param_ty) in param_types.iter().enumerate() {
                let Some(hir_ty) = param_ty else { continue };
                if !self.is_ffi_safe(cx, hir_ty, ffi_safe_entity) {
                    let param_name = self.param_name(cx, i);
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: format!(
                            "parameter '{}' type does not conform to FFISafe",
                            param_name
                        ),
                        labels: vec![DiagLabel {
                            span: util::entity_span(cx.query, cx.entity),
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
        }) {
            // Skip unit return type (empty tuple)
            if !matches!(&ret_ty, HirTy::Tuple(elems, _) if elems.is_empty()) {
                if !self.is_ffi_safe(cx, &ret_ty, ffi_safe_entity) {
                    diags.push(AnalyzeDiagnostic {
                        descriptor_id: DESCRIPTORS[0].id,
                        severity: DESCRIPTORS[0].default_severity,
                        message: "return type does not conform to FFISafe".into(),
                        labels: vec![DiagLabel {
                            span: util::entity_span(cx.query, cx.entity),
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

        diags
    }
}

impl ExternFfiSafeAnalyzer {
    /// Check if a type conforms to FFISafe.
    fn is_ffi_safe(
        &self,
        cx: &DeclContext<'_>,
        hir_ty: &HirTy,
        ffi_safe_entity: kestrel_hecs::Entity,
    ) -> bool {
        let HirTy::Named { entity, .. } = hir_ty else {
            // Non-named types (tuples, functions, etc.) are not FFI-safe
            return false;
        };

        let conforming = cx.query.query(ConformingProtocols {
            entity: *entity,
            root: cx.root,
        });
        conforming.contains(&ffi_safe_entity)
    }

    /// Get the parameter name at index from the Callable component.
    fn param_name(&self, cx: &DeclContext<'_>, index: usize) -> String {
        cx.query
            .get::<kestrel_ast_builder::Callable>(cx.entity)
            .and_then(|c| c.params.get(index))
            .map(|p| p.name.clone())
            .unwrap_or_else(|| format!("#{}", index))
    }
}
