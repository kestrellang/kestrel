//! Analyzer for builtin marker protocol validation
//!
//! Validates that builtin protocols marked with `must_be_marker` have no required members.

use std::sync::Arc;

use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::BuiltinMustBeMarkerError;

/// Analyzer that validates builtin protocols with `must_be_marker` have no required members.
pub struct BuiltinMarkerProtocolAnalyzer;

impl BuiltinMarkerProtocolAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BuiltinMarkerProtocolAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for BuiltinMarkerProtocolAnalyzer {
    fn name(&self) -> &'static str {
        "builtin_marker_protocol"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Protocol {
            return;
        }

        let protocol_id = symbol.metadata().id();

        // Check if this protocol is registered as a builtin with must_be_marker
        let Some(feature) = ctx.model.builtin_registry().protocol_feature(protocol_id) else {
            return;
        };

        let definition = feature.definition();
        let BuiltinKind::Protocol {
            must_be_marker: true,
            ..
        } = definition.kind
        else {
            return;
        };

        // Check if the protocol is actually a marker protocol
        if !ctx
            .model
            .query(kestrel_semantic_model::IsMarkerProtocol { protocol_id })
        {
            ctx.report(BuiltinMustBeMarkerError {
                span: symbol.metadata().span().clone(),
                feature_name: feature.name().to_string(),
            });
        }
    }
}
