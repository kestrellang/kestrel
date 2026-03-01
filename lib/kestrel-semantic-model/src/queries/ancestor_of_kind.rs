//! AncestorOfKind query - find nearest ancestor of a specific kind

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Find the nearest ancestor of a specific kind.
///
/// Walks up the symbol tree from the given symbol, looking for an
/// ancestor that matches the specified kind.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct AncestorOfKind {
    pub symbol_id: SymbolId,
    pub kind: KestrelSymbolKind,
}

impl Query for AncestorOfKind {
    type Output = Option<SymbolId>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let mut current = model.query(SymbolFor { id: self.symbol_id });

        while let Some(symbol) = current {
            if symbol.metadata().kind() == self.kind {
                return Some(symbol.metadata().id());
            }
            current = symbol.metadata().parent();
        }

        None
    }
}
