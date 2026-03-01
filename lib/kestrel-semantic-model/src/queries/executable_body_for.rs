//! ExecutableBodyFor query - get a symbol's executable body (if any)

use kestrel_semantic_tree::behavior::executable::CodeBlock;
use kestrel_semantic_tree::behavior::executable::ExecutableBehavior;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get the executable body (function/initializer) for a symbol.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExecutableBodyFor {
    pub symbol_id: SymbolId,
}

impl Query for ExecutableBodyFor {
    type Output = Option<CodeBlock>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor { id: self.symbol_id })?;
        let executable = symbol.metadata().get_behavior::<ExecutableBehavior>()?;
        Some(executable.body().clone())
    }
}
