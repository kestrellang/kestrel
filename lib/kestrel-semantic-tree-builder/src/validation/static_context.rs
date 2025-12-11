//! Validator for static modifier context
//!
//! Ensures that the `static` keyword is only used inside structs or protocols.

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

use crate::diagnostics::{StaticContext, StaticInWrongContextError};
use crate::validation::{SymbolContext, Validator};

/// Validator that ensures static modifier is only used in valid contexts
pub struct StaticContextValidator;

impl StaticContextValidator {
    const NAME: &'static str = "static_context";

    pub fn new() -> Self {
        Self
    }
}

impl Default for StaticContextValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for StaticContextValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check functions
        if kind != KestrelSymbolKind::Function {
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
            // Static is only valid inside structs, protocols, or extensions
            let in_valid_context = ctx.in_struct || ctx.in_protocol || ctx.in_extension;

            if data.is_static() && !in_valid_context {
                let name = &ctx.symbol.metadata().name().value;
                let span = ctx.symbol.metadata().declaration_span().clone();

                ctx.diagnostics().get().throw(
                    StaticInWrongContextError {
                        span,
                        name: name.clone(),
                        context: StaticContext::ModuleLevel,
                    },
                    ctx.file_id,
                );
            }
        }
    }
}
