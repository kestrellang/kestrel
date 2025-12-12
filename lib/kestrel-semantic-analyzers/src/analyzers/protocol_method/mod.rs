//! Analyzer for protocol methods
//!
//! Ensures that methods declared inside protocols do NOT have bodies.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::ProtocolMethodHasBodyError;

#[derive(Default)]
pub struct ProtocolMethodAnalyzer;

impl ProtocolMethodAnalyzer { pub fn new() -> Self { Self } }

impl Analyzer for ProtocolMethodAnalyzer {
    fn name(&self) -> &'static str { "protocol_method" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
        // Only check protocols
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol { return; }

        let protocol_name = symbol.metadata().name().value.clone();

        // Check all function children
        for child in symbol.metadata().children() {
            if child.metadata().kind() != KestrelSymbolKind::Function { continue; }

            let Some(data) = child.metadata().get_behavior::<FunctionDataBehavior>() else { continue; };
            if !data.has_body() { continue; }

            let name = &child.metadata().name().value;
            let span = child.metadata().declaration_span().clone();
            ctx.report(ProtocolMethodHasBodyError { span, method_name: name.clone(), protocol_name: protocol_name.clone() });
        }
    }
}
