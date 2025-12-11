//! Validator for function bodies
//!
//! Ensures that functions outside of protocols have bodies.

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

use crate::diagnostics::FunctionMissingBodyError;
use crate::validation::{SymbolContext, Validator};

/// Validator that ensures functions have bodies (except in protocols)
pub struct FunctionBodyValidator;

impl FunctionBodyValidator {
    const NAME: &'static str = "function_body";

    pub fn new() -> Self {
        Self
    }
}

impl Default for FunctionBodyValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for FunctionBodyValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check functions
        if kind != KestrelSymbolKind::Function {
            return;
        }

        // Skip functions inside protocols - they're not supposed to have bodies
        if ctx.in_protocol {
            return;
        }

        // Get the FunctionDataBehavior
        let behaviors = ctx.symbol.metadata().behaviors();
        let function_data = behaviors.iter().find_map(|b| {
            if matches!(b.kind(), KestrelBehaviorKind::FunctionData) {
                b.as_ref().downcast_ref::<FunctionDataBehavior>()
            } else {
                None
            }
        });

        if let Some(data) = function_data {
            if !data.has_body() {
                let name = &ctx.symbol.metadata().name().value;
                let span = ctx.symbol.metadata().declaration_span().clone();

                ctx.diagnostics().get().throw(
                    FunctionMissingBodyError {
                        span,
                        function_name: name.clone(),
                    });
            }
        }
    }
}
