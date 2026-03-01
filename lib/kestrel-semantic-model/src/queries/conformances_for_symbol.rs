//! ConformancesForSymbol query - get conformances attached to any symbol

use kestrel_semantic_tree::behavior::conformances::ConformancesBehavior;
use kestrel_semantic_tree::ty::Ty;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Get conformances for a symbol (struct/protocol/extension/etc.) if present.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ConformancesForSymbol {
    pub symbol_id: SymbolId,
}

impl Query for ConformancesForSymbol {
    type Output = Vec<Ty>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = match model.query(SymbolFor { id: self.symbol_id }) {
            Some(s) => s,
            None => return Vec::new(),
        };
        symbol
            .metadata()
            .get_behavior::<ConformancesBehavior>()
            .map(|cb| cb.conformances().to_vec())
            .unwrap_or_default()
    }
}
