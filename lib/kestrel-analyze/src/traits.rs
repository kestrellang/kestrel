//! Analyzer traits — one per analysis granularity.
//!
//! Inspired by Roslyn's action registration model, expressed as Rust traits.
//! Analyzers are stateless ZSTs that implement the relevant trait(s).

use kestrel_ast_builder::NodeKind;

use crate::context::{BodyContext, CompilationContext, DeclContext};
use crate::diagnostic::{AnalyzeDiagnostic, DiagnosticDescriptor};

/// Base: every analyzer identifies itself and declares its diagnostics.
pub trait Describe: Send + Sync + 'static {
    /// Unique analyzer identifier (e.g. "exhaustive_return").
    fn id(&self) -> &'static str;

    /// Diagnostic descriptors this analyzer can produce.
    fn descriptors(&self) -> &'static [DiagnosticDescriptor];
}

/// Analyze function/init bodies (Roslyn: RegisterOperationBlockAction).
///
/// Receives the HIR body + type inference results. Used for control flow
/// analysis, type checking, mutability, dead code, etc.
pub trait BodyCheck: Describe {
    fn check(&self, cx: &BodyContext<'_>) -> Vec<AnalyzeDiagnostic>;
}

/// Analyze declarations structurally (Roslyn: RegisterSymbolAction).
///
/// Receives an entity + its ECS components. Used for conformance checking,
/// duplicate detection, cycle detection, visibility, etc.
pub trait DeclCheck: Describe {
    /// Which declaration kinds this analyzer applies to.
    fn target_kinds(&self) -> &'static [NodeKind];

    fn check(&self, cx: &DeclContext<'_>) -> Vec<AnalyzeDiagnostic>;
}

/// Whole-compilation analysis (Roslyn: RegisterCompilationAction).
///
/// Runs once per compilation over all entities. Used for cross-entity
/// checks like cycle detection across types.
pub trait CompilationCheck: Describe {
    fn check(&self, cx: &CompilationContext<'_>) -> Vec<AnalyzeDiagnostic>;
}
