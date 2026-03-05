//! Analyzer for extern function FFI safety validation
//!
//! Validates that all parameter types and return types of extern functions conform to FFISafe.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::extern_fn::ExternBehavior;
use kestrel_semantic_tree::builtins::LanguageFeature;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_type_inference::TypeOracle;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::TypeNotFFISafeError;

/// Analyzer that validates extern function types conform to FFISafe.
pub struct ExternFFISafeAnalyzer;

impl ExternFFISafeAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ExternFFISafeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for ExternFFISafeAnalyzer {
    fn name(&self) -> &'static str {
        "extern_ffi_safe"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        // Only check functions with ExternBehavior
        if symbol
            .metadata()
            .get_behavior::<ExternBehavior>()
            .is_none()
        {
            return;
        }

        let Some(ffi_safe_id) = ctx.model.builtin_registry().protocol(LanguageFeature::FFISafe)
        else {
            return;
        };

        let Some(callable) = symbol.metadata().get_behavior::<CallableBehavior>() else {
            return;
        };

        // Check each parameter type
        for param in callable.parameters() {
            if !ctx.model.conforms_to(&param.ty, ffi_safe_id) {
                ctx.report(TypeNotFFISafeError {
                    span: param.ty.span().clone(),
                    ty: param.ty.to_string(),
                    context: "parameter".to_string(),
                });
            }
        }

        // Check return type (skip if Unit - void is always valid for extern)
        let return_ty = callable.return_type();
        if !return_ty.is_unit() && !ctx.model.conforms_to(return_ty, ffi_safe_id) {
            ctx.report(TypeNotFFISafeError {
                span: return_ty.span().clone(),
                ty: return_ty.to_string(),
                context: "return type".to_string(),
            });
        }
    }
}
