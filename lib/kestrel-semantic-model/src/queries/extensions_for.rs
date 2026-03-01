//! ExtensionsFor query - get all extensions for a target type

use std::sync::Arc;

use kestrel_semantic_tree::symbol::extension::ExtensionSymbol;
use semantic_tree::symbol::SymbolId;

use crate::SemanticModel;
use crate::query::Query;

/// Get all extensions registered for a target type.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct ExtensionsFor {
    pub target_id: SymbolId,
}

impl Query for ExtensionsFor {
    type Output = Vec<Arc<ExtensionSymbol>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        model
            .extension_registry()
            .get_extensions_for(self.target_id)
    }
}
