pub mod analyzer;
pub mod context;
mod runner;
mod walker;

pub mod analyzers;

pub use analyzer::Analyzer;
pub use context::AnalysisContext;
pub use runner::{run, run_all};

/// Construct the default set of analyzers in standard order.
///
/// This list grows as validators are migrated from the builder.
pub fn default_analyzers() -> Vec<Box<dyn Analyzer>> {
    use analyzers::{
        AssignmentValidationAnalyzer, ConformanceAnalyzer, ConstraintCycleAnalyzer,
        DeadCodeAnalyzer, DuplicateSymbolAnalyzer, ExhaustiveReturnAnalyzer,
        ExtensionConflictAnalyzer, FunctionBodyAnalyzer, GenericsAnalyzer, ImportAnalyzer,
        InitializerVerificationAnalyzer, ProtocolMethodAnalyzer, StaticContextAnalyzer,
        StructCycleAnalyzer, TypeAliasCycleAnalyzer, TypeCheckAnalyzer,
        VisibilityConsistencyAnalyzer,
    };

    // Match historical order from builder ValidationRunner where possible
    vec![
        Box::new(TypeAliasCycleAnalyzer::new()),
        Box::new(StructCycleAnalyzer::new()),
        Box::new(ConstraintCycleAnalyzer::new()),
        Box::new(ConformanceAnalyzer::new()),
        Box::new(ExtensionConflictAnalyzer::new()),
        Box::new(InitializerVerificationAnalyzer::new()),
        Box::new(AssignmentValidationAnalyzer::new()),
        Box::new(DeadCodeAnalyzer::new()),
        Box::new(ExhaustiveReturnAnalyzer::new()),
        Box::new(TypeCheckAnalyzer::new()),
        Box::new(FunctionBodyAnalyzer::new()),
        Box::new(ProtocolMethodAnalyzer::new()),
        Box::new(StaticContextAnalyzer::new()),
        Box::new(DuplicateSymbolAnalyzer::new()),
        Box::new(VisibilityConsistencyAnalyzer::new()),
        Box::new(GenericsAnalyzer::new()),
        Box::new(ImportAnalyzer::new()),
    ]
}
