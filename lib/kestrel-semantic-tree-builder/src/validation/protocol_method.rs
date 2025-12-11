//! Validator for protocol methods
//!
//! Ensures that methods declared inside protocols do NOT have bodies.

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::behavior::KestrelBehaviorKind;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;

use crate::diagnostics::ProtocolMethodHasBodyError;
use crate::validation::{SymbolContext, Validator};

/// Validator that ensures protocol methods don't have bodies
pub struct ProtocolMethodValidator;

impl ProtocolMethodValidator {
    const NAME: &'static str = "protocol_method";

    pub fn new() -> Self {
        Self
    }
}

impl Default for ProtocolMethodValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl Validator for ProtocolMethodValidator {
    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn validate_symbol(&self, ctx: &SymbolContext<'_>) {
        let kind = ctx.symbol.metadata().kind();

        // Only check protocols
        if kind != KestrelSymbolKind::Protocol {
            return;
        }

        let protocol_name = ctx.symbol.metadata().name().value.clone();

        // Check all function children
        for child in ctx.symbol.metadata().children() {
            if child.metadata().kind() != KestrelSymbolKind::Function {
                continue;
            }

            // Get the FunctionDataBehavior
            let behaviors = child.metadata().behaviors();
            let function_data = behaviors.iter().find_map(|b| {
                if matches!(b.kind(), KestrelBehaviorKind::FunctionData) {
                    b.as_ref().downcast_ref::<FunctionDataBehavior>()
                } else {
                    None
                }
            });

            if let Some(data) = function_data {
                if data.has_body() {
                    let name = &child.metadata().name().value;
                    let span = child.metadata().declaration_span().clone();

                    ctx.diagnostics().get().throw(
                        ProtocolMethodHasBodyError {
                            span,
                            method_name: name.clone(),
                            protocol_name: protocol_name.clone(),
                        });
                }
            }
        }
    }
}
