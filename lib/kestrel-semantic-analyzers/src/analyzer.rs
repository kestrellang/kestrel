use std::sync::Arc;

use kestrel_semantic_tree::expr::Expression;
use kestrel_semantic_tree::pattern::Pattern;
use kestrel_semantic_tree::stmt::Statement;
use kestrel_semantic_tree::ty::Ty;
use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::Symbol;

use crate::context::AnalysisContext;

/// Trait implemented by all semantic analyzers.
///
/// Analyzers run over an immutable semantic model and may emit diagnostics.
pub trait Analyzer {
    /// Unique identifier for this analyzer
    fn name(&self) -> &'static str;

    // Pre-visit hooks (called before children)
    fn visit_symbol(&mut self, _symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {}
    fn visit_statement(&mut self, _stmt: &Statement, _ctx: &mut AnalysisContext) {}
    fn visit_expression(&mut self, _expr: &Expression, _ctx: &mut AnalysisContext) {}
    fn visit_type(&mut self, _ty: &Ty, _ctx: &mut AnalysisContext) {}
    fn visit_pattern(&mut self, _pattern: &Pattern, _ctx: &mut AnalysisContext) {}

    // Post-visit hooks (called after children)
    fn visit_symbol_post(&mut self, _symbol: &Arc<dyn Symbol<KestrelLanguage>>, _ctx: &mut AnalysisContext) {}
    fn visit_statement_post(&mut self, _stmt: &Statement, _ctx: &mut AnalysisContext) {}
    fn visit_expression_post(&mut self, _expr: &Expression, _ctx: &mut AnalysisContext) {}
    fn visit_type_post(&mut self, _ty: &Ty, _ctx: &mut AnalysisContext) {}
    fn visit_pattern_post(&mut self, _pattern: &Pattern, _ctx: &mut AnalysisContext) {}

    /// Called after the entire walk completes (even if stopped early)
    fn finalize(&mut self, _ctx: &mut AnalysisContext) {}
}

