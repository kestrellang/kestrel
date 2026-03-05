//! Analyzer for duplicate deinit declarations
//!
//! Ensures a struct has at most one deinit block.

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;
use diagnostics::DuplicateDeinitError;

/// Analyzer that ensures no struct has more than one deinit declaration.
pub struct DuplicateDeinitAnalyzer;

impl DuplicateDeinitAnalyzer {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DuplicateDeinitAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DuplicateDeinitAnalyzer {
    fn name(&self) -> &'static str {
        "duplicate_deinit"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        if symbol.metadata().kind() != KestrelSymbolKind::Struct {
            return;
        }

        let deinits: Vec<_> = symbol
            .metadata()
            .children()
            .into_iter()
            .filter(|c| c.metadata().kind() == KestrelSymbolKind::Deinit)
            .collect();

        if deinits.len() > 1 {
            ctx.report(DuplicateDeinitError {
                first_span: deinits[0].metadata().span().clone(),
                duplicate_span: deinits[1].metadata().span().clone(),
                struct_name: symbol.metadata().name().value.clone(),
            });
        }
    }
}
