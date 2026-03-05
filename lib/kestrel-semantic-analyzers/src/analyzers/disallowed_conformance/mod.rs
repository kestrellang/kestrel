//! Analyzer for disallowed enum conformance validation
//!
//! Validates that enums don't conform to protocols marked with `disallow_enum_conformance`.

use std::sync::Arc;

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::builtins::BuiltinKind;
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::ty::TyKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::ProtocolDisallowsEnumConformanceError;

/// Analyzer that validates enums don't conform to protocols that disallow enum conformance.
pub struct DisallowedConformanceAnalyzer;

impl DisallowedConformanceAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DisallowedConformanceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DisallowedConformanceAnalyzer {
    fn name(&self) -> &'static str {
        "disallowed_conformance"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Enum {
            return;
        }

        let Some(conformances) = symbol.metadata().get_behavior::<ConformancesBehavior>() else {
            return;
        };

        for conformance_ty in conformances.conformances() {
            let TyKind::Protocol {
                symbol: protocol_sym,
                ..
            } = conformance_ty.kind()
            else {
                continue;
            };

            let protocol_id = protocol_sym.metadata().id();

            let Some(feature) = ctx.model.builtin_registry().protocol_feature(protocol_id) else {
                continue;
            };

            let definition = feature.definition();
            if let BuiltinKind::Protocol {
                disallow_enum_conformance: true,
                ..
            } = definition.kind
            {
                ctx.report(ProtocolDisallowsEnumConformanceError {
                    span: symbol.metadata().span().clone(),
                    enum_name: symbol.metadata().name().value.clone(),
                    protocol_name: protocol_sym.metadata().name().value.clone(),
                });
            }
        }
    }
}
