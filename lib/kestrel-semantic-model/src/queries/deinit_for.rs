//! DeinitFor query - find the deinit symbol for a struct/enum

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Find the deinit declaration for a struct or enum.
///
/// Returns the SymbolId of the deinit child if one exists, or None.
/// This replaces the old DeinitBehavior that was attached cross-entity
/// from the deinit binder to the parent struct.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct DeinitFor {
    pub symbol_id: SymbolId,
}

impl Query for DeinitFor {
    type Output = Option<SymbolId>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let symbol = model.query(SymbolFor {
            id: self.symbol_id,
        })?;

        symbol
            .metadata()
            .children()
            .into_iter()
            .find(|c| c.metadata().kind() == KestrelSymbolKind::Deinit)
            .map(|c| c.metadata().id())
    }
}
