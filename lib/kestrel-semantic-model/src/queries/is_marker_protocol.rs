//! IsMarkerProtocol query - check if a protocol has no required methods or associated types

use kestrel_semantic_tree::symbol::kind::KestrelSymbolKind;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::queries::SymbolFor;
use crate::query::Query;

/// Whether a protocol is a marker protocol (no required methods or associated types).
///
/// A marker protocol only has properties or no members at all.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct IsMarkerProtocol {
    pub protocol_id: SymbolId,
}

impl Query for IsMarkerProtocol {
    type Output = bool;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        let Some(symbol) = model.query(SymbolFor {
            id: self.protocol_id,
        }) else {
            return true;
        };

        for child in symbol.metadata().children() {
            let kind = child.metadata().kind();
            if kind == KestrelSymbolKind::Function || kind == KestrelSymbolKind::AssociatedType {
                return false;
            }
        }
        true
    }
}
