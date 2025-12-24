//! Type inference analyzer.
//!
//! This analyzer runs type inference on function bodies and adds
//! `ResolvedExecutableBehavior` to functions with the resolved types.

use std::sync::Arc;

use kestrel_semantic_model::InferenceResultFor;
use kestrel_semantic_tree::behavior::executable::{ExecutableBehavior, ResolvedExecutableBehavior};
use kestrel_semantic_tree::language::KestrelLanguage;
use kestrel_semantic_tree::symbol::function::FunctionSymbol;
use kestrel_semantic_tree::symbol::initializer::InitializerSymbol;
use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use kestrel_semantic_tree::symbol::local::LocalContainer;
use kestrel_semantic_type_inference::{apply_solution, apply_solution_to_locals};
use semantic_tree::symbol::Symbol;

use crate::analyzer::Analyzer;
use crate::context::AnalysisContext;

mod diagnostics;

use diagnostics::InferenceErrorDiagnostic;

/// Analyzer that runs type inference on function bodies.
///
/// This analyzer:
/// 1. Gets the `ExecutableBehavior` from each function/initializer
/// 2. Runs type inference via the `InferenceResultFor` query
/// 3. Reports any inference errors as diagnostics
/// 4. Applies the solution to create a `ResolvedExecutableBehavior`
pub struct TypeInferenceAnalyzer;

impl TypeInferenceAnalyzer {
    /// Create a new type inference analyzer.
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeInferenceAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for TypeInferenceAnalyzer {
    fn name(&self) -> &'static str {
        "type_inference"
    }

    fn visit_symbol(
        &mut self,
        symbol: &Arc<dyn Symbol<KestrelLanguage>>,
        ctx: &mut AnalysisContext,
    ) {
        let kind = symbol.metadata().kind();

        // Only process functions and initializers
        if kind != KestrelSymbolKind::Function && kind != KestrelSymbolKind::Initializer {
            return;
        }

        // Only process symbols with executable bodies
        let Some(executable) = symbol.metadata().get_behavior::<ExecutableBehavior>() else {
            return;
        };

        // Run type inference via query
        let Some(solution) = ctx.model.query(InferenceResultFor {
            symbol_id: symbol.metadata().id(),
        }) else {
            return;
        };

        // Report any inference errors
        for error in solution.errors() {
            ctx.report(InferenceErrorDiagnostic::from(error.clone()));
        }

        // Apply solution to create resolved body (even if there are errors)
        let resolved_body = apply_solution(executable.body(), &solution);

        // Update local variables in the container with resolved types.
        // This is necessary because pattern-bound locals are created with Ty::infer()
        // placeholder types, and subsequent code reads the type from the LocalContainer.
        if let Some(func) = symbol.as_ref().downcast_ref::<FunctionSymbol>() {
            apply_solution_to_locals(func as &dyn LocalContainer, &solution);
        } else if let Some(init) = symbol.as_ref().downcast_ref::<InitializerSymbol>() {
            apply_solution_to_locals(init as &dyn LocalContainer, &solution);
        }

        // Add ResolvedExecutableBehavior to the symbol
        symbol
            .metadata()
            .add_behavior(ResolvedExecutableBehavior::new(resolved_body));
    }
}
