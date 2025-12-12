use kestrel_reporting::DiagnosticContext;
use crate::analyzer::Analyzer;
use crate::runner::AnalyzerId;
use kestrel_semantic_model::SemanticModel;

/// Context provided to analyzers during the walk.
pub struct AnalysisContext<'a> {
    pub model: &'a SemanticModel,
    pub diagnostics: &'a mut DiagnosticContext,
    // Internal control
    pub(crate) stopped: bool,
    pub(crate) skip_children: bool,
    // Index of current analyzer when running multiple
    pub(crate) current: AnalyzerId,
}

impl<'a> AnalysisContext<'a> {
    pub fn new(model: &'a SemanticModel, diagnostics: &'a mut DiagnosticContext) -> Self {
        Self { model, diagnostics, stopped: false, skip_children: false, current: AnalyzerId(0) }
    }

    /// Report a diagnostic
    pub fn report<D: kestrel_reporting::IntoDiagnostic>(&mut self, d: D) {
        self.diagnostics.throw(d);
    }

    /// Stop the entire walk immediately
    pub fn stop(&mut self) { self.stopped = true; }

    /// Skip children of the current node
    pub fn skip_children(&mut self) { self.skip_children = true; }
}

/// Reset per-node flags before visiting children
pub(crate) fn reset_node_flags(ctx: &mut AnalysisContext) {
    ctx.skip_children = false;
}

