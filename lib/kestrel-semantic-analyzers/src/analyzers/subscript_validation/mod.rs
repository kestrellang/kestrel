//! Analyzer for subscript declarations.
//!
//! This analyzer validates that subscripts are well-formed:
//! - Subscripts must have at least one parameter
//! - Subscripts must have a body (unless they are protocol requirements)
//!
//! Note: Parent type validation (subscripts only in struct/enum/protocol/extension)
//! is already handled by the builder.

use std::sync::Arc;

use kestrel_semantic_model::queries::AncestorOfKind;
use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::subscript::SubscriptSymbol;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{SubscriptMissingBodyError, SubscriptMissingParametersError};

/// Analyzer that validates subscript declarations.
///
/// Checks:
/// 1. Subscripts must have at least one parameter
/// 2. Subscripts must have a body (unless protocol requirement)
#[derive(Default)]
pub struct SubscriptValidationAnalyzer;

impl SubscriptValidationAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for SubscriptValidationAnalyzer {
    fn name(&self) -> &'static str {
        "subscript_validation"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only process subscript symbols
        if symbol.metadata().kind() != KestrelSymbolKind::Subscript {
            return;
        }

        // Downcast to SubscriptSymbol
        let Some(subscript) = symbol.as_ref().downcast_ref::<SubscriptSymbol>() else {
            return;
        };

        let symbol_id = symbol.metadata().id();
        let span = symbol.metadata().declaration_span().clone();

        // Check 1: Subscript must have at least one parameter
        // Get parameters from the getter's CallableBehavior
        if let Some(getter) = subscript.getter() {
            if let Some(callable) = getter.metadata().get_behavior::<CallableBehavior>() {
                if callable.parameters().is_empty() {
                    ctx.report(SubscriptMissingParametersError { span: span.clone() });
                }
            }
        }

        // Check 2: Subscript must have a body (unless protocol requirement)
        // Skip if inside a protocol - protocol subscripts don't need bodies
        if ctx
            .model
            .query(AncestorOfKind {
                symbol_id,
                kind: KestrelSymbolKind::Protocol,
            })
            .is_some()
        {
            return;
        }

        // For non-protocol subscripts, the getter must have an ExecutableBehavior
        if let Some(getter) = subscript.getter() {
            let has_body = getter
                .metadata()
                .get_behavior::<ExecutableBehavior>()
                .is_some();

            if !has_body {
                ctx.report(SubscriptMissingBodyError { span });
            }
        } else {
            // No getter at all - this shouldn't happen but report if it does
            ctx.report(SubscriptMissingBodyError { span });
        }
    }
}
