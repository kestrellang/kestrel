//! IsVisibleFrom query - check if a target symbol is visible from a context

use semantic_tree::symbol::SymbolId;

use crate::query::Query;
use crate::visibility;
use crate::SemanticModel;

/// Check if a target symbol is visible from a context.
///
/// Applies visibility rules (public, private, internal, fileprivate)
/// to determine if `target` can be accessed from `context`.
pub struct IsVisibleFrom {
    pub target: SymbolId,
    pub context: SymbolId,
}

impl Query for IsVisibleFrom {
    type Output = bool;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let target_symbol = match model.registry().get(self.target) {
            Some(s) => s,
            None => return false,
        };
        let context_symbol = match model.registry().get(self.context) {
            Some(s) => s,
            None => return false,
        };

        visibility::is_visible_from(&target_symbol, &context_symbol)
    }
}
