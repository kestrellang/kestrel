//! Analyzer for static modifier context
//!
//! Ensures that the `static` keyword is only used inside structs, protocols, or extensions.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_model::queries::AncestorOfKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::{StaticContext, StaticInWrongContextError};

#[derive(Default)]
pub struct StaticContextAnalyzer;

impl StaticContextAnalyzer { pub fn new() -> Self { Self } }

impl Analyzer for StaticContextAnalyzer {
    fn name(&self) -> &'static str { "static_context" }

    fn visit_symbol(&mut self, symbol: &Arc<dyn Symbol<KestrelLanguage>>, ctx: &mut AnalysisContext) {
        if symbol.metadata().kind() != KestrelSymbolKind::Function { return; }

        // Get the FunctionDataBehavior
        let Some(data) = symbol.metadata().get_behavior::<FunctionDataBehavior>() else { return; };

        // Static is only valid inside structs, protocols, or extensions
        let symbol_id = symbol.metadata().id();
        let in_struct = ctx.model.query(AncestorOfKind { symbol_id, kind: KestrelSymbolKind::Struct }).is_some();
        let in_protocol = ctx.model.query(AncestorOfKind { symbol_id, kind: KestrelSymbolKind::Protocol }).is_some();
        let in_extension = ctx.model.query(AncestorOfKind { symbol_id, kind: KestrelSymbolKind::Extension }).is_some();

        let in_valid_context = in_struct || in_protocol || in_extension;
        if !(data.is_static() && !in_valid_context) { return; }

        let name = &symbol.metadata().name().value;
        let span = symbol.metadata().declaration_span().clone();
        ctx.report(StaticInWrongContextError { span, name: name.clone(), context: StaticContext::ModuleLevel });
    }
}
