//! Query for running type inference on a function body.

use kestrel_semantic_tree::behavior::callable::CallableBehavior;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use kestrel_semantic_type_inference::{
    InferenceContext, Solution, generate_constraints, generate_default_value_constraints,
};
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;
use crate::type_oracle::ContextualOracle;

/// Run type inference on a function/initializer body and return the solution.
///
/// This query:
/// 1. Gets the ExecutableBehavior from the symbol
/// 2. Creates an InferenceContext with a ContextualOracle that knows the current function
/// 3. Generates constraints from the code block
/// 4. Solves the constraints and returns the Solution (with any errors)
#[derive(Clone, PartialEq, Eq, Hash)]
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

        // Get CallableBehavior for return type and parameter types
        let callable = symbol.metadata().get_behavior::<CallableBehavior>();
        let return_type = callable.as_ref().map(|c| c.return_type().clone());
        let param_types: Vec<_> = callable
            .as_ref()
            .map(|c| c.parameters().iter().map(|p| p.ty.clone()).collect())
            .unwrap_or_default();

        // Create a contextual oracle that knows which function we're analyzing.
        // This allows extension where clause bounds to be discovered when resolving
        // members on type parameters (e.g., T: Equatable in extension where clause).
        let oracle = ContextualOracle::new(model, self.symbol_id);
        let mut ctx = InferenceContext::new(&oracle);

        // Generate constraints from the code block
        generate_constraints(&mut ctx, executable.body(), return_type.as_ref());

        // Generate constraints for default value expressions
        let default_values = executable.default_values();
        if !default_values.is_empty() {
            generate_default_value_constraints(&mut ctx, default_values, &param_types);
        }

        // Solve and return the solution
        Some(ctx.solve())
    }
}
