//! SymbolFor query - get a symbol by its ID

use std::sync::Arc;

use kestrel_semantic_tree::language::KestrelLanguage;
use semantic_tree::symbol::{Symbol, SymbolId};

use crate::SemanticModel;
use crate::query::Query;

/// Get a symbol by its ID.
pub struct SymbolFor {
    pub id: SymbolId,
}

impl Query for SymbolFor {
    type Output = Option<Arc<dyn Symbol<KestrelLanguage>>>;

    fn execute(self, model: &SemanticModel) -> Self::Output {
        model.registry().get(self.id)
    }
}
