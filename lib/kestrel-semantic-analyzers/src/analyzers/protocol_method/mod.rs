//! Analyzer for protocol methods
//!
//! Ensures that methods declared inside protocols do NOT have bodies.

use std::sync::Arc;

use kestrel_semantic_model::{FunctionsInSymbol, HasBody};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::ProtocolMethodHasBodyError;

#[derive(Default)]
pub struct ProtocolMethodAnalyzer;

impl ProtocolMethodAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Analyzer for ProtocolMethodAnalyzer {
    fn name(&self) -> &'static str {
        "protocol_method"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        // Only check protocols
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return;
        }

        let protocol_name = symbol.metadata().name().value.clone();

        let protocol_id = symbol.metadata().id();
        for method in ctx.model.query(FunctionsInSymbol {
            parent_id: protocol_id,
        }) {
            let method_id = method.metadata().id();
            let Some(true) = ctx.model.query(HasBody {
                function_id: method_id,
            }) else {
                continue;
            };

            let name = &method.metadata().name().value;
            let span = method.metadata().declaration_span().clone();
            ctx.report(ProtocolMethodHasBodyError {
                span,
                method_name: name.clone(),
                protocol_name: protocol_name.clone(),
            });
        }
    }
}
