//! IsInsideAny query - check if a symbol is inside any ancestor of given kinds

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Check whether a symbol has an ancestor whose kind matches any of `kinds`.
pub struct IsInsideAny {
    pub symbol_id: SymbolId,
    pub kinds: Vec<KestrelSymbolKind>,
}

impl Query for IsInsideAny {
    type Output = bool;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut current = model.query(SymbolFor { id: self.symbol_id });

        while let Some(symbol) = current {
            if self.kinds.contains(&symbol.metadata().kind()) {
                return true;
            }
            current = symbol.metadata().parent();
        }

        false
    }
}
