//! HasBody query - check whether a function symbol has a body

use kestrel_semantic_tree::behavior::function_data::FunctionDataBehavior;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Check whether a function has a body.
///
/// Returns `None` if the symbol is missing `FunctionDataBehavior` (not a function).
pub struct HasBody {
    pub function_id: SymbolId,
}

impl Query for HasBody {
    type Output = Option<bool>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor {
            id: self.function_id,
        })?;
        let data = symbol.metadata().get_behavior::<FunctionDataBehavior>()?;
        Some(data.has_body())
    }
}
