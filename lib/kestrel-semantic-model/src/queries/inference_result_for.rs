//! Query for running type inference on a function body.

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_type_inference::{InferenceContext, Solution, generate_constraints};
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Run type inference on a function/initializer body and return the solution.
///
/// This query:
/// 1. Gets the ExecutableBehavior from the symbol
/// 2. Creates an InferenceContext with the SemanticModel as TypeOracle
/// 3. Generates constraints from the code block
/// 4. Solves the constraints and returns the Solution (with any errors)
pub struct InferenceResultFor {
    pub symbol_id: SymbolId,
}

impl Query for InferenceResultFor {
    type Output = Option<Solution>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        // Get the symbol
        let symbol = model.query(SymbolFor { id: self.symbol_id })?;

        // Get the executable behavior (body)
        let executable = symbol.metadata().get_behavior::<ExecutableBehavior>()?;

        // Get the return type from CallableBehavior if present
        let return_type = symbol
            .metadata()
            .get_behavior::<CallableBehavior>()
            .map(|c| c.return_type().clone());

        // Create inference context with model as TypeOracle
        let mut ctx = InferenceContext::new(model);

        // Generate constraints from the code block
        generate_constraints(&mut ctx, executable.body(), return_type.as_ref());

        // Solve and return the solution
        Some(ctx.solve())
    }
}
