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
        AssignmentValidationAnalyzer, BuiltinMarkerProtocolAnalyzer, CloneableFieldAnalyzer,
        ClosureAnalyzer, ConformanceAnalyzer, ConstraintCycleAnalyzer, DeadCodeAnalyzer,
        DefiniteAssignmentAnalyzer, DisallowedConformanceAnalyzer, DuplicateCallableAnalyzer,
        DuplicateCaseAnalyzer, DuplicateDeinitAnalyzer, DuplicateLabelAnalyzer,
        DuplicateSymbolAnalyzer, ExhaustiveReturnAnalyzer, ExhaustivenessAnalyzer,
        ExternFFISafeAnalyzer, ExtensionConflictAnalyzer, FieldAnalyzer, ForLoopPatternAnalyzer,
        FunctionBodyAnalyzer, GenericsAnalyzer, GuardLetDivergenceAnalyzer, ImportAnalyzer,
        InitializerVerificationAnalyzer, IrrefutablePatternAnalyzer,
        ParentProtocolConformanceAnalyzer, ProtocolFieldConformanceAnalyzer,
        ProtocolMethodAnalyzer, RecursiveEnumAnalyzer, RefutablePatternAnalyzer,
        StaticContextAnalyzer, StructCycleAnalyzer, SubscriptValidationAnalyzer,
        TypeAliasCycleAnalyzer, TypeCheckAnalyzer, TypeInferenceAnalyzer,
        VisibilityConsistencyAnalyzer,
    };

    // Match historical order from builder ValidationRunner where possible
    vec![
        Box::new(TypeAliasCycleAnalyzer::new()),
        Box::new(StructCycleAnalyzer::new()),
        Box::new(ConstraintCycleAnalyzer::new()),
        Box::new(ConformanceAnalyzer::new()),
        Box::new(ExtensionConflictAnalyzer::new()),
        Box::new(FieldAnalyzer::new()),
        Box::new(InitializerVerificationAnalyzer::new()),
        Box::new(AssignmentValidationAnalyzer::new()),
        Box::new(DefiniteAssignmentAnalyzer::new()),
        Box::new(DeadCodeAnalyzer::new()),
        Box::new(ExhaustiveReturnAnalyzer::new()),
        Box::new(GuardLetDivergenceAnalyzer::new()),
        // Closure analyzer runs before type inference to see original closure structure
        Box::new(ClosureAnalyzer::new()),
        // Type inference runs before type checking to resolve inference placeholders
        Box::new(TypeInferenceAnalyzer::new()),
        // Pattern analyzers run after type inference so enum types are resolved
        Box::new(RefutablePatternAnalyzer::new()),
        Box::new(ForLoopPatternAnalyzer::new()),
        Box::new(IrrefutablePatternAnalyzer::new()),
        Box::new(ExhaustivenessAnalyzer::new()),
        Box::new(TypeCheckAnalyzer::new()),
        Box::new(FunctionBodyAnalyzer::new()),
        Box::new(SubscriptValidationAnalyzer::new()),
        Box::new(ProtocolMethodAnalyzer::new()),
        Box::new(StaticContextAnalyzer::new()),
        Box::new(BuiltinMarkerProtocolAnalyzer::new()),
        Box::new(ExternFFISafeAnalyzer::new()),
        Box::new(ProtocolFieldConformanceAnalyzer::new()),
        Box::new(CloneableFieldAnalyzer::new()),
        Box::new(ParentProtocolConformanceAnalyzer::new()),
        Box::new(DisallowedConformanceAnalyzer::new()),
        Box::new(DuplicateDeinitAnalyzer::new()),
        Box::new(DuplicateSymbolAnalyzer::new()),
        Box::new(DuplicateCallableAnalyzer::new()),
        Box::new(DuplicateCaseAnalyzer::new()),
        Box::new(DuplicateLabelAnalyzer::new()),
        Box::new(RecursiveEnumAnalyzer::new()),
        Box::new(VisibilityConsistencyAnalyzer::new()),
        Box::new(GenericsAnalyzer::new()),
        Box::new(ImportAnalyzer::new()),
    ]
}
