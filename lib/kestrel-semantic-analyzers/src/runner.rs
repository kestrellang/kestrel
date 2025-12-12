use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;
use crate::walker::walk_root;
use kestrel_reporting::DiagnosticContext;
use kestrel_semantic_model::SemanticModel;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnalyzerId(pub usize);

/// Run a single analyzer against the model
pub fn run<A: Analyzer>(
    analyzer: &mut A,
    model: &SemanticModel,
    diagnostics: &mut DiagnosticContext,
) {
    let mut ctx = AnalysisContext::new(model, diagnostics);
    let mut analyzers: [&mut dyn Analyzer; 1] = [analyzer];
    run_all(&mut analyzers, model, &mut ctx);
}

/// Run multiple analyzers in a single walk
pub fn run_all(
    analyzers: &mut [&mut dyn Analyzer],
    model: &SemanticModel,
    ctx: &mut AnalysisContext,
) {
    // Walk the tree, calling into analyzers
    walk_root(analyzers, model, ctx);

    // Finalize all analyzers
    for (i, analyzer) in analyzers.iter_mut().enumerate() {
        ctx.current = AnalyzerId(i);
        analyzer.finalize(ctx);
    }
}
