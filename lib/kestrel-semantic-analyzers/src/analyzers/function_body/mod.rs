//! Analyzer for function bodies
//!
//! Ensures that functions outside of protocols have bodies.

use kestrel_semantic_model::queries::AncestorOfKind;
use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;
use std::sync::Arc;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::FunctionMissingBodyError;

#[derive(Default)]
pub struct FunctionBodyAnalyzer;

impl FunctionBodyAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for FunctionBodyAnalyzer {
    fn name(&self) -> &'static str {
        "function_body"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Function {
            return;
        }

        // Skip functions inside protocols - they're not supposed to have bodies
        let symbol_id = symbol.metadata().id();
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

        // Get the FunctionDataBehavior
        let Some(data) = symbol.metadata().get_behavior::<FunctionDataBehavior>() else { return; };
        if data.has_body() { return; }

        let name = &symbol.metadata().name().value;
        let span = symbol.metadata().declaration_span().clone();
        ctx.report(FunctionMissingBodyError { span, function_name: name.clone() });
    }
}
